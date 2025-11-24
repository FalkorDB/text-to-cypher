//! Vercel serverless function adapter module
//! 
//! This module provides utilities for adapting the application's functionality
//! to work with Vercel's serverless function interface.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Vercel HTTP request structure
#[derive(Debug, Deserialize)]
pub struct VercelRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

/// Vercel HTTP response structure
#[derive(Debug, Serialize)]
pub struct VercelResponse {
    #[serde(rename = "statusCode")]
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl VercelResponse {
    pub fn json(status_code: u16, body: impl Serialize) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        
        Self {
            status_code,
            headers,
            body: serde_json::to_string(&body).unwrap_or_else(|_| "{}".to_string()),
        }
    }

    #[must_use]
    pub fn error(status_code: u16, message: &str) -> Self {
        let error = serde_json::json!({ "error": message });
        Self::json(status_code, error)
    }
}

/// Parse request body as JSON
pub fn parse_json_body<T: for<'de> Deserialize<'de>>(body: &str) -> Result<T, String> {
    serde_json::from_str(body).map_err(|e| format!("Failed to parse JSON: {e}"))
}

/// Get environment variable with fallback
#[must_use]
pub fn get_env_or_default(key: &str, default: &str) -> String {
    std::env::var(key).unwrap_or_else(|_| default.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vercel_response_json() {
        let response = VercelResponse::json(200, serde_json::json!({"message": "success"}));
        assert_eq!(response.status_code, 200);
        assert!(response.body.contains("success"));
    }

    #[test]
    fn test_vercel_response_error() {
        let response = VercelResponse::error(400, "Bad request");
        assert_eq!(response.status_code, 400);
        assert!(response.body.contains("Bad request"));
    }
}
