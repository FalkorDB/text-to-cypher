//! Core functionality shared between standalone and serverless deployments
//!
//! This module contains the shared logic for text-to-cypher conversion that works
//! in both streaming (standalone HTTP server) and non-streaming (serverless) contexts.

use crate::chat::{ChatRequest, ChatRole};
use crate::formatter::format_query_records;
use crate::schema::discovery::Schema;
use crate::template::TemplateEngine;
use crate::validator::CypherValidator;
use falkordb::{FalkorAsyncClient, FalkorClientBuilder, FalkorConnectionInfo};
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
    let genai_chat_request = create_cypher_query_chat_request(chat_request, schema);

    let chat_response = client
        .exec_chat(model, genai_chat_request, None)
        .await
        .map_err(|e| format!("Chat request failed: {e}"))?;

    let query = chat_response
        .content_text_into_string()
        .unwrap_or_else(|| "NO ANSWER".to_string());

    if query.trim().is_empty() || query.trim() == "NO ANSWER" {
        return Err("No valid query was generated".into());
    }

    let clean_query = query.replace('\n', " ").replace("```", "").trim().to_string();

    // Validate the query
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
    let genai_chat_request = create_answer_chat_request(chat_request, cypher_query, cypher_result);

    let chat_response = client
        .exec_chat(model, genai_chat_request, None)
        .await
        .map_err(|e| format!("Chat request failed: {e}"))?;

    let answer = chat_response
        .content_text_into_string()
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

fn create_cypher_query_chat_request(
    chat_request: &ChatRequest,
    ontology: &str,
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

    chat_req = chat_req.with_system(TemplateEngine::render_system_prompt(ontology));

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
