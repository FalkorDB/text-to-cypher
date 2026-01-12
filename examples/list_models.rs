//! Example demonstrating how to list available AI models
//!
//! This example shows how to query all supported AI providers
//! and list their available models.
//!
//! To run this example:
//!  ```bash
//! cargo run --example list_models --no-default-features
//!  ```

use text_to_cypher::{AdapterKind, core};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Define adapters to check
    const ADAPTERS: &[AdapterKind] = &[
        AdapterKind::OpenAI,
        AdapterKind::Ollama,
        AdapterKind::Gemini,
        AdapterKind::Anthropic,
        AdapterKind::Groq,
        AdapterKind::Cohere,
    ];

    // Initialize tracing for better debugging (only if available)
    #[cfg(feature = "server")]
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    println!("=== Listing All Supported AI Models ===\n");

    // Create a GenAI client (no API key needed for listing models in most cases)
    let client = core::create_genai_client(None);

    // Method 1: List models for a specific adapter
    println!("Method 1: List models for specific adapter");
    println!("-------------------------------------------");

    for &adapter in ADAPTERS {
        println!("\n--- Models for {adapter}");
        match core::list_adapter_models(adapter, &client).await {
            Ok(models) => {
                println!("Found {} models:", models.len());
                for model in &models {
                    println!("  • {model}");
                }
            }
            Err(e) => {
                eprintln!("  ✗ Error fetching models:  {e}");
            }
        }
    }

    // Method 2: List all models at once
    println!("\n\nMethod 2: List all models at once");
    println!("----------------------------------");

    match core::list_all_models(&client).await {
        Ok(all_models) => {
            let total: usize = all_models.values().map(Vec::len).sum();
            println!("\nTotal models across all providers: {total}\n");

            for (kind, models) in all_models {
                println!("{kind}: {} models", models.len());
            }
        }
        Err(e) => {
            eprintln!("Error fetching all models: {e}");
        }
    }

    println!("\n=== Example completed successfully! ===");
    Ok(())
}
