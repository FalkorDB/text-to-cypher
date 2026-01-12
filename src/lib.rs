//! # text-to-cypher
//!
//! A library for translating natural language text to Cypher queries using AI models.
//!
//! This library provides both a programmatic API and a REST server for converting
//! natural language questions into Cypher queries for graph databases, with built-in
//! support for `FalkorDB`.
//!
//! ## Features
//!
//! - **Text to Cypher Translation**: Convert natural language queries to Cypher database queries using AI
//! - **Schema Discovery**: Automatically discover and analyze graph database schemas
//! - **Query Validation**: Built-in validation system to catch syntax errors before execution
//! - **Self-Healing Queries**: Automatic retry with error feedback when queries fail
//! - **Flexible AI Integration**: Support for multiple AI providers through the genai crate
//!
//! ## Library Usage
//!
//! To use text-to-cypher as a library in your Rust application:
//!
//! ```toml
//! [dependencies]
//! text-to-cypher = { version = "0.1", default-features = false }
//! ```
//!
//! ### Basic Example
//!
//! ```rust,no_run
//! use text_to_cypher::{TextToCypherClient, ChatRequest, ChatMessage, ChatRole};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     // Create a client
//!     let client = TextToCypherClient::new(
//!         "gpt-4o-mini",
//!         "your-api-key",
//!         "falkor://127.0.0.1:6379"
//!     );
//!
//!     // Create a chat request
//!     let request = ChatRequest {
//!         messages: vec![
//!             ChatMessage {
//!                 role: ChatRole::User,
//!                 content: "Find all actors who appeared in movies released after 2020".to_string(),
//!             }
//!         ]
//!     };
//!
//!     // Convert text to Cypher and execute
//!     let response = client.text_to_cypher("movies", request).await?;
//!     
//!     println!("Generated query: {}", response.cypher_query.unwrap());
//!     println!("Result: {}", response.cypher_result.unwrap());
//!     println!("Answer: {}", response.answer.unwrap());
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Generate Cypher Only (Without Execution)
//!
//! ```rust,no_run
//! use text_to_cypher::{TextToCypherClient, ChatRequest, ChatMessage, ChatRole};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     let client = TextToCypherClient::new(
//!         "gpt-4o-mini",
//!         "your-api-key",
//!         "falkor://127.0.0.1:6379"
//!     );
//!
//!     let request = ChatRequest {
//!         messages: vec![
//!             ChatMessage {
//!                 role: ChatRole::User,
//!                 content: "Find all people with more than 5 friends".to_string(),
//!             }
//!         ]
//!     };
//!
//!     // Generate query only, don't execute
//!     let response = client.cypher_only("social", request).await?;
//!     println!("Query: {}", response.cypher_query.unwrap());
//!
//!     Ok(())
//! }
//! ```
//!
//! ### Using Core Functions Directly
//!
//! For more control, you can use the core functions directly:
//!
//! ```rust,no_run
//! use text_to_cypher::{core, ChatRequest, ChatMessage, ChatRole};
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
//!     // Discover schema
//!     let schema = core::discover_graph_schema(
//!         "falkor://127.0.0.1:6379",
//!         "movies"
//!     ).await?;
//!     
//!     // Create GenAI client
//!     let client = core::create_genai_client(Some("your-api-key"));
//!     
//!     // Generate query
//!     let chat_req = ChatRequest {
//!         messages: vec![
//!             ChatMessage {
//!                 role: ChatRole::User,
//!                 content: "Find all actors".to_string(),
//!             }
//!         ]
//!     };
//!     
//!     let query = core::generate_cypher_query(
//!         &chat_req,
//!         &schema,
//!         &client,
//!         "gpt-4o-mini"
//!     ).await?;
//!     
//!     // Execute query
//!     let result = core::execute_cypher_query(
//!         &query,
//!         "movies",
//!         "falkor://127.0.0.1:6379",
//!         true
//!     ).await?;
//!     
//!     println!("Result: {}", result);
//!     Ok(())
//! }
//! ```
//!
//! ## Server Mode
//!
//! To use the REST server, enable the `server` feature (enabled by default):
//!
//! ```toml
//! [dependencies]
//! text-to-cypher = "0.1"
//! ```
//!
//! Then run the binary:
//!
//! ```bash
//! cargo run
//! ```

