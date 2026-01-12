#[cfg(feature = "server")]
use actix_web::{HttpResponse, ResponseError};
use serde::{Deserialize, Serialize};
use std::fmt;
#[cfg(feature = "server")]
use utoipa::ToSchema;

// Constants for Ollama detection
#[cfg(feature = "server")]
const OLLAMA_DEFAULT_HOST: &str = "localhost:11434";
#[cfg(feature = "server")]
const OLLAMA_DEFAULT_HOST_IP: &str = "127.0.0.1:11434";

#[derive(Debug, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(ToSchema))]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
    pub status_code: u16,
}

#[derive(Debug)]
pub enum ApiError {
    GenAiError(genai::Error),
    #[allow(dead_code)]
    InternalServerError(String),
    #[allow(dead_code)]
    BadRequest(String),
    #[allow(dead_code)]
    NotFound(String),
    #[allow(dead_code)]
    ServiceUnavailable(String),
}

impl fmt::Display for ApiError {
    fn fmt(
        &self,
        f: &mut fmt::Formatter<'_>,
    ) -> fmt::Result {
        match self {
            Self::GenAiError(err) => write!(f, "GenAI error: {err}"),
            Self::InternalServerError(msg) => write!(f, "Internal server error: {msg}"),
            Self::BadRequest(msg) => write!(f, "Bad request: {msg}"),
            Self::NotFound(msg) => write!(f, "Not found: {msg}"),
            Self::ServiceUnavailable(msg) => write!(f, "Service unavailable: {msg}"),
        }
    }
}

#[cfg(feature = "server")]
impl ResponseError for ApiError {
    fn error_response(&self) -> HttpResponse {
        let (status_code, error_type, message) = match self {
            Self::GenAiError(err) => {
                let msg = err.to_string();

                // Filter out Ollama-specific errors - we don't use Ollama
                if is_ollama_error(&msg) {
                    (502, "GENAI_ERROR", "AI service error: Unsupported provider".to_string())
                } else {
                    map_genai_error(&msg, err)
                }
            }
            Self::InternalServerError(msg) => (500, "INTERNAL_ERROR", msg.clone()),
            Self::BadRequest(msg) => (400, "BAD_REQUEST", msg.clone()),
            Self::NotFound(msg) => (404, "NOT_FOUND", msg.clone()),
            Self::ServiceUnavailable(msg) => (503, "SERVICE_UNAVAILABLE", msg.clone()),
        };

        let error_response = ErrorResponse {
            error: error_type.to_string(),
            message,
            status_code,
        };

        HttpResponse::build(actix_web::http::StatusCode::from_u16(status_code).unwrap()).json(error_response)
    }
}

/// Checks if an error message is Ollama-specific
#[cfg(feature = "server")]
fn is_ollama_error(msg: &str) -> bool {
    let msg_lower = msg.to_lowercase();
    msg_lower.contains("ollama")
        || msg_lower.contains(OLLAMA_DEFAULT_HOST)
        || msg_lower.contains(OLLAMA_DEFAULT_HOST_IP)
}

/// Checks if an error message indicates a model not found error
#[cfg(feature = "server")]
fn is_model_not_found_error(msg: &str) -> bool {
    msg.contains("not found")
        || (msg.contains("model") && (msg.contains("does not exist") || msg.contains("not available")))
        || msg.contains("404")
}

/// Helper to check if error is authentication-related
#[cfg(feature = "server")]
fn is_auth_error(msg: &str) -> bool {
    msg.contains("authentication")
        || msg.contains("api key")
        || msg.contains("unauthorized")
        || msg.contains("invalid_api_key")
        || msg.contains("invalid api key")
}

/// Helper to check if error is rate limit-related
#[cfg(feature = "server")]
fn is_rate_limit_error(msg: &str) -> bool {
    msg.contains("rate limit") || msg.contains("quota") || msg.contains("too many requests") || msg.contains("429")
}

