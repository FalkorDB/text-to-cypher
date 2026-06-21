//! Text-to-cypher request/response processor
//!
//! This module provides the non-streaming request/response interface for
//! text-to-cypher conversion, used by the library API and the standalone server.

use crate::chat::ChatRequest;
use crate::core::{
    create_genai_client_with_endpoint, discover_graph_schema, discover_udfs, execute_cypher_query,
    generate_cypher_query_with_context_and_usage, generate_final_answer_with_usage,
};
use crate::skills::SkillCatalog;
use crate::udf::{UdfError, UdfSource};
use crate::usage::TokenUsage;
use serde::{Deserialize, Serialize};
use std::error::Error;

/// Request structure for text-to-cypher conversion
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TextToCypherRequest {
    pub graph_name: String,
    pub chat_request: ChatRequest,
    pub model: Option<String>,
    pub key: Option<String>,
    pub falkordb_connection: Option<String>,
    /// Optional LLM provider endpoint/base URL override.
    #[serde(default, alias = "endpoint", alias = "base_url", alias = "baseUrl")]
    pub llm_endpoint: Option<String>,
    /// When true, returns only the generated Cypher query without executing it
    #[serde(default)]
    pub cypher_only: bool,
}

/// Response structure for text-to-cypher conversion
#[derive(Debug, Serialize, Deserialize)]
pub struct TextToCypherResponse {
    // Note: status is currently a String for simplicity. Future versions may use an enum.
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cypher_query: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cypher_result: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub answer: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Aggregated token usage across all LLM calls made while serving the request.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_usage: Option<TokenUsage>,
}

impl TextToCypherResponse {
    /// Checks if the response represents a successful operation
    #[must_use]
    pub fn is_success(&self) -> bool {
        self.status == "success"
    }

    /// Checks if the response represents an error
    #[must_use]
    pub fn is_error(&self) -> bool {
        self.status == "error"
    }

    #[must_use]
    pub fn success(
        schema: String,
        cypher_query: String,
        cypher_result: Option<String>,
        answer: Option<String>,
    ) -> Self {
        Self::success_with_usage(schema, cypher_query, cypher_result, answer, None)
    }

    #[must_use]
    pub fn success_with_usage(
        schema: String,
        cypher_query: String,
        cypher_result: Option<String>,
        answer: Option<String>,
        token_usage: Option<TokenUsage>,
    ) -> Self {
        Self {
            status: "success".to_string(),
            schema: Some(schema),
            cypher_query: Some(cypher_query),
            cypher_result,
            answer,
            error: None,
            token_usage,
        }
    }

    #[must_use]
    pub fn error(error_message: String) -> Self {
        Self::error_with_usage(error_message, None)
    }

    /// Creates an error response that also reports the token usage consumed before failure.
    ///
    /// Use this on error paths that occur after one or more LLM calls so that consumers
    /// can still account for the tokens the request spent.
    #[must_use]
    pub fn error_with_usage(
        error_message: String,
        token_usage: Option<TokenUsage>,
    ) -> Self {
        Self {
            status: "error".to_string(),
            schema: None,
            cypher_query: None,
            cypher_result: None,
            answer: None,
            error: Some(error_message),
            token_usage,
        }
    }
}

/// Main processor function for non-streaming text-to-cypher conversion
///
/// # Errors
///
/// This function does not return errors. All errors are captured and returned
/// as `TextToCypherResponse::error` with appropriate error messages.
///
/// # Panics
///
/// This function does not panic. All errors are handled gracefully and returned
/// as error responses within the `TextToCypherResponse` structure.
pub async fn process_text_to_cypher(
    request: TextToCypherRequest,
    default_model: Option<String>,
    default_key: Option<String>,
    default_connection: String,
) -> TextToCypherResponse {
    let builtin = SkillCatalog::builtin();
    process_text_to_cypher_with_skills(request, default_model, default_key, default_connection, Some(&builtin)).await
}

