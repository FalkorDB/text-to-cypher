//! Core text-to-cypher pipeline logic
//!
//! This module contains the shared logic for text-to-cypher conversion that works
//! in both the standalone HTTP server and library contexts.

use crate::chat::{ChatRequest, ChatRole};
use crate::formatter::format_query_records;
use crate::schema::discovery::Schema;
use crate::skills::{self, SkillCatalog};
use crate::template::TemplateEngine;
use crate::usage::TokenUsage;
use crate::validator::CypherValidator;
use falkordb::{FalkorAsyncClient, FalkorClientBuilder, FalkorConnectionInfo};
use genai::adapter::AdapterKind;
use genai::chat::ChatMessage as GenAiChatMessage;
use genai::resolver::{AuthData, AuthResolver};
use genai::{Client as GenAiClient, ModelIden};
use std::error::Error;

/// Discovers the graph schema and returns it as a JSON string
///
/// # Errors
///
/// Returns an error if connection fails, schema discovery fails, or JSON serialization fails
pub async fn discover_graph_schema(
    falkordb_connection: &str,
    graph_name: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let connection_info: FalkorConnectionInfo = falkordb_connection
        .try_into()
        .map_err(|e| format!("Invalid connection info: {e}"))?;

    let client = FalkorClientBuilder::new_async()
        .with_connection_info(connection_info)
        .build()
        .await
        .map_err(|e| format!("Failed to build client: {e}"))?;

    let mut graph = client.select_graph(graph_name);
    let schema = Schema::discover_from_graph(&mut graph, 100)
        .await
        .map_err(|e| format!("Failed to discover schema: {e}"))?;

    let json_schema = serde_json::to_string(&schema).map_err(|e| format!("Failed to serialize schema: {e}"))?;

    Ok(json_schema)
}

/// Generates a Cypher query from natural language using AI
///
/// # Errors
///
/// Returns an error if AI chat request fails, validation fails, or no query is generated
pub async fn generate_cypher_query(
    chat_request: &ChatRequest,
    schema: &str,
    client: &GenAiClient,
    model: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    generate_cypher_query_with_skills(chat_request, schema, client, model, None).await
}

/// Generates a Cypher query with optional dynamic skill loading via tool calling.
///
/// When skills are provided and the model supports tool calling, the LLM can
/// request full skill content on-demand via the `read_skill` tool. For providers
/// without tool support, skill content is injected directly into the prompt.
///
/// # Errors
///
/// Returns an error if AI chat request fails, validation fails, or no query is generated
pub async fn generate_cypher_query_with_skills(
    chat_request: &ChatRequest,
    schema: &str,
    client: &GenAiClient,
    model: &str,
    skill_catalog: Option<&SkillCatalog>,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let mut usage = TokenUsage::new();
    generate_cypher_query_with_skills_and_usage(chat_request, schema, client, model, skill_catalog, &mut usage).await
}

