use actix_web::{HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};
use std::fmt;
use utoipa::ToSchema;

#[derive(Debug, Serialize, Deserialize, ToSchema)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub status_code: u16,
}

#[derive(Debug)]
pub enum ApiError {
    GenAiError(genai::Error),
    InternalServerError(String),
    BadRequest(String),
    NotFound(String),
    ServiceUnavailable(String),
}

impl fmt::Display for ApiError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            ApiError::GenAiError(err) => write!(f, "GenAI error: {}", err),
            ApiError::InternalServerError(msg) => write!(f, "Internal server error: {}", msg),
            ApiError::BadRequest(msg) => write!(f, "Bad request: {}", msg),
            ApiError::NotFound(msg) => write!(f, "Not found: {}", msg),
            ApiError::ServiceUnavailable(msg) => write!(f, "Service unavailable: {}", msg),
        }
    }
}

impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        let (status_code, error_type, message) = match self {
            ApiError::GenAiError(err) => {
                // You can inspect the genai error and map to appropriate HTTP status
                let msg = err.to_string();
                if msg.contains("not found") || msg.contains("model") {
                    (404, "MODEL_NOT_FOUND", format!("Model not found: {}", err))
                } else if msg.contains("rate limit") || msg.contains("quota") {
                    (429, "RATE_LIMITED", format!("Rate limited: {}", err))
                } else if msg.contains("authentication") || msg.contains("api key") {
                    (
                        401,
                        "AUTHENTICATION_ERROR",
                        format!("Authentication failed: {}", err),
                    )
                } else {
                    (502, "GENAI_ERROR", format!("AI service error: {}", err))
                }
            }
            ApiError::InternalServerError(msg) => (500, "INTERNAL_ERROR", msg.clone()),
            ApiError::BadRequest(msg) => (400, "BAD_REQUEST", msg.clone()),
            ApiError::NotFound(msg) => (404, "NOT_FOUND", msg.clone()),
            ApiError::ServiceUnavailable(msg) => (503, "SERVICE_UNAVAILABLE", msg.clone()),
        };

        let error_response = ErrorResponse {
            error: error_type.to_string(),
            message,
            status_code,
        };

        HttpResponse::build(actix_web::http::StatusCode::from_u16(status_code).unwrap())
            .json(error_response)
    }
}

// Conversion from genai::Error to ApiError
impl From<genai::Error> for ApiError {
    fn from(err: genai::Error) -> Self {
        ApiError::GenAiError(err)
    }
}

// Helper functions for creating specific error types
impl ApiError {
    pub fn internal_server_error(msg: impl Into<String>) -> Self {
        ApiError::InternalServerError(msg.into())
    }

    pub fn bad_request(msg: impl Into<String>) -> Self {
        ApiError::BadRequest(msg.into())
    }

    pub fn not_found(msg: impl Into<String>) -> Self {
        ApiError::NotFound(msg.into())
    }

    pub fn service_unavailable(msg: impl Into<String>) -> Self {
        ApiError::ServiceUnavailable(msg.into())
    }
}
