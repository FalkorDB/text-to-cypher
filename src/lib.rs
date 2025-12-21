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
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
/// async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    ///
    /// TODO: Consider creating a specific error enum instead of Box<dyn Error>
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
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
    /// # async fn main() -> Result<(), Box<dyn std::error::Error>> {
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
}
