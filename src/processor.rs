//! Simple text-to-cypher processor for serverless functions
//! 
//! This module provides a simplified, non-streaming interface for text-to-cypher
//! conversion that can be used in serverless environments.

use crate::chat::{ChatRequest, ChatRole};
use crate::formatter::format_query_records;
use crate::schema::discovery::Schema;
use crate::template::TemplateEngine;
use falkordb::{FalkorClientBuilder, FalkorConnectionInfo};
use genai::resolver::{AuthData, AuthResolver};
use genai::ModelIden;
use serde::{Deserialize, Serialize};

/// Request structure for text-to-cypher conversion
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct ProcessorRequest {
    pub graph_name: String,
    pub chat_request: ChatRequest,
    pub model: Option<String>,
    pub key: Option<String>,
    pub falkordb_connection: Option<String>,
}

/// Response structure for text-to-cypher conversion
#[derive(Serialize, Deserialize, Debug)]
pub struct ProcessorResponse {
    pub cypher_query: Option<String>,
    pub query_result: Option<String>,
    pub final_answer: Option<String>,
    pub error: Option<String>,
}

/// Process a text-to-cypher request and return the complete response
pub async fn process(mut request: ProcessorRequest) -> ProcessorResponse {
    // Apply defaults from environment if not provided
    if request.model.is_none() {
        request.model = std::env::var("DEFAULT_MODEL").ok();
    }

    if request.key.is_none() {
        request.key = std::env::var("DEFAULT_KEY").ok();
    }

    // Ensure we have a model
    let Some(ref model) = request.model else {
        return ProcessorResponse {
            cypher_query: None,
            query_result: None,
            final_answer: None,
            error: Some("Model must be provided either in request or as DEFAULT_MODEL in environment".to_string()),
        };
    };

    // Create genai client
    let client = match request.key.as_ref() {
        Some(key) => {
            let key = key.clone();
            let auth_resolver = AuthResolver::from_resolver_fn(
                move |_model_iden: ModelIden| -> Result<Option<AuthData>, genai::resolver::Error> {
                    Ok(Some(AuthData::from_single(key)))
                },
            );
            genai::Client::builder().with_auth_resolver(auth_resolver).build()
        }
        None => genai::Client::default(),
    };

    // Resolve service target
    if let Err(e) = client.resolve_service_target(model).await {
        return ProcessorResponse {
            cypher_query: None,
            query_result: None,
            final_answer: None,
            error: Some(format!("Failed to resolve service target: {e}")),
        };
    }

    let falkordb_connection = request
        .falkordb_connection
        .clone()
        .unwrap_or_else(|| std::env::var("FALKORDB_CONNECTION").unwrap_or_else(|_| "falkor://127.0.0.1:6379".to_string()));

    // Discover schema
    let schema = match discover_schema(&falkordb_connection, &request.graph_name).await {
        Ok(s) => s,
        Err(e) => {
            return ProcessorResponse {
                cypher_query: None,
                query_result: None,
                final_answer: None,
                error: Some(format!("Failed to discover schema: {e}")),
            };
        }
    };

    // Generate cypher query
    let cypher_query = match generate_query(&request, &schema, &client, model).await {
        Ok(q) => q,
        Err(e) => {
            return ProcessorResponse {
                cypher_query: None,
                query_result: None,
                final_answer: None,
                error: Some(format!("Failed to generate Cypher query: {e}")),
            };
        }
    };

    // Execute query
    let query_result = match execute_query(&cypher_query, &request.graph_name, &falkordb_connection).await {
        Ok(r) => r,
        Err(e) => {
            return ProcessorResponse {
                cypher_query: Some(cypher_query),
                query_result: None,
                final_answer: None,
                error: Some(format!("Failed to execute query: {e}")),
            };
        }
    };

    // Generate final answer
    let final_answer = match generate_answer(&request, &cypher_query, &query_result, &client, model).await {
        Ok(a) => a,
        Err(e) => {
            return ProcessorResponse {
                cypher_query: Some(cypher_query),
                query_result: Some(query_result),
                final_answer: None,
                error: Some(format!("Failed to generate final answer: {e}")),
            };
        }
    };

    ProcessorResponse {
        cypher_query: Some(cypher_query),
        query_result: Some(query_result),
        final_answer: Some(final_answer),
        error: None,
    }
}

