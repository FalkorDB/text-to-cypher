//! Vercel serverless function for text to Cypher conversion
//!
//! This endpoint accepts natural language queries and converts them to Cypher queries.

use text_to_cypher::processor::{ProcessorRequest, process};
use text_to_cypher::vercel::{VercelRequest, VercelResponse};

#[tokio::main]
async fn main() {
    // Read request from stdin (Vercel's standard input method)
    let mut buffer = String::new();
    match std::io::Read::read_to_string(&mut std::io::stdin(), &mut buffer) {
        Ok(_) => match serde_json::from_str::<VercelRequest>(&buffer) {
            Ok(request) => {
                let response = handler(&request).await;
                match serde_json::to_string(&response) {
                    Ok(json) => println!("{json}"),
                    Err(e) => eprintln!("Failed to serialize response: {e}"),
                }
            }
            Err(e) => eprintln!("Failed to parse request: {e}"),
        },
        Err(e) => eprintln!("Failed to read from stdin: {e}"),
    }
}

/// Handler function for Vercel serverless deployment
///
/// This is the entry point that Vercel will call when the function is invoked.
async fn handler(request: &VercelRequest) -> VercelResponse {
    // Only accept POST requests
    if request.method != "POST" {
        return VercelResponse::error(405, "Method not allowed");
    }

    // Parse request body
    let Some(ref body) = request.body else {
        return VercelResponse::error(400, "Request body is required");
    };

    // Parse the ProcessorRequest from the body
    let processor_request: ProcessorRequest = match serde_json::from_str(body) {
        Ok(req) => req,
        Err(e) => {
            return VercelResponse::error(400, &format!("Invalid request body: {e}"));
        }
    };

    // Process the request using the core logic
    let result = process(processor_request).await;

    // Return the response
    VercelResponse::json(200, result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_handler_get() {
        let request = VercelRequest {
            method: "GET".to_string(),
            path: "/text_to_cypher".to_string(),
            headers: HashMap::new(),
            body: None,
        };

        let response = handler(&request).await;
        assert_eq!(response.status_code, 405);
    }

    #[tokio::test]
    async fn test_handler_missing_body() {
        let request = VercelRequest {
            method: "POST".to_string(),
            path: "/text_to_cypher".to_string(),
            headers: HashMap::new(),
            body: None,
        };

        let response = handler(&request).await;
        assert_eq!(response.status_code, 400);
    }

    #[tokio::test]
    async fn test_handler_invalid_json() {
        let request = VercelRequest {
            method: "POST".to_string(),
            path: "/text_to_cypher".to_string(),
            headers: HashMap::new(),
            body: Some("invalid json".to_string()),
        };

        let response = handler(&request).await;
        assert_eq!(response.status_code, 400);
    }
}