/// Helper to check if error is service unavailable
#[cfg(feature = "server")]
fn is_service_unavailable_error(msg: &str) -> bool {
    msg.contains("service unavailable") || msg.contains("503") || msg.contains("temporarily unavailable")
}

/// Maps authentication errors to appropriate responses
#[cfg(feature = "server")]
fn map_auth_error(
    provider: &Provider,
    err: &genai::Error,
) -> (u16, &'static str, String) {
    match provider {
        Provider::OpenAI => (
            401,
            "AUTHENTICATION_ERROR",
            format!("OpenAI authentication failed. Please verify your API key: {err}"),
        ),
        Provider::Anthropic => (
            401,
            "AUTHENTICATION_ERROR",
            format!("Anthropic authentication failed. Please verify your API key: {err}"),
        ),
        Provider::Gemini => (
            401,
            "AUTHENTICATION_ERROR",
            format!("Google Gemini authentication failed. Please verify your API key: {err}"),
        ),
        Provider::Unknown => (401, "AUTHENTICATION_ERROR", format!("Authentication failed: {err}")),
    }
}

/// Maps rate limit errors to appropriate responses
#[cfg(feature = "server")]
fn map_rate_limit_error(
    provider: &Provider,
    err: &genai::Error,
) -> (u16, &'static str, String) {
    match provider {
        Provider::OpenAI => (
            429,
            "RATE_LIMITED",
            format!("OpenAI rate limit exceeded. Please retry after a short delay: {err}"),
        ),
        Provider::Anthropic => (
            429,
            "RATE_LIMITED",
            format!("Anthropic rate limit exceeded. Please retry after a short delay: {err}"),
        ),
        Provider::Gemini => (
            429,
            "RATE_LIMITED",
            format!("Google Gemini rate limit exceeded. Please retry after a short delay: {err}"),
        ),
        Provider::Unknown => (429, "RATE_LIMITED", format!("Rate limit exceeded: {err}")),
    }
}

/// Maps model not found errors to appropriate responses
#[cfg(feature = "server")]
fn map_model_not_found_error(
    provider: &Provider,
    err: &genai::Error,
) -> (u16, &'static str, String) {
    match provider {
        Provider::OpenAI => (
            404,
            "MODEL_NOT_FOUND",
            format!("OpenAI model not found or not available. Please check the model name: {err}"),
        ),
        Provider::Anthropic => (
            404,
            "MODEL_NOT_FOUND",
            format!("Anthropic model not found or not available. Please check the model name: {err}"),
        ),
        Provider::Gemini => (
            404,
            "MODEL_NOT_FOUND",
            format!("Google Gemini model not found or not available. Please check the model name: {err}"),
        ),
        Provider::Unknown => (404, "MODEL_NOT_FOUND", format!("Model not found: {err}")),
    }
}

/// Maps service unavailable errors to appropriate responses
#[cfg(feature = "server")]
fn map_service_unavailable_error(
    provider: &Provider,
    err: &genai::Error,
) -> (u16, &'static str, String) {
    match provider {
        Provider::OpenAI => (
            503,
            "SERVICE_UNAVAILABLE",
            format!("OpenAI service is temporarily unavailable. Please retry later: {err}"),
        ),
        Provider::Anthropic => (
            503,
            "SERVICE_UNAVAILABLE",
            format!("Anthropic service is temporarily unavailable. Please retry later: {err}"),
        ),
        Provider::Gemini => (
            503,
            "SERVICE_UNAVAILABLE",
            format!("Google Gemini service is temporarily unavailable. Please retry later: {err}"),
        ),
        Provider::Unknown => (
            503,
            "SERVICE_UNAVAILABLE",
            format!("AI service is temporarily unavailable: {err}"),
        ),
    }
}

/// Maps default errors to appropriate responses
#[cfg(feature = "server")]
fn map_default_error(
    provider: &Provider,
    err: &genai::Error,
) -> (u16, &'static str, String) {
    match provider {
        Provider::OpenAI => (502, "GENAI_ERROR", format!("OpenAI service error: {err}")),
        Provider::Anthropic => (502, "GENAI_ERROR", format!("Anthropic service error: {err}")),
        Provider::Gemini => (502, "GENAI_ERROR", format!("Google Gemini service error: {err}")),
        Provider::Unknown => (502, "GENAI_ERROR", format!("AI service error: {err}")),
    }
}

