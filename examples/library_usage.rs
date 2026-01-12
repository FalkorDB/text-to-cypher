//! Example demonstrating how to use text-to-cypher as a library
//!
//! This example shows various ways to use the text-to-cypher library
//! in your Rust application, without using the REST API.
//!
//! To run this example:
//! 1. Ensure `FalkorDB` is running on localhost:6379
//! 2. Set your API key: export OPENAI_API_KEY=your-key-here
//! 3. Run: cargo run --example `library_usage` --no-default-features

use text_to_cypher::{ChatMessage, ChatRequest, ChatRole, TextToCypherClient, core};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for logging (only if tracing_subscriber is available)
    #[cfg(feature = "server")]
    tracing_subscriber::fmt().with_max_level(tracing::Level::INFO).init();

    println!("=== Text-to-Cypher Library Usage Examples ===\n");

    // Get API key from environment
    let api_key = std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::var("ANTHROPIC_API_KEY"))
        .or_else(|_| std::env::var("GEMINI_API_KEY"))
        .expect("Please set OPENAI_API_KEY, ANTHROPIC_API_KEY, or GEMINI_API_KEY environment variable");

    // Configuration
    let model = "gpt-4o-mini"; // or "anthropic:claude-3-5-sonnet-20241022", "gemini:gemini-2.0-flash-exp"
    let falkordb_connection = "falkor://127.0.0.1:6379";
    let graph_name = "demo_graph";

    // Example 1: Using the high-level client
    println!("Example 1: Using TextToCypherClient");
    println!("=====================================");
    example_with_client(&api_key, model, falkordb_connection, graph_name).await?;

    // Example 2: Using core functions directly for more control
    println!("\nExample 2: Using Core Functions Directly");
    println!("=========================================");
    example_with_core_functions(&api_key, model, falkordb_connection, graph_name).await?;

    // Example 3: Generate Cypher only (without execution)
    println!("\nExample 3: Generate Cypher Only");
    println!("================================");
    example_cypher_only(&api_key, model, falkordb_connection, graph_name).await?;

    println!("\n=== All examples completed successfully! ===");
    Ok(())
}

/// Example using the high-level `TextToCypherClient`
async fn example_with_client(
    api_key: &str,
    model: &str,
    falkordb_connection: &str,
    graph_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // Create a client
    let client = TextToCypherClient::new(model, api_key, falkordb_connection);

    // Discover the schema first (optional, but helpful to see what's available)
    println!("Discovering graph schema...");
    match client.discover_schema(graph_name).await {
        Ok(schema) => {
            println!("Schema discovered: {schema}");
        }
        Err(e) => {
            println!("Note: Schema discovery failed (graph may not exist yet): {e}");
            println!("This is okay - continuing with examples...");
        }
    }

    // Create a simple question
    let request = ChatRequest {
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "Create a simple example with 3 people nodes named Alice, Bob, and Charlie".to_string(),
        }],
    };

    println!("\nProcessing question: {:?}", request.messages[0].content);

    // Process the request
    match client.text_to_cypher(graph_name, request).await {
        Ok(response) => {
            println!("✓ Success!");
            if let Some(query) = &response.cypher_query {
                println!("  Generated Query: {query}");
            }
            if let Some(result) = &response.cypher_result {
                println!("  Query Result: {result}");
            }
            if let Some(answer) = &response.answer {
                println!("  AI Answer: {answer}");
            }
        }
        Err(e) => {
            println!("✗ Error: {e}");
        }
    }

    Ok(())
}

/// Example using core functions directly
async fn example_with_core_functions(
    api_key: &str,
    model: &str,
    falkordb_connection: &str,
    graph_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("Step 1: Creating GenAI client...");
    let genai_client = core::create_genai_client(Some(api_key));

    println!("Step 2: Discovering graph schema...");
    let schema = match core::discover_graph_schema(falkordb_connection, graph_name).await {
        Ok(s) => {
            println!("  ✓ Schema discovered");
            s
        }
        Err(e) => {
            println!("  Note: Using empty schema (graph may not exist): {e}");
            "{}".to_string()
        }
    };

    println!("Step 3: Generating Cypher query...");
    let chat_request = ChatRequest {
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "Find all people nodes".to_string(),
        }],
    };

    match core::generate_cypher_query(&chat_request, &schema, &genai_client, model).await {
        Ok(query) => {
            println!("  ✓ Query generated: {query}");

            println!("Step 4: Executing query...");
            match core::execute_cypher_query(&query, graph_name, falkordb_connection, true).await {
                Ok(result) => {
                    println!("  ✓ Query executed successfully");
                    println!("  Result: {result}");

                    println!("Step 5: Generating natural language answer...");
                    match core::generate_final_answer(&chat_request, &query, &result, &genai_client, model).await {
                        Ok(answer) => {
                            println!("  ✓ Answer generated: {answer}");
                        }
                        Err(e) => {
                            println!("  ✗ Failed to generate answer: {e}");
                        }
                    }
                }
                Err(e) => {
                    println!("  ✗ Query execution failed: {e}");
                }
            }
        }
        Err(e) => {
            println!("  ✗ Query generation failed: {e}");
        }
    }

    Ok(())
}

/// Example generating Cypher query only (without execution)
async fn example_cypher_only(
    api_key: &str,
    model: &str,
    falkordb_connection: &str,
    graph_name: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let client = TextToCypherClient::new(model, api_key, falkordb_connection);

    let request = ChatRequest {
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "Find all people who have more than 5 friends".to_string(),
        }],
    };

    println!("Generating Cypher query for: {:?}", request.messages[0].content);

    match client.cypher_only(graph_name, request).await {
        Ok(response) => {
            println!("✓ Query generated successfully!");
            if let Some(query) = &response.cypher_query {
                println!("  Generated Query: {query}");
                println!("\n  You can now:");
                println!("  1. Review the query for correctness");
                println!("  2. Execute it manually");
                println!("  3. Modify it as needed");
            }
        }
        Err(e) => {
            println!("✗ Error: {e}");
        }
    }

    Ok(())
}
