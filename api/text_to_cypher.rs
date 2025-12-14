//! Vercel serverless function for text-to-cypher conversion
//!
//! This handler processes natural language queries and converts them to Cypher queries
//! for `FalkorDB` graph databases. It runs as a serverless function on Vercel.
//! Supports both JSON responses and Server-Sent Events (SSE) streaming.

use futures::StreamExt;
use serde_json::json;
use std::env;
use text_to_cypher::processor::{process_text_to_cypher, TextToCypherRequest};
use text_to_cypher::streaming::process_text_to_cypher_stream;
use tracing_subscriber::fmt;
use vercel_runtime::{run, Body, Error, Request, Response, StatusCode};

#[tokio::main]
async fn main() -> Result<(), Error> {
    // Initialize tracing
    fmt().with_max_level(tracing::Level::INFO).init();

    tracing::info!("Starting text-to-cypher serverless function");

    run(handler).await
}

/// Handles incoming HTTP requests for text-to-cypher conversion
///
/// # Errors
///
/// Returns an error if response building fails or JSON serialization fails
#[allow(clippy::too_many_lines)]
pub async fn handler(req: Request) -> Result<Response<Body>, Error> {
    tracing::info!("Received request: {} {}", req.method(), req.uri().path());

    // Handle CORS preflight
    if req.method() == "OPTIONS" {
        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Access-Control-Allow-Origin", "*")
            .header("Access-Control-Allow-Methods", "POST, OPTIONS")
            .header("Access-Control-Allow-Headers", "Content-Type")
            .body(Body::Empty)?);
    }

    // Only accept POST requests
    if req.method() != "POST" {
        return Ok(Response::builder()
            .status(StatusCode::METHOD_NOT_ALLOWED)
            .header("Content-Type", "application/json")
            .header("Access-Control-Allow-Origin", "*")
            .body(
                json!({
                    "error": "Method not allowed. Use POST.",
                    "status": "error"
                })
                .to_string()
                .into(),
            )?);
    }

    // Parse request body - vercel_runtime provides the body as bytes
    let body_bytes = req.body();

    let request: TextToCypherRequest = match serde_json::from_slice(body_bytes) {
        Ok(req) => req,
        Err(e) => {
            tracing::error!("Failed to parse request JSON: {}", e);
            return Ok(Response::builder()
                .status(StatusCode::BAD_REQUEST)
                .header("Content-Type", "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .body(
                    json!({
                        "error": format!("Invalid JSON: {}", e),
                        "status": "error"
                    })
                    .to_string()
                    .into(),
                )?);
        }
    };

    tracing::info!("Processing text-to-cypher request for graph: {}", request.graph_name);

    // Get default configuration from environment
    let default_model = env::var("DEFAULT_MODEL").ok();
    let default_key = env::var("DEFAULT_KEY").ok();
    let default_connection = env::var("FALKORDB_CONNECTION").unwrap_or_else(|_| "falkor://127.0.0.1:6379".to_string());

    // Check if streaming is requested
    if request.stream {
        tracing::info!("Starting SSE streaming mode");

        // Apply defaults for streaming
        let model = request.model.clone().or(default_model);
        let key = request.key.clone().or(default_key);
        let connection = request.falkordb_connection.clone().unwrap_or(default_connection);

        // Create the stream
        let mut stream = process_text_to_cypher_stream(
            request.graph_name,
            request.chat_request,
            model,
            key,
            connection,
            request.cypher_only,
        );

        // Collect stream events into a single string
        let mut output = String::new();
        while let Some(result) = stream.next().await {
            match result {
                Ok(event) => output.push_str(&event),
                Err(e) => {
                    use std::fmt::Write;
                    let error_payload = serde_json::json!({ "Error": e.to_string() });
                    let _ = write!(output, "data: {error_payload}\n\n");
                }
            }
        }

        return Ok(Response::builder()
            .status(StatusCode::OK)
            .header("Content-Type", "text/event-stream")
            .header("Cache-Control", "no-cache")
            .header("Connection", "keep-alive")
            .header("Access-Control-Allow-Origin", "*")
            .body(output.into())?);
    }

    // Non-streaming mode (original behavior)
    let response = process_text_to_cypher(request, default_model, default_key, default_connection).await;

    // Return response
    let status = if response.status == "success" {
        StatusCode::OK
    } else {
        StatusCode::INTERNAL_SERVER_ERROR
    };

    match serde_json::to_string(&response) {
        Ok(json_body) => Ok(Response::builder()
            .status(status)
            .header("Content-Type", "application/json")
            .header("Access-Control-Allow-Origin", "*")
            .body(json_body.into())?),
        Err(e) => {
            tracing::error!("Failed to serialize response: {}", e);
            Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header("Content-Type", "application/json")
                .header("Access-Control-Allow-Origin", "*")
                .body(
                    json!({
                        "error": format!("Failed to serialize response: {}", e),
                        "status": "error"
                    })
                    .to_string()
                    .into(),
                )?)
        }
    }
}