// Core modules - always available
pub mod chat;
pub mod core;
pub mod error;
pub mod formatter;
pub mod processor;
pub mod schema;
pub mod template;
pub mod validator;

// Re-export commonly used types for easier access
pub use chat::{ChatMessage, ChatRequest, ChatRole};
pub use error::ErrorResponse;
pub use genai::adapter::AdapterKind;
pub use processor::{TextToCypherRequest, TextToCypherResponse};
// Server-specific modules - only when server feature is enabled
#[cfg(feature = "server")]
pub mod mcp;
#[cfg(feature = "server")]
pub mod streaming;
#[cfg(feature = "server")]
pub mod vercel;

/// A high-level client for text-to-cypher operations.
///
/// This client provides a convenient interface for converting natural language
/// to Cypher queries and executing them against a `FalkorDB` instance.
///
/// # Example
///
/// ```no_run
/// use text_to_cypher::{TextToCypherClient, ChatRequest, ChatMessage, ChatRole};
///
/// #[tokio::main]
/// async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
///     let client = TextToCypherClient::new(
///         "gpt-4o-mini",
///         "your-api-key",
///         "falkor://127.0.0.1:6379"
///     );
///
///     let request = ChatRequest {
///         messages: vec![
///             ChatMessage {
///                 role: ChatRole::User,
///                 content: "Find all nodes".to_string(),
///             }
///         ]
///     };
///
///     let response = client.text_to_cypher("my_graph", request).await?;
///     Ok(())
/// }
/// ```
pub struct TextToCypherClient {
    model: String,
    api_key: String,
    falkordb_connection: String,
}

impl TextToCypherClient {
    /// Creates a new `TextToCypherClient`.
    ///
    /// # Arguments
    ///
    /// * `model` - The AI model to use (e.g., "gpt-4o-mini", "anthropic:claude-3")
    /// * `api_key` - API key for the AI service
    /// * `falkordb_connection` - `FalkorDB` connection string (e.g., `falkor://127.0.0.1:6379`)
    ///
    /// # Example
    ///
    /// ```no_run
    /// use text_to_cypher::TextToCypherClient;
    ///
    /// let client = TextToCypherClient::new(
    ///     "gpt-4o-mini",
    ///     "your-api-key",
    ///     "falkor://127.0.0.1:6379"
    /// );
    /// ```
    #[must_use]
    pub fn new(
        model: impl Into<String>,
        api_key: impl Into<String>,
        falkordb_connection: impl Into<String>,
    ) -> Self {
        Self {
            model: model.into(),
            api_key: api_key.into(),
            falkordb_connection: falkordb_connection.into(),
        }
    }

