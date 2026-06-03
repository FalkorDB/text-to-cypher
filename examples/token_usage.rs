//! Example demonstrating token-usage tracking with the text-to-cypher library.
//!
//! A single `text_to_cypher` request may issue several LLM calls (schema-aware Cypher
//! generation, optional self-healing retries, skill tool-call rounds, and final answer
//! generation). The library aggregates the token counts from every one of those calls
//! into `TextToCypherResponse::token_usage`.
//!
//! This example runs a real request against an `OpenAI` model and prints the aggregated
//! `TokenUsage`.
//!
//! To run this example:
//! 1. Ensure `FalkorDB` is running, e.g.:
//!    `docker run -d -p 6379:6379 falkordb/falkordb:latest`
//! 2. Export your `OpenAI` key: `export OPENAI_API_KEY=sk-...`
//! 3. Optionally override defaults via `MODEL` (defaults to `gpt-5.5`),
//!    `FALKORDB_CONNECTION`, or `GRAPH_NAME`.
//! 4. Run: `cargo run --example token_usage --no-default-features`

use text_to_cypher::{ChatMessage, ChatRequest, ChatRole, TextToCypherClient, TextToCypherResponse, TokenUsage, core};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing so the per-call usage accumulation is visible (server feature only).
    #[cfg(feature = "server")]
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let api_key = std::env::var("OPENAI_API_KEY")
        .expect("Please set OPENAI_API_KEY to run this example (e.g. export OPENAI_API_KEY=sk-...)");

    // An OpenAI model. `gpt-5.5` reports usage; override via the `MODEL` env var.
    let model = std::env::var("MODEL").unwrap_or_else(|_| "gpt-5.5".to_string());
    let falkordb_connection =
        std::env::var("FALKORDB_CONNECTION").unwrap_or_else(|_| "falkor://127.0.0.1:6379".to_string());
    let graph_name = std::env::var("GRAPH_NAME").unwrap_or_else(|_| "demo_graph".to_string());

    println!("=== Token Usage Tracking Example ===");
    println!("model: {model}");
    println!("connection: {falkordb_connection}");
    println!("graph: {graph_name}\n");

    let client = TextToCypherClient::new(&model, &api_key, &falkordb_connection);

    // Seed the graph with a little data so the generated query has something to run
    // against and so schema discovery returns a non-empty ontology. We write directly
    // with a Cypher query (not via the LLM) because schema discovery fails on a graph
    // that does not exist yet.
    seed_graph(&falkordb_connection, &graph_name).await;

    // Example 1: full pipeline (generate -> execute -> answer). Usage is summed across
    // every LLM call the request made.
    println!("--- Example 1: full text_to_cypher pipeline ---");
    let request = chat("How many people are in the graph?");
    match client.text_to_cypher(&graph_name, request).await {
        Ok(response) => report_usage(&response),
        Err(e) => eprintln!("✗ Request failed: {e}"),
    }

    // Example 2: cypher-only (no execution / no answer generation). Usage should reflect
    // just the query-generation call(s) and therefore typically be smaller.
    println!("\n--- Example 2: cypher_only (generation only) ---");
    let request = chat("List the names of all people.");
    match client.cypher_only(&graph_name, request).await {
        Ok(response) => {
            if let Some(query) = &response.cypher_query {
                println!("Generated query: {query}");
            }
            report_usage(&response);
        }
        Err(e) => eprintln!("✗ Request failed: {e}"),
    }

    println!("\n=== Example completed ===");
    Ok(())
}

/// Builds a single-message user chat request.
fn chat(content: &str) -> ChatRequest {
    ChatRequest {
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: content.to_string(),
        }],
    }
}

/// Prints the aggregated token usage from a response, making it obvious whether the
/// feature reported any tokens.
fn report_usage(response: &TextToCypherResponse) {
    if let Some(answer) = &response.answer {
        println!("Answer: {answer}");
    }

    match response.token_usage {
        Some(TokenUsage {
            prompt_tokens,
            completion_tokens,
            total_tokens,
        }) => {
            println!("Token usage:");
            println!("  prompt_tokens     = {prompt_tokens}");
            println!("  completion_tokens = {completion_tokens}");
            println!("  total_tokens      = {total_tokens}");

            if total_tokens == 0 {
                eprintln!("⚠ token_usage was reported but total_tokens is 0 — the provider may not return usage data.");
            } else {
                println!("✓ Token usage tracking is working.");
            }
        }
        None => {
            eprintln!("✗ No token_usage reported on the response — the provider may not return usage data.");
        }
    }
}

/// Creates a few `Person` nodes (directly, without the LLM) so the generated queries
/// have data to run against and schema discovery returns a non-empty ontology.
/// Failures here are non-fatal; the example continues so usage can still be shown.
async fn seed_graph(
    falkordb_connection: &str,
    graph_name: &str,
) {
    let seed_query = "MERGE (:Person {name: 'Alice'}) MERGE (:Person {name: 'Bob'}) MERGE (:Person {name: 'Charlie'})";
    match core::execute_cypher_query(seed_query, graph_name, falkordb_connection, false).await {
        Ok(_) => println!("Seeded graph '{graph_name}' with sample data.\n"),
        Err(e) => println!("Note: seeding graph failed (continuing anyway): {e}\n"),
    }
}