/// Generates a Cypher query (with optional skills), accumulating token usage.
///
/// Behaves like [`generate_cypher_query_with_skills`] but also records the
/// [`TokenUsage`] summed across every LLM call made while producing the query
/// (tool-call rounds, fallback, and the final forced response) into `token_usage`.
///
/// Usage is accumulated as calls are made, so the counts captured in `token_usage`
/// remain valid even when this function returns an error (e.g. validation failure).
///
/// # Errors
///
/// Returns an error if AI chat request fails, validation fails, or no query is generated
pub async fn generate_cypher_query_with_skills_and_usage(
    chat_request: &ChatRequest,
    schema: &str,
    client: &GenAiClient,
    model: &str,
    skill_catalog: Option<&SkillCatalog>,
    token_usage: &mut TokenUsage,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let use_tools = skill_catalog.is_some_and(|c| !c.is_empty()) && skills::supports_tool_calling(model);

    let mut genai_chat_request =
        create_cypher_query_chat_request_with_skills(chat_request, schema, skill_catalog, use_tools);

    // Register the read_skill tool if supported
    if use_tools {
        if let Some(catalog) = skill_catalog {
            genai_chat_request = genai_chat_request.append_tool(catalog.tool_definition());
        }
    }

    for _round in 0..skills::MAX_TOOL_ROUNDS {
        let chat_response = match client.exec_chat(model, genai_chat_request.clone(), None).await {
            Ok(response) => response,
            Err(err) if use_tools => {
                tracing::warn!("Tool-enabled chat request failed; retrying without tools: {err}");
                let fallback_request =
                    create_cypher_query_chat_request_with_skills(chat_request, schema, skill_catalog, false);
                let fallback_response = client
                    .exec_chat(model, fallback_request, None)
                    .await
                    .map_err(|fallback_err| format!("Chat request failed: {err}; fallback failed: {fallback_err}"))?;
                token_usage.add_genai_usage(&fallback_response.usage);
                let query = fallback_response.into_first_text().unwrap_or_else(|| "NO ANSWER".to_string());
                return validate_generated_query(&query);
            }
            Err(err) => return Err(format!("Chat request failed: {err}").into()),
        };

        token_usage.add_genai_usage(&chat_response.usage);

        let tool_calls = chat_response.tool_calls().into_iter().cloned().collect::<Vec<_>>();

        if tool_calls.is_empty() {
            // No tool calls — extract query from text response
            let query = chat_response.into_first_text().unwrap_or_else(|| "NO ANSWER".to_string());
            return validate_generated_query(&query);
        }

        // Handle tool calls: append assistant turn once, then each tool response
        tracing::info!("LLM requested {} skill(s)", tool_calls.len());
        genai_chat_request = genai_chat_request.append_message(GenAiChatMessage::from(tool_calls.clone()));

        for tool_response in skills::resolve_skill_tool_calls(&tool_calls, skill_catalog) {
            genai_chat_request = genai_chat_request.append_message(GenAiChatMessage::from(tool_response));
        }
    }

    // If we exhausted tool rounds, force one final text response without allowing another tool call.
    genai_chat_request.tools = None;
    let final_response = client
        .exec_chat(model, genai_chat_request, None)
        .await
        .map_err(|e| format!("Chat request failed after tool rounds: {e}"))?;

    token_usage.add_genai_usage(&final_response.usage);
    let query = final_response.into_first_text().unwrap_or_else(|| "NO ANSWER".to_string());
    validate_generated_query(&query)
}

/// Validate and clean a generated query string.
fn validate_generated_query(query: &str) -> Result<String, Box<dyn Error + Send + Sync>> {
    if query.trim().is_empty() || query.trim() == "NO ANSWER" {
        return Err("No valid query was generated".into());
    }

    let clean_query = query.replace('\n', " ").replace("```", "").trim().to_string();

    let validation_result = CypherValidator::validate(&clean_query);
    if !validation_result.is_valid {
        return Err(format!("Query validation failed: {}", validation_result.errors.join("; ")).into());
    }

    Ok(clean_query)
}

/// Executes a Cypher query against the graph database
///
/// # Errors
///
/// Returns an error if connection fails, query execution fails, or task spawning fails
pub async fn execute_cypher_query(
    query: &str,
    graph_name: &str,
    falkordb_connection: &str,
    read_only: bool,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let connection_info: FalkorConnectionInfo = falkordb_connection
        .try_into()
        .map_err(|e| format!("Invalid connection info: {e}"))?;

    let client = FalkorClientBuilder::new_async()
        .with_connection_info(connection_info)
        .build()
        .await
        .map_err(|e| format!("Failed to build client: {e}"))?;

    let graph_name = graph_name.to_string();
    let query = query.to_string();

    let result = tokio::task::spawn_blocking(move || execute_query_blocking(&client, &graph_name, &query, read_only))
        .await
        .map_err(|e| format!("Failed to execute blocking task: {e}"))??;

    let formatted_result = format_query_records(&result);
    Ok(formatted_result)
}

/// Generates a final answer using AI based on the query and results
///
/// # Errors
///
/// Returns an error if the AI chat request fails
pub async fn generate_final_answer(
    chat_request: &ChatRequest,
    cypher_query: &str,
    cypher_result: &str,
    client: &GenAiClient,
    model: &str,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let mut usage = TokenUsage::new();
    generate_final_answer_with_usage(chat_request, cypher_query, cypher_result, client, model, &mut usage).await
}

/// Generates a final answer, accumulating the token usage of the call.
///
/// Behaves like [`generate_final_answer`] but also records the [`TokenUsage`]
/// consumed by the answer-generation LLM call into `token_usage`.
///
/// # Errors
///
/// Returns an error if the AI chat request fails
pub async fn generate_final_answer_with_usage(
    chat_request: &ChatRequest,
    cypher_query: &str,
    cypher_result: &str,
    client: &GenAiClient,
    model: &str,
    token_usage: &mut TokenUsage,
) -> Result<String, Box<dyn Error + Send + Sync>> {
    let genai_chat_request = create_answer_chat_request(chat_request, cypher_query, cypher_result);

    let chat_response = client
        .exec_chat(model, genai_chat_request, None)
        .await
        .map_err(|e| format!("Chat request failed: {e}"))?;

    token_usage.add_genai_usage(&chat_response.usage);

    let answer = chat_response
        .into_first_text()
        .unwrap_or_else(|| "Unable to generate answer".to_string());

    Ok(answer)
}