    /// Converts natural language text to Cypher and executes the query.
    ///
    /// This is the main method for full text-to-cypher processing:
    /// 1. Discovers the graph schema
    /// 2. Generates a Cypher query using AI
    /// 3. Executes the query
    /// 4. Generates a natural language answer
    ///
    /// # Arguments
    ///
    /// * `graph_name` - Name of the graph to query
    /// * `request` - Chat request containing the user's question
    ///
    /// # Errors
    ///
    /// Returns an error if schema discovery, query generation, execution, or answer generation fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use text_to_cypher::{TextToCypherClient, ChatRequest, ChatMessage, ChatRole};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # let client = TextToCypherClient::new("gpt-4o-mini", "key", "falkor://127.0.0.1:6379");
    /// let request = ChatRequest {
    ///     messages: vec![
    ///         ChatMessage {
    ///             role: ChatRole::User,
    ///             content: "Find all actors".to_string(),
    ///         }
    ///     ]
    /// };
    ///
    /// let response = client.text_to_cypher("movies", request).await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn text_to_cypher(
        &self,
        graph_name: impl Into<String>,
        request: ChatRequest,
    ) -> Result<TextToCypherResponse, Box<dyn std::error::Error + Send + Sync>> {
        let req = TextToCypherRequest {
            graph_name: graph_name.into(),
            chat_request: request,
            model: Some(self.model.clone()),
            key: Some(self.api_key.clone()),
            falkordb_connection: Some(self.falkordb_connection.clone()),
            cypher_only: false,
            stream: false,
        };

        let response = processor::process_text_to_cypher(
            req,
            Some(self.model.clone()),
            Some(self.api_key.clone()),
            self.falkordb_connection.clone(),
        )
        .await;

        if response.is_error() {
            return Err(response.error.unwrap_or_else(|| "Unknown error".to_string()).into());
        }

        Ok(response)
    }

    /// Generates a Cypher query without executing it.
    ///
    /// Use this method when you only want to generate the query for inspection
    /// or manual execution.
    ///
    /// # Arguments
    ///
    /// * `graph_name` - Name of the graph to generate query for
    /// * `request` - Chat request containing the user's question
    ///
    /// # Errors
    ///
    /// Returns an error if schema discovery or query generation fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use text_to_cypher::{TextToCypherClient, ChatRequest, ChatMessage, ChatRole};
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # let client = TextToCypherClient::new("gpt-4o-mini", "key", "falkor://127.0.0.1:6379");
    /// let request = ChatRequest {
    ///     messages: vec![
    ///         ChatMessage {
    ///             role: ChatRole::User,
    ///             content: "Find all actors".to_string(),
    ///         }
    ///     ]
    /// };
    ///
    /// let response = client.cypher_only("movies", request).await?;
    /// println!("Generated query: {}", response.cypher_query.unwrap());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn cypher_only(
        &self,
        graph_name: impl Into<String>,
        request: ChatRequest,
    ) -> Result<TextToCypherResponse, Box<dyn std::error::Error + Send + Sync>> {
        let req = TextToCypherRequest {
            graph_name: graph_name.into(),
            chat_request: request,
            model: Some(self.model.clone()),
            key: Some(self.api_key.clone()),
            falkordb_connection: Some(self.falkordb_connection.clone()),
            cypher_only: true,
            stream: false,
        };

        let response = processor::process_text_to_cypher(
            req,
            Some(self.model.clone()),
            Some(self.api_key.clone()),
            self.falkordb_connection.clone(),
        )
        .await;

        if response.is_error() {
            return Err(response.error.unwrap_or_else(|| "Unknown error".to_string()).into());
        }

        Ok(response)
    }

    /// Discovers and returns the schema of a graph.
    ///
    /// # Arguments
    ///
    /// * `graph_name` - Name of the graph to discover schema for
    ///
    /// # Errors
    ///
    /// Returns an error if schema discovery fails.
    ///
    /// # Example
    ///
    /// ```no_run
    /// # use text_to_cypher::TextToCypherClient;
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// # let client = TextToCypherClient::new("gpt-4o-mini", "key", "falkor://127.0.0.1:6379");
    /// let schema = client.discover_schema("movies").await?;
    /// println!("Schema: {}", schema);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn discover_schema(
        &self,
        graph_name: impl Into<String>,
    ) -> Result<String, Box<dyn std::error::Error + Send + Sync>> {
        core::discover_graph_schema(&self.falkordb_connection, &graph_name.into()).await
    }

    /// Lists all available model names for a specific AI provider
    ///
    /// # Arguments
    ///
    /// * `adapter_kind` - The AI provider to query
    ///
    /// # Returns
    ///
    /// A vector of model names supported by the adapter
    ///
    /// # Errors
    ///
    /// Returns an error if the model listing request fails
    ///
    /// # Example
    ///
    /// ```no_run
    /// use text_to_cypher::TextToCypherClient;
    /// use genai::adapter::AdapterKind;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// let client = TextToCypherClient::new(
    ///     "gpt-4o-mini",
    ///     "your-api-key",
    ///     "falkor://127.0.0.1:6379"
    /// );
    ///
    /// let models = client.list_models(AdapterKind::OpenAI).await?;
    /// println!("Available OpenAI models: {:?}", models);
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_models(
        &self,
        adapter_kind: AdapterKind,
    ) -> Result<Vec<String>, Box<dyn std::error::Error + Send + Sync>> {
        let client = core::create_genai_client(Some(&self.api_key));
        core::list_adapter_models(adapter_kind, &client).await
    }

    /// Lists all available models across all supported AI providers
    ///
    /// # Returns
    ///
    /// A hashmap mapping adapter kinds to their available model names
    ///
    /// # Errors
    ///
    /// Returns an error if the model listing fails
    ///
    /// # Note
    ///
    /// This method uses the API key configured in the client. Different providers
    /// require different API keys, so only the provider matching the configured key
    /// will successfully return results. Other providers will be logged as warnings
    /// and skipped.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use text_to_cypher::TextToCypherClient;
    ///
    /// # #[tokio::main]
    /// # async fn main() -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    /// let client = TextToCypherClient::new(
    ///     "gpt-4o-mini",
    ///     "your-api-key",
    ///     "falkor://127.0.0.1:6379"
    /// );
    ///
    /// let all_models = client.list_all_models().await?;
    /// for (kind, models) in all_models {
    ///     println!("{kind}: {} models", models.len());
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn list_all_models(
        &self
    ) -> Result<std::collections::HashMap<AdapterKind, Vec<String>>, Box<dyn std::error::Error + Send + Sync>> {
        let client = core::create_genai_client(Some(&self.api_key));
        core::list_all_models(&client).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_client_creation() {
        let client = TextToCypherClient::new("gpt-4o-mini", "test-api-key", "falkor://127.0.0.1:6379");

        assert_eq!(client.model, "gpt-4o-mini");
        assert_eq!(client.api_key, "test-api-key");
        assert_eq!(client.falkordb_connection, "falkor://127.0.0.1:6379");
    }

    #[test]
    fn test_client_creation_with_string() {
        let client = TextToCypherClient::new(
            "anthropic:claude-3".to_string(),
            "key123".to_string(),
            "falkor://localhost:6379".to_string(),
        );

        assert_eq!(client.model, "anthropic:claude-3");
        assert_eq!(client.api_key, "key123");
        assert_eq!(client.falkordb_connection, "falkor://localhost:6379");
    }

    #[test]
    fn test_chat_request_construction() {
        let request = ChatRequest {
            messages: vec![
                ChatMessage {
                    role: ChatRole::User,
                    content: "Hello".to_string(),
                },
                ChatMessage {
                    role: ChatRole::Assistant,
                    content: "Hi there".to_string(),
                },
            ],
        };

        assert_eq!(request.messages.len(), 2);
        assert_eq!(request.messages[0].role, ChatRole::User);
        assert_eq!(request.messages[1].role, ChatRole::Assistant);
    }

    #[test]
    fn test_chat_role_serialization() {
        let role = ChatRole::User;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, r#""user""#);

        let role = ChatRole::Assistant;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, r#""assistant""#);

        let role = ChatRole::System;
        let json = serde_json::to_string(&role).unwrap();
        assert_eq!(json, r#""system""#);
    }

    #[test]
    fn test_chat_message_serialization() {
        let message = ChatMessage {
            role: ChatRole::User,
            content: "Test message".to_string(),
        };

        let json = serde_json::to_string(&message).unwrap();
        let deserialized: ChatMessage = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.role, ChatRole::User);
        assert_eq!(deserialized.content, "Test message");
    }

    #[test]
    fn test_chat_request_serialization() {
        let request = ChatRequest {
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: "Find all nodes".to_string(),
            }],
        };

        let json = serde_json::to_string(&request).unwrap();
        let deserialized: ChatRequest = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.messages.len(), 1);
        assert_eq!(deserialized.messages[0].content, "Find all nodes");
    }

    #[test]
    fn test_error_response_structure() {
        let error = ErrorResponse {
            error: "Test error".to_string(),
            message: "Detailed message".to_string(),
            status_code: 500,
        };

        assert_eq!(error.error, "Test error");
        assert_eq!(error.message, "Detailed message");
        assert_eq!(error.status_code, 500);
    }

    #[test]
    fn test_chat_role_equality() {
        assert_eq!(ChatRole::User, ChatRole::User);
        assert_eq!(ChatRole::Assistant, ChatRole::Assistant);
        assert_eq!(ChatRole::System, ChatRole::System);
        assert_ne!(ChatRole::User, ChatRole::Assistant);
    }

    #[test]
    fn test_client_with_different_models() {
        let models = vec!["gpt-4o-mini", "gpt-4o", "anthropic:claude-3", "gemini:gemini-2.0-flash-exp"];

        for model in models {
            let client = TextToCypherClient::new(model, "key", "falkor://localhost:6379");
            assert_eq!(client.model, model);
        }
    }
}