/// Helper function to get the last message if it's from a user
fn get_last_user_message(request: &ProcessorRequest) -> Option<&crate::chat::ChatMessage> {
    request
        .chat_request
        .messages
        .last()
        .filter(|msg| matches!(msg.role, ChatRole::User))
}

async fn discover_schema(falkordb_connection: &str, graph_name: &str) -> Result<String, String> {
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

    serde_json::to_string(&schema).map_err(|e| format!("Failed to serialize schema: {e}"))
}

async fn generate_query(
    request: &ProcessorRequest,
    schema: &str,
    client: &genai::Client,
    model: &str,
) -> Result<String, String> {
    // Build the chat request
    let mut chat_req = genai::chat::ChatRequest::default();

    // Add conversation history
    for message in &request.chat_request.messages {
        let genai_message = match message.role {
            ChatRole::User => genai::chat::ChatMessage::user(message.content.clone()),
            ChatRole::Assistant => genai::chat::ChatMessage::assistant(message.content.clone()),
            ChatRole::System => genai::chat::ChatMessage::system(message.content.clone()),
        };
        chat_req = chat_req.append_message(genai_message);
    }

    // Add system prompt with schema
    chat_req = chat_req.with_system(
        TemplateEngine::render_system_prompt(schema)
            .unwrap_or_else(|e| format!("Generate OpenCypher statements using this ontology: {schema}\n\nError loading template: {e}")),
    );

    // Process last user message if exists
    if let Some(last_msg) = get_last_user_message(request) {
        let user_prompt = TemplateEngine::render_user_prompt(&last_msg.content)
            .unwrap_or_else(|_| format!("Generate an OpenCypher statement for: {}", last_msg.content));
        chat_req = chat_req.append_message(genai::chat::ChatMessage::user(user_prompt));
    }

    let response = client
        .exec_chat(model, chat_req, None)
        .await
        .map_err(|e| format!("Chat request failed: {e}"))?;

    let query = response
        .content_text_into_string()
        .ok_or_else(|| "No response from AI model".to_string())?;

    // Clean up the query
    let clean_query = query.replace('\n', " ").replace("```", "").trim().to_string();
    Ok(clean_query)
}

async fn execute_query(
    query: &str,
    graph_name: &str,
    falkordb_connection: &str,
) -> Result<String, String> {
    let connection_info: FalkorConnectionInfo = falkordb_connection
        .try_into()
        .map_err(|e| format!("Invalid connection info: {e}"))?;

    let client = FalkorClientBuilder::new_async()
        .with_connection_info(connection_info)
        .build()
        .await
        .map_err(|e| format!("Failed to build client: {e}"))?;

    let mut graph = client.select_graph(graph_name);
    let query_result = graph
        .query(query)
        .with_timeout(30_000)
        .execute()
        .await
        .map_err(|e| format!("Query execution failed: {e}"))?;

    // Convert LazyResultSet to Vec<Vec<FalkorValue>>
    let mut records = Vec::new();
    for record in query_result.data {
        records.push(record);
    }

    let formatted = format_query_records(&records);
    Ok(formatted)
}

async fn generate_answer(
    request: &ProcessorRequest,
    cypher_query: &str,
    query_result: &str,
    client: &genai::Client,
    model: &str,
) -> Result<String, String> {
    // Build messages for final answer
    let mut chat_req = genai::chat::ChatRequest::default();

    // Add conversation history
    for message in &request.chat_request.messages {
        let genai_message = match message.role {
            ChatRole::User => genai::chat::ChatMessage::user(message.content.clone()),
            ChatRole::Assistant => genai::chat::ChatMessage::assistant(message.content.clone()),
            ChatRole::System => genai::chat::ChatMessage::system(message.content.clone()),
        };
        chat_req = chat_req.append_message(genai_message);
    }

    // Add query and results
    if let Some(last_msg) = get_last_user_message(request) {
        let final_prompt = TemplateEngine::render_last_request_prompt(&last_msg.content, cypher_query, query_result)
            .unwrap_or_else(|_| {
                format!(
                    "Based on the question: {}\n\nCypher query: {}\n\nResults: {}\n\nProvide a natural language answer.",
                    last_msg.content, cypher_query, query_result
                )
            });
        chat_req = chat_req.append_message(genai::chat::ChatMessage::user(final_prompt));
    }

    let response = client
        .exec_chat(model, chat_req, None)
        .await
        .map_err(|e| format!("Chat request failed: {e}"))?;

    response
        .content_text_into_string()
        .ok_or_else(|| "No response from AI model".to_string())
}