/// Creates a `GenAI` client with optional custom API key
#[must_use]
pub fn create_genai_client(api_key: Option<&str>) -> GenAiClient {
    api_key.map_or_else(GenAiClient::default, |key| {
        let key = key.to_string();
        let auth_resolver = AuthResolver::from_resolver_fn(
            move |model_iden: ModelIden| -> Result<Option<AuthData>, genai::resolver::Error> {
                let ModelIden {
                    adapter_kind,
                    model_name,
                } = model_iden;
                tracing::info!("Using custom auth provider for {adapter_kind} (model: {model_name})");
                Ok(Some(AuthData::from_single(key.clone())))
            },
        );
        GenAiClient::builder().with_auth_resolver(auth_resolver).build()
    })
}

// Private helper functions

#[must_use]
fn create_cypher_query_chat_request_with_skills(
    chat_request: &ChatRequest,
    ontology: &str,
    skill_catalog: Option<&SkillCatalog>,
    use_tools: bool,
) -> genai::chat::ChatRequest {
    let mut chat_req = genai::chat::ChatRequest::default();

    for (index, message) in chat_request.messages.iter().enumerate() {
        let is_last_user_message = index == chat_request.messages.len() - 1 && message.role == ChatRole::User;

        let genai_message = match message.role {
            ChatRole::User => {
                if is_last_user_message {
                    let processed_content = process_last_user_message(&message.content);
                    genai::chat::ChatMessage::user(processed_content)
                } else {
                    genai::chat::ChatMessage::user(message.content.clone())
                }
            }
            ChatRole::Assistant => genai::chat::ChatMessage::assistant(message.content.clone()),
            ChatRole::System => genai::chat::ChatMessage::system(message.content.clone()),
        };

        chat_req = chat_req.append_message(genai_message);
    }

    // Build the skills catalog text for the prompt
    let skills_text = match skill_catalog {
        Some(catalog) if !catalog.is_empty() => {
            if use_tools {
                // Tool-calling mode: compact catalog, LLM will call read_skill for details
                catalog.render_catalog()
            } else {
                // Fallback mode: inject full content for providers without tool support
                catalog.render_all_content()
            }
        }
        _ => String::new(),
    };

    let system_prompt = if skills_text.is_empty() {
        TemplateEngine::render_system_prompt(ontology)
    } else {
        TemplateEngine::render_system_prompt_with_skills(ontology, &skills_text)
    };
    chat_req = chat_req.with_system(system_prompt);

    chat_req
}

fn create_answer_chat_request(
    chat_request: &ChatRequest,
    cypher_query: &str,
    cypher_result: &str,
) -> genai::chat::ChatRequest {
    let mut chat_req = genai::chat::ChatRequest::default();

    for (index, message) in chat_request.messages.iter().enumerate() {
        let is_last_user_message = index == chat_request.messages.len() - 1 && message.role == ChatRole::User;

        let genai_message = match message.role {
            ChatRole::User => {
                if is_last_user_message {
                    let processed_content = process_last_request_prompt(&message.content, cypher_query, cypher_result);
                    genai::chat::ChatMessage::user(processed_content)
                } else {
                    genai::chat::ChatMessage::user(message.content.clone())
                }
            }
            ChatRole::Assistant => genai::chat::ChatMessage::assistant(message.content.clone()),
            ChatRole::System => genai::chat::ChatMessage::system(message.content.clone()),
        };

        chat_req = chat_req.append_message(genai_message);
    }

    chat_req
}

fn process_last_user_message(question: &str) -> String {
    TemplateEngine::render_user_prompt(question)
}

fn process_last_request_prompt(
    content: &str,
    cypher_query: &str,
    cypher_result: &str,
) -> String {
    TemplateEngine::render_last_request_prompt(content, cypher_query, cypher_result)
}

