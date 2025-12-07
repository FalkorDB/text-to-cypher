//! Vercel serverless function utilities
//!
//! This module provides HTTP adapter utilities for Vercel serverless functions.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Vercel request structure (simplified)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VercelRequest {
    pub method: String,
    pub path: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

/// Vercel response structure
#[derive(Debug, Serialize, Deserialize)]
pub struct VercelResponse {
    pub status_code: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

impl VercelResponse {
    pub fn json(
        status_code: u16,
        body: impl Serialize,
    ) -> Result<Self, serde_json::Error> {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Access-Control-Allow-Origin".to_string(), "*".to_string());

        Ok(Self {
            status_code,
            headers,
            body: serde_json::to_string(&body)?,
        })
    }

    pub fn error(
        status_code: u16,
        message: &str,
    ) -> Self {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());
        headers.insert("Access-Control-Allow-Origin".to_string(), "*".to_string());

        let error_body = serde_json::json!({
            "error": message,
            "status": "error"
        });

        Self {
            status_code,
            headers,
            body: error_body.to_string(),
        }
    }

    pub fn ok(body: impl Serialize) -> Result<Self, serde_json::Error> {
        Self::json(200, body)
    }

    pub fn bad_request(message: &str) -> Self {
        Self::error(400, message)
    }

    pub fn internal_error(message: &str) -> Self {
        Self::error(500, message)
    }
}