/// Process a text-to-cypher request with optional dynamic skill support.
///
/// When a `SkillCatalog` is provided, the AI model can access specialized `FalkorDB`
/// Cypher skills for better query generation. Providers that support tool calling
/// load skills on-demand; others get skill content injected into the prompt.
///
/// This is equivalent to [`process_text_to_cypher_with_context`] with UDF context disabled.
///
/// # Errors
///
/// This function does not return errors. All errors are captured and returned
/// as `TextToCypherResponse::error` with appropriate error messages.
///
/// # Panics
///
/// This function does not panic. All errors are handled gracefully and returned
/// as error responses within the `TextToCypherResponse` structure.
pub async fn process_text_to_cypher_with_skills(
    request: TextToCypherRequest,
    default_model: Option<String>,
    default_key: Option<String>,
    default_connection: String,
    skill_catalog: Option<&SkillCatalog>,
) -> TextToCypherResponse {
    process_text_to_cypher_with_context(
        request,
        default_model,
        default_key,
        default_connection,
        skill_catalog,
        &UdfSource::Off,
    )
    .await
}

/// Process a text-to-cypher request with optional skills and optional UDF context.
///
/// In addition to the behavior of [`process_text_to_cypher_with_skills`], the `udf_source`
/// controls whether the connected instance's user-defined functions are surfaced to the model:
/// [`UdfSource::Off`] adds nothing, [`UdfSource::Provided`] uses a caller-supplied catalog, and
/// [`UdfSource::Discover`] runs `GRAPH.UDF LIST` (degrading to no UDF context when unsupported).
///
/// # Errors
///
/// This function does not return errors. All errors are captured and returned
/// as `TextToCypherResponse::error` with appropriate error messages.
///
/// # Panics
///
/// This function does not panic. All errors are handled gracefully and returned
/// as error responses within the `TextToCypherResponse` structure.
#[allow(clippy::too_many_lines)]
pub async fn process_text_to_cypher_with_context(
    request: TextToCypherRequest,
    default_model: Option<String>,
    default_key: Option<String>,
    default_connection: String,
    skill_catalog: Option<&SkillCatalog>,
    udf_source: &UdfSource,
) -> TextToCypherResponse {
    // Apply defaults
    let model = request.model.clone().or(default_model);
    let key = request.key.clone().or(default_key);

    // Track if user provided custom connection
    let has_custom_connection = request.falkordb_connection.is_some();
    let falkordb_connection = request.falkordb_connection.clone().unwrap_or(default_connection);

    let Some(model) = model else {
        return TextToCypherResponse::error("Model must be provided either in request or as DEFAULT_MODEL".to_string());
    };

    // Create GenAI client
    let client = create_genai_client_with_endpoint(key.as_deref(), request.llm_endpoint.as_deref());

    // Resolve service target
    let service_target = match client.resolve_service_target(&model).await {
        Ok(target) => target,
        Err(e) => {
            return TextToCypherResponse::error(format!("Failed to resolve service target: {e}"));
        }
    };

    tracing::info!(
        "Processing text-to-cypher for graph: {} using model: {} ({:?})",
        request.graph_name,
        model,
        service_target.model.adapter_kind
    );

    // Step 1: Discover schema (skip if cypher_only and no custom connection provided)
    let schema = if request.cypher_only && !has_custom_connection {
        // Use empty schema for cypher_only mode without FalkorDB
        tracing::info!("Skipping schema discovery in cypher_only mode");
        "{}".to_string()
    } else {
        match discover_graph_schema(&falkordb_connection, &request.graph_name).await {
            Ok(s) => {
                tracing::info!("Schema discovered successfully");
                s
            }
            Err(e) => {
                return TextToCypherResponse::error(format!("Failed to discover schema: {e}"));
            }
        }
    };

    // Step 1b: Resolve UDF context (instance-global). Discovery degrades to empty on
    // servers without UDF support; an empty string adds no UDF section to the prompt.
    let udfs_text = resolve_udfs(
        udf_source,
        &falkordb_connection,
        request.cypher_only,
        has_custom_connection,
    )
    .await;

    // Track token usage across every LLM call made for this request.
    let mut token_usage = TokenUsage::new();

    // Step 2: Generate Cypher query
    let cypher_query = match generate_cypher_query_with_context_and_usage(
        &request.chat_request,
        &schema,
        &client,
        &model,
        skill_catalog,
        &udfs_text,
        &mut token_usage,
    )
    .await
    {
        Ok(q) => q,
        Err(e) => {
            return TextToCypherResponse::error_with_usage(format!("Failed to generate query: {e}"), Some(token_usage));
        }
    };

    tracing::info!("Cypher query generated: {}", cypher_query);

    // If cypher_only mode, return just the query
    if request.cypher_only {
        return TextToCypherResponse::success_with_usage(schema, cypher_query, None, None, Some(token_usage));
    }

    // Step 3: Execute query
    let cypher_result = match execute_cypher_query(&cypher_query, &request.graph_name, &falkordb_connection, true).await
    {
        Ok(r) => r,
        Err(e) => {
            // Try self-healing once
            tracing::warn!("Query execution failed, attempting self-healing: {}", e);

            match attempt_self_healing(
                &request,
                &schema,
                &cypher_query,
                &e.to_string(),
                &client,
                &model,
                &falkordb_connection,
                skill_catalog,
                &udfs_text,
                &mut token_usage,
            )
            .await
            {
                Ok((healed_query, healed_result)) => {
                    tracing::info!("Self-healing successful");
                    // Return the healed version
                    let answer = match generate_final_answer_with_usage(
                        &request.chat_request,
                        &healed_query,
                        &healed_result,
                        &client,
                        &model,
                        &mut token_usage,
                    )
                    .await
                    {
                        Ok(a) => Some(a),
                        Err(e) => {
                            tracing::error!("Failed to generate answer: {}", e);
                            None
                        }
                    };

                    return TextToCypherResponse::success_with_usage(
                        schema,
                        healed_query,
                        Some(healed_result),
                        answer,
                        Some(token_usage),
                    );
                }
                Err(heal_error) => {
                    return TextToCypherResponse::error_with_usage(
                        format!("Query execution failed: {e}. Self-healing also failed: {heal_error}"),
                        Some(token_usage),
                    );
                }
            }
        }
    };

    tracing::info!("Query executed successfully");

    // Step 4: Generate final answer
    let answer = match generate_final_answer_with_usage(
        &request.chat_request,
        &cypher_query,
        &cypher_result,
        &client,
        &model,
        &mut token_usage,
    )
    .await
    {
        Ok(a) => Some(a),
        Err(e) => {
            return TextToCypherResponse::error_with_usage(
                format!("Failed to generate answer: {e}"),
                Some(token_usage),
            );
        }
    };

    TextToCypherResponse::success_with_usage(schema, cypher_query, Some(cypher_result), answer, Some(token_usage))
}