fn execute_query_blocking(
    client: &FalkorAsyncClient,
    graph_name: &str,
    query: &str,
    read_only: bool,
) -> Result<Vec<Vec<falkordb::FalkorValue>>, Box<dyn Error + Send + Sync>> {
    let rt = tokio::runtime::Runtime::new().map_err(|e| format!("Failed to create runtime: {e}"))?;

    rt.block_on(async {
        let mut graph = client.select_graph(graph_name);
        let query_result = if read_only {
            graph
                .ro_query(query)
                .execute()
                .await
                .map_err(|e| format!("Query execution failed: {e}"))?
        } else {
            graph
                .query(query)
                .execute()
                .await
                .map_err(|e| format!("Query execution failed: {e}"))?
        };

        let mut records = Vec::new();
        for record in query_result.data {
            records.push(record);
        }
        Ok(records)
    })
}

/// Lists all available model names for a specific AI provider
///
/// # Arguments
///
/// * `adapter_kind` - The AI provider (`OpenAI`, `Ollama`, `Gemini`, `Anthropic`, `Groq`, `Cohere`)
/// * `client` - The `GenAI` client
///
/// # Returns
///
/// A vector of model names supported by the adapter
///
/// # Errors
///
/// Returns an error if the model listing request fails
///
/// # Examples
///
/// ```rust,no_run
/// use text_to_cypher::core;
/// use genai::adapter::AdapterKind;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
///     let client = core::create_genai_client(None);
///     let models = core::list_adapter_models(AdapterKind::OpenAI, &client).await?;
///     println!("OpenAI models: {:?}", models);
///     Ok(())
/// }
/// ```
pub async fn list_adapter_models(
    adapter_kind: AdapterKind,
    client: &GenAiClient,
) -> Result<Vec<String>, Box<dyn Error + Send + Sync>> {
    let models = client
        .all_model_names(adapter_kind, ())
        .await
        .map_err(|e| format!("Failed to fetch models for {adapter_kind}: {e}"))?;

    Ok(models)
}

/// Lists all available models across all supported AI providers
///
/// # Arguments
///
/// * `client` - The `GenAI` client
///
/// # Returns
///
/// A hashmap mapping adapter kinds to their available model names
///
/// # Errors
///
/// This function returns `Ok` with partial results even when individual adapters fail.
/// Individual adapter failures are logged as warnings and skipped, allowing the function
/// to continue and return results from successful adapters
///
/// # Examples
///
/// ```rust,no_run
/// use text_to_cypher::core;
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
///     let client = core::create_genai_client(None);
///     let all_models = core::list_all_models(&client).await?;
///     for (kind, models) in all_models {
///         println!("{kind}: {models:?}");
///     }
///     Ok(())
/// }
/// ```
pub async fn list_all_models(
    client: &GenAiClient
) -> Result<std::collections::HashMap<AdapterKind, Vec<String>>, Box<dyn Error + Send + Sync>> {
    use std::collections::HashMap;

    const ADAPTERS: &[AdapterKind] = &[
        AdapterKind::OpenAI,
        AdapterKind::Ollama,
        AdapterKind::Gemini,
        AdapterKind::Anthropic,
        AdapterKind::Groq,
        AdapterKind::Cohere,
        // Add DeepSeek, xAI/Grok if available in your version
    ];

    let mut results = HashMap::new();

    for &adapter in ADAPTERS {
        match client.all_model_names(adapter, ()).await {
            Ok(models) => {
                results.insert(adapter, models);
            }
            Err(e) => {
                tracing::warn!("Failed to fetch models for {adapter}: {e}");
                // Continue with other adapters even if one fails
            }
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore = "Requires valid API key"]
    async fn test_list_adapter_models_openai() {
        let client = create_genai_client(None);
        let result = list_adapter_models(AdapterKind::OpenAI, &client).await;

        assert!(result.is_ok(), "Should successfully list OpenAI models");
        let models = result.unwrap();
        assert!(!models.is_empty(), "Should have at least one model");

        // OpenAI should have common models
        assert!(models.iter().any(|m| m.contains("gpt")), "Should contain GPT models");
    }

    #[tokio::test]
    #[ignore = "Requires valid API key"]
    async fn test_list_all_models() {
        let client = create_genai_client(None);
        let result = list_all_models(&client).await;

        assert!(result.is_ok(), "Should successfully list all models");
        let all_models = result.unwrap();

        // Should have at least some adapters
        assert!(!all_models.is_empty(), "Should have at least one adapter");

        // Each adapter should have models
        for (kind, models) in &all_models {
            assert!(!models.is_empty(), "{kind} should have models");
        }
    }
}
