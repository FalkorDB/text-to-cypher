use crate::chat::{ChatMessage, ChatRequest, ChatRole};
use crate::mcp::tools::TextToCypherTool;
use async_trait::async_trait;
use futures_util::StreamExt;
use rust_mcp_sdk::schema::TextContent;
use rust_mcp_sdk::schema::{
    CallToolRequest, CallToolResult, ListResourcesRequest, ListResourcesResult, ListToolsRequest, ListToolsResult,
    ReadResourceRequest, ReadResourceResult, Resource, RpcError, TextResourceContents, schema_utils::CallToolError,
};
use rust_mcp_sdk::{McpServer, mcp_server::ServerHandler};
use std::fmt::Write;

// Custom Handler to handle MCP Messages
pub struct MyServerHandler;

#[async_trait]
impl ServerHandler for MyServerHandler {
    async fn handle_list_tools_request(
        &self,
        _request: ListToolsRequest,
        _runtime: &dyn McpServer,
    ) -> std::result::Result<ListToolsResult, RpcError> {
        tracing::info!("Handling List Tools Request");
        Ok(ListToolsResult {
            meta: None,
            next_cursor: None,
            tools: vec![TextToCypherTool::tool()],
        })
    }

    async fn handle_list_resources_request(
        &self,
        _request: ListResourcesRequest,
        _runtime: &dyn McpServer,
    ) -> std::result::Result<ListResourcesResult, RpcError> {
        tracing::info!("Handling List Resources Request");

        match get_falkordb_graphs().await {
            Ok(graphs) => {
                let resources: Vec<Resource> = graphs
                    .into_iter()
                    .map(|graph_name| Resource {
                        uri: format!("falkordb://graph/{graph_name}"),
                        name: format!("Graph: {graph_name}"),
                        description: Some(format!("FalkorDB graph database: {graph_name}")),
                        mime_type: Some("application/json".to_string()),
                        annotations: None,
                        meta: None,
                        size: None,
                        title: None,
                    })
                    .collect();

                Ok(ListResourcesResult {
                    meta: None,
                    next_cursor: None,
                    resources,
                })
            }
            Err(e) => {
                tracing::error!("Failed to list FalkorDB graphs: {}", e);
                Err(RpcError::internal_error())
            }
        }
    }

    async fn handle_read_resource_request(
        &self,
        request: ReadResourceRequest,
        _runtime: &dyn McpServer,
    ) -> std::result::Result<ReadResourceResult, RpcError> {
        tracing::info!("Handling Read Resource Request for URI: {}", request.params.uri);

        // Parse the URI to extract graph name
        if let Some(graph_name) = request.params.uri.strip_prefix("falkordb://graph/") {
            match get_graph_schema_via_api(graph_name).await {
                Ok(schema_info) => {
                    let text_content = TextResourceContents {
                        uri: request.params.uri,
                        mime_type: Some("application/json".to_string()),
                        text: schema_info,
                        meta: None,
                    };
                    Ok(ReadResourceResult {
                        meta: None,
                        contents: vec![text_content.into()],
                    })
                }
                Err(e) => {
                    tracing::error!("Failed to read graph schema for {}: {}", graph_name, e);
                    Err(RpcError::invalid_params())
                }
            }
        } else {
            Err(RpcError::invalid_params())
        }
    }

    async fn handle_call_tool_request(
        &self,
        request: CallToolRequest,
        _runtime: &dyn McpServer,
    ) -> std::result::Result<CallToolResult, CallToolError> {
        tracing::info!("Handling Call Tool Request");
        if request.tool_name() == TextToCypherTool::tool_name() {
            // Get the arguments from the request
            let arguments = request.params.arguments.unwrap_or_default();
            let arguments_value = serde_json::Value::Object(arguments);

            // Parse the tool arguments
            match serde_json::from_value::<TextToCypherTool>(arguments_value.clone()) {
                Ok(tool_args) => {
                    tracing::info!("TextToCypherTool called with arguments:");
                    tracing::info!("  graph_name: {}", tool_args.graph_name);
                    tracing::info!("  question: {}", tool_args.question);

                    // Forward the request to the HTTP endpoint
                    match forward_to_http_endpoint(tool_args).await {
                        Ok(result) => Ok(CallToolResult::text_content(vec![TextContent::from(result)])),
                        Err(e) => {
                            tracing::error!("Failed to forward request to HTTP endpoint: {}", e);
                            Err(CallToolError::new(std::io::Error::other(format!(
                                "HTTP forwarding failed: {e}"
                            ))))
                        }
                    }
                }
                Err(e) => {
                    tracing::error!("Failed to parse TextToCypherTool arguments: {}", e);
                    Err(CallToolError::new(e))
                }
            }
        } else {
            Err(CallToolError::unknown_tool(request.tool_name().to_string()))
        }
    }

    async fn on_server_started(
        &self,
        _runtime: &dyn McpServer,
    ) {
    }
}

// Helper function to forward MCP tool request to HTTP endpoint
async fn forward_to_http_endpoint(
    tool_args: TextToCypherTool
) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let http_request = create_http_request_payload(tool_args);
    let response = send_http_request(&http_request).await?;
    process_sse_response(response).await
}

// Create HTTP request payload for the text-to-cypher endpoint
fn create_http_request_payload(tool_args: TextToCypherTool) -> serde_json::Value {
    let chat_request = ChatRequest {
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: tool_args.question,
        }],
    };

    serde_json::json!({
        "graph_name": tool_args.graph_name,
        "chat_request": chat_request,
        "model": null,
        "key": null
    })
}

