//! Streaming processor for Server-Sent Events (SSE)
//!
//! This module provides streaming progress updates during text-to-cypher conversion for Vercel.

use crate::chat::ChatRequest;
use crate::core::{
    create_genai_client, discover_graph_schema, execute_cypher_query, generate_cypher_query, generate_final_answer,
};
use futures::stream::Stream;
use serde::{Deserialize, Serialize};
use std::error::Error;
use std::pin::Pin;

/// Type alias for the stream returned by `process_text_to_cypher_stream`
pub type ProgressStream = Pin<Box<dyn Stream<Item = Result<String, Box<dyn Error + Send + Sync>>> + Send>>;

/// Progress update events (matching standalone server format)
#[derive(Serialize, Deserialize, Clone)]
pub enum Progress {
    Status(String),
    Schema(String),
    CypherQuery(String),
    CypherResult(String),
    ModelOutputChunk(String),
    Result(String),
    Error(String),
}

impl Progress {
    /// Format as SSE message
    #[must_use]
    pub fn to_sse(&self) -> String {
        match serde_json::to_string(self) {
            Ok(json) => format!("data: {json}\n\n"),
            Err(e) => {
                let error_payload = serde_json::json!({ "Error": e.to_string() });
                format!("data: {error_payload}\n\n")
            }
        }
    }
}

/// Process text-to-cypher with streaming progress updates
#[must_use]
pub fn process_text_to_cypher_stream(
    graph_name: String,
    chat_request: ChatRequest,
    model: Option<String>,
    key: Option<String>,
    falkordb_connection: String,
    cypher_only: bool,
) -> ProgressStream {
    let events = async_stream::stream! {
        // Step 1: Create AI client
        yield Ok(Progress::Status("Initializing AI client...".to_string()).to_sse());

        let client = create_genai_client(key.as_deref());
        let model = model.unwrap_or_else(|| "gpt-4o-mini".to_string());

        // Step 2: Discover schema (unless cypher_only)
        let schema = if cypher_only {
            yield Ok(Progress::Status("Skipping schema discovery (cypher_only mode)".to_string()).to_sse());
            "{}".to_string()
        } else {
            yield Ok(Progress::Status("Connecting to database...".to_string()).to_sse());
            yield Ok(Progress::Status("Discovering graph schema...".to_string()).to_sse());

            match discover_graph_schema(&falkordb_connection, &graph_name).await {
                Ok(s) => {
                    yield Ok(Progress::Schema(s.clone()).to_sse());
                    s
                }
                Err(e) => {
                    yield Ok(Progress::Error(format!("Failed to discover schema: {e}")).to_sse());
                    return;
                }
            }
        };

        // Step 3: Generate Cypher query
        yield Ok(Progress::Status("Generating Cypher query with AI...".to_string()).to_sse());

        let cypher_query = match generate_cypher_query(&chat_request, &schema, &client, &model).await {
            Ok(q) => {
                yield Ok(Progress::CypherQuery(q.clone()).to_sse());
                q
            }
            Err(e) => {
                yield Ok(Progress::Error(format!("Failed to generate query: {e}")).to_sse());
                return;
            }
        };

        // If cypher_only, stop here
        if cypher_only {
            return;
        }

        // Step 4: Execute query
        yield Ok(Progress::Status("Executing Cypher query on database...".to_string()).to_sse());

        let cypher_result = match execute_cypher_query(&cypher_query, &graph_name, &falkordb_connection, true).await {
            Ok(r) => {
                yield Ok(Progress::CypherResult(r.clone()).to_sse());
                r
            }
            Err(e) => {
                yield Ok(Progress::Error(format!("Query execution failed: {e}")).to_sse());
                return;
            }
        };

        // Step 5: Generate final answer
        yield Ok(Progress::Status("Generating natural language answer...".to_string()).to_sse());

        match generate_final_answer(&chat_request, &cypher_query, &cypher_result, &client, &model).await {
            Ok(answer) => {
                yield Ok(Progress::Result(answer).to_sse());
            }
            Err(e) => {
                yield Ok(Progress::Error(format!("Failed to generate answer: {e}")).to_sse());
                return;
            }
        }
    };

    Box::pin(events)
}
