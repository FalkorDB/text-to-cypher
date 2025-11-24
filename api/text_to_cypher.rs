//! Vercel serverless function for text to Cypher conversion
//! 
//! This endpoint accepts natural language queries and converts them to Cypher queries.

use serde_json::json;
use text_to_cypher::vercel::{VercelRequest, VercelResponse};

fn main() {
    // Read request from stdin (Vercel's standard input method)
    let mut buffer = String::new();
    if std::io::Read::read_to_string(&mut std::io::stdin(), &mut buffer).is_ok() {
        if let Ok(request) = serde_json::from_str::<VercelRequest>(&buffer) {
            let response = handler(&request);
            if let Ok(json) = serde_json::to_string(&response) {
                println!("{json}");
            }
        }
    }
}

/// Handler function for Vercel serverless deployment
/// 
/// This is the entry point that Vercel will call when the function is invoked.
fn handler(request: &VercelRequest) -> VercelResponse {
    // Only accept POST requests
    if request.method != "POST" {
        return VercelResponse::error(405, "Method not allowed");
    }

    // Parse request body
    let Some(ref body) = request.body else {
        return VercelResponse::error(400, "Request body is required");
    };

    // For now, return a simple message indicating this is a serverless endpoint
    // In a full implementation, this would parse the request, call the text-to-cypher logic,
    // and return the results
    let response_body = json!({
        "message": "text_to_cypher endpoint (Vercel serverless)",
        "status": "This is a stub implementation for Vercel deployment",
        "note": "Full implementation would process the request body and return Cypher queries",
        "received_body_length": body.len()
    });

    VercelResponse::json(200, response_body)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn test_handler_post() {
        let request = VercelRequest {
            method: "POST".to_string(),
            path: "/text_to_cypher".to_string(),
            headers: HashMap::new(),
            body: Some(r#"{"graph_name": "test"}"#.to_string()),
        };

        let response = handler(&request);
        assert_eq!(response.status_code, 200);
    }

    #[test]
    fn test_handler_get() {
        let request = VercelRequest {
            method: "GET".to_string(),
            path: "/text_to_cypher".to_string(),
            headers: HashMap::new(),
            body: None,
        };

        let response = handler(&request);
        assert_eq!(response.status_code, 405);
    }
}
