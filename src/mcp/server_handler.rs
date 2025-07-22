use crate::chat::{ChatMessage, ChatRequest, ChatRole};
use crate::mcp::tools::TextToCypherTool;
use async_trait::async_trait;
use futures_util::StreamExt;
use rust_mcp_sdk::schema::TextContent;
use rust_mcp_sdk::schema::{
    CallToolRequest, CallToolResult, ListToolsRequest, ListToolsResult, RpcError, schema_utils::CallToolError,
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
    // Create a simple chat request with the question
    let chat_request = ChatRequest {
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: tool_args.question,
        }],
    };

    // Create the HTTP request payload (matching TextToCypherRequest structure)
    let http_request = serde_json::json!({
        "graph_name": tool_args.graph_name,
        "chat_request": chat_request,
        "model": null,  // Will use defaults from .env
        "key": null     // Will use defaults from .env
    });

    tracing::info!(
        "Forwarding request to HTTP endpoint: {}",
        serde_json::to_string_pretty(&http_request)?
    );

    // Make HTTP request to the local endpoint
    let client = reqwest::Client::new();
    let response = client
        .post("http://127.0.0.1:8080/text_to_cypher")
        .header("Content-Type", "application/json")
        .json(&http_request)
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(format!("HTTP request failed with status: {}", response.status()).into());
    }

    // Handle the SSE stream
    let mut stream = response.bytes_stream();
    let mut result_buffer = String::new();
    let mut final_result = String::new();

    while let Some(chunk_result) = stream.next().await {
        let chunk = chunk_result?;
        let chunk_str = String::from_utf8_lossy(&chunk);

        // Parse SSE events
        for line in chunk_str.lines() {
            if let Some(data) = line.strip_prefix("data: ") {
                // Remove "data: " prefix

                // Parse the JSON data
                if let Ok(progress) = serde_json::from_str::<serde_json::Value>(data) {
                    if let Some(event_type) = progress.as_object().and_then(|obj| obj.keys().next()) {
                        match event_type.as_str() {
                            "Status" => {
                                if let Some(status) = progress.get("Status").and_then(|v| v.as_str()) {
                                    tracing::info!("Status: {}", status);
                                    writeln!(result_buffer, "Status: {status}").unwrap();
                                }
                            }
                            "Schema" => {
                                if let Some(_schema) = progress.get("Schema").and_then(|v| v.as_str()) {
                                    tracing::info!("Schema discovered");
                                    result_buffer.push_str("Schema: Discovered\n");
                                }
                            }
                            "CypherQuery" => {
                                if let Some(query) = progress.get("CypherQuery").and_then(|v| v.as_str()) {
                                    tracing::info!("Generated Cypher: {}", query);
                                    writeln!(result_buffer, "Cypher Query: {query}").unwrap();
                                }
                            }
                            "CypherResult" => {
                                if let Some(cypher_result) = progress.get("CypherResult").and_then(|v| v.as_str()) {
                                    tracing::info!("Cypher result: {}", cypher_result);
                                    writeln!(result_buffer, "Query Result: {cypher_result}").unwrap();
                                }
                            }
                            "ModelOutputChunk" => {
                                if let Some(chunk) = progress.get("ModelOutputChunk").and_then(|v| v.as_str()) {
                                    final_result.push_str(chunk);
                                }
                            }
                            "Result" => {
                                if let Some(result) = progress.get("Result").and_then(|v| v.as_str()) {
                                    tracing::info!("Final result received");
                                    final_result = result.to_string();
                                }
                            }
                            "Error" => {
                                if let Some(error) = progress.get("Error").and_then(|v| v.as_str()) {
                                    tracing::error!("Error from HTTP endpoint: {}", error);
                                    return Err(format!("Error from text-to-cypher service: {error}").into());
                                }
                            }
                            _ => {
                                tracing::debug!("Unknown event type: {}", event_type);
                            }
                        }
                    }
                }
            }
        }
    }

    // Combine the process information with the final result
    let complete_response = if final_result.is_empty() {
        result_buffer.trim().to_string()
    } else {
        format!("{}\n\nFinal Answer:\n{}", result_buffer.trim(), final_result)
    };

    Ok(complete_response)
}