/// Resolve the UDF context block for a request based on its [`UdfSource`].
///
/// Returns the rendered prompt block (empty string for no UDF context). [`UdfSource::Discover`]
/// runs `GRAPH.UDF LIST`; an unsupported server (older `FalkorDB`) or a `cypher_only` request
/// without a live connection yields an empty block, and transport errors are logged and treated
/// as "no UDFs" so they never fail the request.
async fn resolve_udfs(
    udf_source: &UdfSource,
    falkordb_connection: &str,
    cypher_only: bool,
    has_custom_connection: bool,
) -> String {
    match udf_source {
        UdfSource::Off => String::new(),
        UdfSource::Provided(catalog) => catalog.render(),
        UdfSource::Discover => {
            // No live database to query in cypher_only mode without a connection.
            if cypher_only && !has_custom_connection {
                return String::new();
            }
            match discover_udfs(falkordb_connection).await {
                Ok(catalog) => catalog.render(),
                Err(UdfError::Unsupported) => {
                    tracing::debug!("FalkorDB instance does not support UDFs; skipping UDF context");
                    String::new()
                }
                Err(UdfError::Transport(message)) => {
                    tracing::warn!("UDF discovery failed; continuing without UDF context");
                    tracing::debug!("UDF discovery failure detail: {message}");
                    String::new()
                }
            }
        }
    }
}

