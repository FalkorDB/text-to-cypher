//! Non-streaming text-to-cypher processor for serverless deployments
//!
//! This module provides a request/response interface for serverless functions
//! that don't support streaming (unlike the SSE-based streaming in main.rs).

use crate::chat::ChatRequest;
use crate::core::{
    create_genai_client, discover_graph_schema, execute_cypher_query, generate_cypher_query, generate_final_answer,
};
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
    /// When true, returns only the generated Cypher query without executing it
    #[serde(default)]
    pub cypher_only: bool,
}

/// Response structure for text-to-cypher conversion
#[derive(Debug, Serialize, Deserialize)]
pub struct TextToCypherResponse {
    pub status: String,
    pub schema: Option<String>,
    pub cypher_query: Option<String>,
    pub cypher_result: Option<String>,
    pub answer: Option<String>,
    pub error: Option<String>,
}

impl TextToCypherResponse {
    #[must_use]
    pub fn success(
        schema: String,
        cypher_query: String,
        cypher_result: Option<String>,
        answer: Option<String>,
    ) -> Self {
        Self {
            status: "success".to_string(),
            schema: Some(schema),
            cypher_query: Some(cypher_query),
            cypher_result,
            answer,
            error: None,
        }
    }

    #[must_use]
    pub fn error(error_message: String) -> Self {
        Self {
            status: "error".to_string(),
            schema: None,
            cypher_query: None,
            cypher_result: None,
            answer: None,
            error: Some(error_message),
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
    // Apply defaults
    let model = request.model.clone().or(default_model);
    let key = request.key.clone().or(default_key);

    // Track if user provided custom connection
    let has_custom_connection = request.falkordb_connection.is_some();
    let falkordb_connection = request.falkordb_connection.clone().unwrap_or(default_connection);

    // Validate required parameters
    if model.is_none() {
        return TextToCypherResponse::error("Model must be provided either in request or as DEFAULT_MODEL".to_string());
    }

    let model = model.unwrap();

    // Create GenAI client
    let client = create_genai_client(key.as_deref());

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

    // Step 2: Generate Cypher query
    let cypher_query = match generate_cypher_query(&request.chat_request, &schema, &client, &model).await {
        Ok(q) => q,
        Err(e) => {
            return TextToCypherResponse::error(format!("Failed to generate query: {e}"));
        }
    };

    tracing::info!("Cypher query generated: {}", cypher_query);

    // If cypher_only mode, return just the query
    if request.cypher_only {
        return TextToCypherResponse::success(schema, cypher_query, None, None);
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
            )
            .await
            {
                Ok((healed_query, healed_result)) => {
                    tracing::info!("Self-healing successful");
                    // Return the healed version
                    let answer = match generate_final_answer(
                        &request.chat_request,
                        &healed_query,
                        &healed_result,
                        &client,
                        &model,
                    )
                    .await
                    {
                        Ok(a) => Some(a),
                        Err(e) => {
                            tracing::error!("Failed to generate answer: {}", e);
                            None
                        }
                    };

                    return TextToCypherResponse::success(schema, healed_query, Some(healed_result), answer);
                }
                Err(heal_error) => {
                    return TextToCypherResponse::error(format!(
                        "Query execution failed: {e}. Self-healing also failed: {heal_error}"
                    ));
                }
            }
        }
    };

    tracing::info!("Query executed successfully");

    // Step 4: Generate final answer
    let answer =
        match generate_final_answer(&request.chat_request, &cypher_query, &cypher_result, &client, &model).await {
            Ok(a) => Some(a),
            Err(e) => {
                tracing::error!("Failed to generate answer: {}", e);
                None
            }
        };

    TextToCypherResponse::success(schema, cypher_query, Some(cypher_result), answer)
}

/// Attempts to self-heal a failed query by regenerating with error context
async fn attempt_self_healing(
    request: &TextToCypherRequest,
    schema: &str,
    failed_query: &str,
    error_message: &str,
    client: &genai::Client,
    model: &str,
    falkordb_connection: &str,
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

    // Generate new query
    let healed_query = generate_cypher_query(&retry_request, schema, client, model).await?;

    tracing::info!("Self-healed query generated: {}", healed_query);

    // Try executing the healed query
    let result = execute_cypher_query(&healed_query, &request.graph_name, falkordb_connection, true).await?;

    Ok((healed_query, result))
}