// Send HTTP request to the text-to-cypher endpoint
async fn send_http_request(
    http_request: &serde_json::Value
) -> Result<reqwest::Response, Box<dyn std::error::Error + Send + Sync>> {
    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:8080/text_to_cypher")
        .header("Content-Type", "application/json")
        .json(http_request)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("HTTP request failed with status: {}", response.status()).into());
    }

    Ok(response)
}

// Process SSE response stream from the HTTP endpoint
async fn process_sse_response(response: reqwest::Response) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    let mut stream = response.bytes_stream();
    let mut result_buffer = String::new();
    let mut final_result = String::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        let chunk_str = String::from_utf8_lossy(&chunk);

        for line in chunk_str.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                process_sse_event(data, &mut result_buffer, &mut final_result)?;
            }
        }
    }

    Ok(build_complete_response(&result_buffer, &final_result))
}

// Process individual SSE event
#[allow(clippy::collapsible_if)]
fn process_sse_event(
    data: &str,
    result_buffer: &mut String,
    final_result: &mut String,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Ok(progress) = serde_json::from_str::<serde_json::Value>(data) {
        if let Some(event_type) = progress.as_object().and_then(|obj| obj.keys().next()) {
            match event_type.as_str() {
                "Status" => handle_status_event(&progress, result_buffer),
                "Schema" => handle_schema_event(result_buffer),
                "CypherQuery" => handle_cypher_query_event(&progress, result_buffer),
                "CypherResult" => handle_cypher_result_event(&progress, result_buffer),
                "ModelOutputChunk" => handle_model_output_chunk(&progress, final_result),
                "Result" => handle_result_event(&progress, final_result),
                "Error" => return handle_error_event(&progress),
                _ => tracing::debug!("Unknown event type: {}", event_type),
            }
        }
    }
    Ok(())
}

// Handle different types of SSE events
fn handle_status_event(
    progress: &serde_json::Value,
    result_buffer: &mut String,
) {
    if let Some(status) = progress.get("Status").and_then(|v| v.as_str()) {
        tracing::info!("Status: {}", status);
        writeln!(result_buffer, "Status: {status}").unwrap();
    }
}

fn handle_schema_event(result_buffer: &mut String) {
    tracing::info!("Schema discovered");
    result_buffer.push_str("Schema: Discovered\n");
}

fn handle_cypher_query_event(
    progress: &serde_json::Value,
    result_buffer: &mut String,
) {
    if let Some(query) = progress.get("CypherQuery").and_then(|v| v.as_str()) {
        tracing::info!("Generated Cypher: {}", query);
        writeln!(result_buffer, "Cypher Query: {query}").unwrap();
    }
}

fn handle_cypher_result_event(
    progress: &serde_json::Value,
    result_buffer: &mut String,
) {
    if let Some(cypher_result) = progress.get("CypherResult").and_then(|v| v.as_str()) {
        tracing::info!("Cypher result: {}", cypher_result);
        writeln!(result_buffer, "Query Result: {cypher_result}").unwrap();
    }
}

fn handle_model_output_chunk(
    progress: &serde_json::Value,
    final_result: &mut String,
) {
    if let Some(chunk) = progress.get("ModelOutputChunk").and_then(|v| v.as_str()) {
        final_result.push_str(chunk);
    }
}

fn handle_result_event(
    progress: &serde_json::Value,
    final_result: &mut String,
) {
    if let Some(result) = progress.get("Result").and_then(|v| v.as_str()) {
        tracing::info!("Final result received");
        *final_result = result.to_string();
    }
}

fn handle_error_event(progress: &serde_json::Value) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if let Some(error) = progress.get("Error").and_then(|v| v.as_str()) {
        tracing::error!("Error from HTTP endpoint: {}", error);
        return Err(format!("Error from text-to-cypher service: {error}").into());
    }
    Ok(())
}

// Build the complete response from buffer and final result
fn build_complete_response(
    result_buffer: &str,
    final_result: &str,
) -> String {
    if final_result.is_empty() {
        result_buffer.trim().to_string()
    } else {
        format!("{}\n\nFinal Answer:\n{}", result_buffer.trim(), final_result)
    }
}

// Helper function to get list of graphs from FalkorDB via REST API
async fn get_falkordb_graphs() -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
    // Call the local REST API endpoint
    let client = reqwest::Client::new();
    let response = client
        .get("http://localhost:8080/list_graphs")
        .send()
        .await
        .map_err(|e| format!("Failed to call list_graphs API: {e}"))?;

    if response.status().is_success() {
        let graphs: Vec<String> = response.json().await.map_err(|e| format!("Failed to parse response: {e}"))?;
        Ok(graphs)
    } else {
        Err(format!("API returned error status: {}", response.status()).into())
    }
}

// Helper function to get schema information for a specific graph via REST API
async fn get_graph_schema_via_api(graph_name: &str) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
    // Call the local REST API endpoint
    let client = reqwest::Client::new();
    let response = client
        .get(format!("http://localhost:8080/get_schema/{graph_name}"))
        .send()
        .await
        .map_err(|e| format!("Failed to call get_schema API: {e}"))?;

    if response.status().is_success() {
        let schema: String = response.json().await.map_err(|e| format!("Failed to parse response: {e}"))?;
        Ok(schema)
    } else {
        Err(format!("API returned error status: {}", response.status()).into())
    }
}