/// Attempts to self-heal a failed query by regenerating with error context
///
/// Token usage from the regeneration call is accumulated into `token_usage` even when the
/// subsequent execution fails, so the caller can report it on error responses.
#[allow(clippy::too_many_arguments)]
async fn attempt_self_healing(
    request: &TextToCypherRequest,
    schema: &str,
    failed_query: &str,
    error_message: &str,
    client: &genai::Client,
    model: &str,
    falkordb_connection: &str,
    skill_catalog: Option<&SkillCatalog>,
    udfs: &str,
    token_usage: &mut TokenUsage,
) -> Result<(String, String), Box<dyn Error + Send + Sync>> {
    use crate::chat::{ChatMessage, ChatRole};

    tracing::info!("Attempting self-healing for failed query");

    // Create a new chat request with error feedback
    let mut retry_request = request.chat_request.clone();
    retry_request.messages.push(ChatMessage {
        role: ChatRole::Assistant,
        content: failed_query.to_string(),
    });
    retry_request.messages.push(ChatMessage {
        role: ChatRole::User,
        content: format!(
            "The previous query failed with error: {error_message}. Please generate a corrected Cypher query."
        ),
    });

    // Generate new query (include skill catalog and UDF context for consistent prompt).
    // Usage is accumulated into `token_usage` even if generation/execution below fails.
    let healed_query = generate_cypher_query_with_context_and_usage(
        &retry_request,
        schema,
        client,
        model,
        skill_catalog,
        udfs,
        token_usage,
    )
    .await?;

    tracing::info!("Self-healed query generated: {}", healed_query);

    // Try executing the healed query
    let result = execute_cypher_query(&healed_query, &request.graph_name, falkordb_connection, true).await?;

    Ok((healed_query, result))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::{ChatMessage, ChatRole};
    use crate::udf::{UdfCatalog, UdfFunction, UdfLibrary};

    #[tokio::test]
    async fn resolve_udfs_off_returns_empty() {
        let text = resolve_udfs(&UdfSource::Off, "falkor://127.0.0.1:6379", false, false).await;
        assert!(text.is_empty());
    }

    #[tokio::test]
    async fn resolve_udfs_provided_renders_catalog() {
        let catalog = UdfCatalog::from_libraries(vec![UdfLibrary {
            name: "mylib".to_string(),
            functions: vec![UdfFunction::new("Foo")],
        }]);
        let text = resolve_udfs(&UdfSource::Provided(catalog), "falkor://127.0.0.1:6379", false, false).await;
        assert!(text.contains("- mylib.Foo"));
    }

    #[tokio::test]
    async fn resolve_udfs_discover_skips_when_cypher_only_without_connection() {
        // cypher_only with no custom connection => no live database => empty (no discovery attempted).
        let text = resolve_udfs(&UdfSource::Discover, "falkor://127.0.0.1:6379", true, false).await;
        assert!(text.is_empty());
    }

    #[test]
    fn test_response_is_success() {
        let response = TextToCypherResponse::success(
            "schema".to_string(),
            "MATCH (n) RETURN n".to_string(),
            Some("result".to_string()),
            Some("answer".to_string()),
        );
        assert!(response.is_success());
        assert!(!response.is_error());
    }

    #[test]
    fn test_response_is_error() {
        let response = TextToCypherResponse::error("Something went wrong".to_string());
        assert!(response.is_error());
        assert!(!response.is_success());
    }

    #[test]
    fn test_success_response_structure() {
        let response = TextToCypherResponse::success(
            "test_schema".to_string(),
            "MATCH (n) RETURN n".to_string(),
            Some("test_result".to_string()),
            Some("test_answer".to_string()),
        );

        assert_eq!(response.status, "success");
        assert_eq!(response.schema, Some("test_schema".to_string()));
        assert_eq!(response.cypher_query, Some("MATCH (n) RETURN n".to_string()));
        assert_eq!(response.cypher_result, Some("test_result".to_string()));
        assert_eq!(response.answer, Some("test_answer".to_string()));
        assert_eq!(response.error, None);
    }

    #[test]
    fn test_error_response_structure() {
        let response = TextToCypherResponse::error("Test error".to_string());

        assert_eq!(response.status, "error");
        assert_eq!(response.schema, None);
        assert_eq!(response.cypher_query, None);
        assert_eq!(response.cypher_result, None);
        assert_eq!(response.answer, None);
        assert_eq!(response.error, Some("Test error".to_string()));
        assert_eq!(response.token_usage, None);
    }

    #[test]
    fn test_error_with_usage_reports_tokens() {
        let usage = TokenUsage {
            prompt_tokens: 30,
            completion_tokens: 0,
            total_tokens: 30,
        };
        let response = TextToCypherResponse::error_with_usage("boom".to_string(), Some(usage));

        assert!(response.is_error());
        assert_eq!(response.error, Some("boom".to_string()));
        assert_eq!(response.token_usage, Some(usage));

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("token_usage"), "token_usage should be present: {json}");
        let deserialized: TextToCypherResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.token_usage, Some(usage));
    }

    #[test]
    fn test_request_serialization() {
        let request = TextToCypherRequest {
            graph_name: "test_graph".to_string(),
            chat_request: ChatRequest {
                messages: vec![ChatMessage {
                    role: ChatRole::User,
                    content: "Find all nodes".to_string(),
                }],
            },
            model: Some("gpt-4o-mini".to_string()),
            key: Some("test-key".to_string()),
            falkordb_connection: Some("falkor://localhost:6379".to_string()),
            llm_endpoint: Some("http://localhost:1234/v1".to_string()),
            cypher_only: false,
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: TextToCypherRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.graph_name, "test_graph");
        assert_eq!(deserialized.model, Some("gpt-4o-mini".to_string()));
        assert_eq!(deserialized.llm_endpoint, Some("http://localhost:1234/v1".to_string()));
        assert!(!deserialized.cypher_only);
    }

    #[test]
    fn test_request_endpoint_alias_deserialization() {
        let json = r#"{
            "graph_name": "test",
            "chat_request": {
                "messages": []
            },
            "endpoint": "http://localhost:1234/v1"
        }"#;

        let request: TextToCypherRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.llm_endpoint, Some("http://localhost:1234/v1".to_string()));
    }

    #[test]
    fn test_request_default_values() {
        let json = r#"{
            "graph_name": "test",
            "chat_request": {
                "messages": []
            }
        }"#;

        let request: TextToCypherRequest = serde_json::from_str(json).unwrap();

        assert_eq!(request.graph_name, "test");
        assert_eq!(request.model, None);
        assert_eq!(request.key, None);
        assert_eq!(request.llm_endpoint, None);
        assert!(!request.cypher_only);
    }

    #[test]
    fn test_response_serialization() {
        let response = TextToCypherResponse::success(
            "schema".to_string(),
            "MATCH (n) RETURN n".to_string(),
            None,
            Some("answer".to_string()),
        );

        let json = serde_json::to_string(&response).unwrap();
        let deserialized: TextToCypherResponse = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.status, "success");
        assert_eq!(deserialized.cypher_query, Some("MATCH (n) RETURN n".to_string()));
        assert_eq!(deserialized.cypher_result, None);
    }

    #[test]
    fn test_token_usage_omitted_when_absent() {
        let response = TextToCypherResponse::success(
            "schema".to_string(),
            "MATCH (n) RETURN n".to_string(),
            None,
            Some("answer".to_string()),
        );

        assert_eq!(response.token_usage, None);
        let json = serde_json::to_string(&response).unwrap();
        assert!(
            !json.contains("token_usage"),
            "token_usage should be omitted when None: {json}"
        );
    }

    #[test]
    fn test_token_usage_present_when_set() {
        let usage = TokenUsage {
            prompt_tokens: 12,
            completion_tokens: 8,
            total_tokens: 20,
        };
        let response = TextToCypherResponse::success_with_usage(
            "schema".to_string(),
            "MATCH (n) RETURN n".to_string(),
            Some("result".to_string()),
            Some("answer".to_string()),
            Some(usage),
        );

        let json = serde_json::to_string(&response).unwrap();
        assert!(json.contains("token_usage"), "token_usage should be present: {json}");

        let deserialized: TextToCypherResponse = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.token_usage, Some(usage));
    }

    #[test]
    fn test_request_clone() {
        let request = TextToCypherRequest {
            graph_name: "test".to_string(),
            chat_request: ChatRequest { messages: vec![] },
            model: Some("gpt-4".to_string()),
            key: None,
            falkordb_connection: None,
            llm_endpoint: None,
            cypher_only: true,
        };

        let cloned = request.clone();
        assert_eq!(cloned.graph_name, request.graph_name);
        assert_eq!(cloned.model, request.model);
        assert_eq!(cloned.cypher_only, request.cypher_only);
    }
}