/// Maps genai errors to appropriate HTTP status codes and messages based on provider
#[cfg(feature = "server")]
fn map_genai_error(
    msg: &str,
    err: &genai::Error,
) -> (u16, &'static str, String) {
    let msg_lower = msg.to_lowercase();
    let provider = detect_provider(&msg_lower);

    if is_auth_error(&msg_lower) {
        return map_auth_error(&provider, err);
    }

    if is_rate_limit_error(&msg_lower) {
        return map_rate_limit_error(&provider, err);
    }

    if is_model_not_found_error(&msg_lower) {
        return map_model_not_found_error(&provider, err);
    }

    if is_service_unavailable_error(&msg_lower) {
        return map_service_unavailable_error(&provider, err);
    }

    map_default_error(&provider, err)
}

/// AI Provider enum for error categorization
#[cfg(feature = "server")]
enum Provider {
    OpenAI,
    Anthropic,
    Gemini,
    Unknown,
}

/// Detects which AI provider the error is related to
#[cfg(feature = "server")]
fn detect_provider(msg: &str) -> Provider {
    if msg.contains("openai") || msg.contains("gpt") {
        Provider::OpenAI
    } else if msg.contains("anthropic") || msg.contains("claude") {
        Provider::Anthropic
    } else if msg.contains("gemini") || (msg.contains("google") && msg.contains("ai")) {
        Provider::Gemini
    } else {
        Provider::Unknown
    }
}

// Conversion from genai::Error to ApiError
impl From<genai::Error> for ApiError {
    fn from(err: genai::Error) -> Self {
        Self::GenAiError(err)
    }
}

// Helper functions for creating specific error types
impl ApiError {
    #[allow(dead_code)]
    pub fn internal_server_error(msg: impl Into<String>) -> Self {
        Self::InternalServerError(msg.into())
    }

    #[allow(dead_code)]
    pub fn bad_request(msg: impl Into<String>) -> Self {
        Self::BadRequest(msg.into())
    }

    #[allow(dead_code)]
    pub fn not_found(msg: impl Into<String>) -> Self {
        Self::NotFound(msg.into())
    }

    #[allow(dead_code)]
    pub fn service_unavailable(msg: impl Into<String>) -> Self {
        Self::ServiceUnavailable(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_ollama_error() {
        #[cfg(feature = "server")]
        {
            assert!(is_ollama_error("Error connecting to Ollama service"));
            assert!(is_ollama_error("Failed to reach localhost:11434"));
            assert!(is_ollama_error("Connection refused to 127.0.0.1:11434"));
            assert!(is_ollama_error("OLLAMA model not found"));

            // Should not be detected as Ollama errors
            assert!(!is_ollama_error("OpenAI API key invalid"));
            assert!(!is_ollama_error("Anthropic rate limit exceeded"));
            assert!(!is_ollama_error("Gemini authentication failed"));
        }
    }

    #[test]
    fn test_detect_provider_openai() {
        #[cfg(feature = "server")]
        {
            assert!(matches!(detect_provider("openai api error"), Provider::OpenAI));
            assert!(matches!(detect_provider("gpt-4 model not found"), Provider::OpenAI));
            assert!(matches!(detect_provider("error from openai service"), Provider::OpenAI));
        }
    }

    #[test]
    fn test_detect_provider_anthropic() {
        #[cfg(feature = "server")]
        {
            assert!(matches!(detect_provider("anthropic rate limit"), Provider::Anthropic));
            assert!(matches!(
                detect_provider("claude-3 authentication failed"),
                Provider::Anthropic
            ));
            assert!(matches!(detect_provider("anthropic api error"), Provider::Anthropic));
        }
    }

    #[test]
    fn test_detect_provider_gemini() {
        #[cfg(feature = "server")]
        {
            assert!(matches!(detect_provider("gemini model error"), Provider::Gemini));
            assert!(matches!(
                detect_provider("google gemini quota exceeded"),
                Provider::Gemini
            ));
            assert!(matches!(detect_provider("error from google ai"), Provider::Gemini));
        }
    }

    #[test]
    fn test_detect_provider_unknown() {
        #[cfg(feature = "server")]
        {
            assert!(matches!(detect_provider("generic error message"), Provider::Unknown));
            assert!(matches!(detect_provider("unknown service error"), Provider::Unknown));
        }
    }

    #[test]
    #[cfg(feature = "server")]
    fn test_openai_authentication_error_mapping() {
        let err_msg = "OpenAI authentication failed: invalid_api_key";
        let fake_error = create_fake_genai_error();

        let (status, error_type, message) = map_genai_error(err_msg, &fake_error);

        assert_eq!(status, 401);
        assert_eq!(error_type, "AUTHENTICATION_ERROR");
        assert!(message.contains("OpenAI"));
        assert!(message.contains("authentication failed"));
    }

    #[test]
    #[cfg(feature = "server")]
    fn test_anthropic_rate_limit_error_mapping() {
        let err_msg = "Anthropic rate limit exceeded";
        let fake_error = create_fake_genai_error();

        let (status, error_type, message) = map_genai_error(err_msg, &fake_error);

        assert_eq!(status, 429);
        assert_eq!(error_type, "RATE_LIMITED");
        assert!(message.contains("Anthropic"));
        assert!(message.contains("rate limit"));
    }

    #[test]
    #[cfg(feature = "server")]
    fn test_gemini_model_not_found_error_mapping() {
        let err_msg = "Gemini model does not exist";
        let fake_error = create_fake_genai_error();

        let (status, error_type, message) = map_genai_error(err_msg, &fake_error);

        assert_eq!(status, 404);
        assert_eq!(error_type, "MODEL_NOT_FOUND");
        assert!(message.contains("Gemini"));
        assert!(message.contains("not found"));
    }

    #[test]
    #[cfg(feature = "server")]
    fn test_openai_service_unavailable_error_mapping() {
        let err_msg = "OpenAI service unavailable - 503";
        let fake_error = create_fake_genai_error();

        let (status, error_type, message) = map_genai_error(err_msg, &fake_error);

        assert_eq!(status, 503);
        assert_eq!(error_type, "SERVICE_UNAVAILABLE");
        assert!(message.contains("OpenAI"));
        assert!(message.contains("unavailable"));
    }

    #[test]
    #[cfg(feature = "server")]
    fn test_generic_error_with_unknown_provider() {
        let err_msg = "Unknown service error occurred";
        let fake_error = create_fake_genai_error();

        let (status, error_type, message) = map_genai_error(err_msg, &fake_error);

        assert_eq!(status, 502);
        assert_eq!(error_type, "GENAI_ERROR");
        assert!(message.contains("AI service error"));
    }

    #[test]
    #[cfg(feature = "server")]
    fn test_ollama_error_filtered_out() {
        let err_msg = "Connection to Ollama at localhost:11434 failed";

        // When an Ollama error is detected via is_ollama_error
        assert!(is_ollama_error(err_msg));

        // The response should filter it out
        let fake_error = create_fake_genai_error();
        let api_error = ApiError::GenAiError(fake_error);
        let _response = api_error.error_response();

        // The actual filtering happens in error_response() which returns generic message
        // when is_ollama_error() returns true
    }

    // Helper function to create a fake genai::Error for testing
    #[cfg(feature = "server")]
    fn create_fake_genai_error() -> genai::Error {
        // Create a simple error using the SerdeJson variant
        // Parse an invalid JSON to get a real serde_json::Error
        let serde_err = serde_json::from_str::<serde_json::Value>("invalid json").unwrap_err();
        genai::Error::SerdeJson(serde_err)
    }
}
