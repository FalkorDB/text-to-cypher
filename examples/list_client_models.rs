//! Example demonstrating `TextToCypherClient::list_models` and `list_all_models`
//!
//! This example shows how to discover which AI models are available, using the
//! high-level `TextToCypherClient` API:
//! - `list_models(adapter_kind)` lists the models for a single provider
//! - `list_all_models()` lists models across all supported providers
//!
//! No `FalkorDB` connection is required for listing models, but the client is
//! configured with an API key which is forwarded to the providers.
//!
//! To run this example:
//! 1. Set an API key for the provider you want to query, e.g.
//!    `export OPENAI_API_KEY=your-key-here`
//! 2. Run: `cargo run --example list_client_models --no-default-features`

use text_to_cypher::{AdapterKind, TextToCypherClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing for better debugging (only if available).
    #[cfg(feature = "server")]
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // Pick up an API key from the environment. Listing models for some providers
    // (e.g. Ollama) works without a key, while hosted providers require one.
    let api_key = std::env::var("OPENAI_API_KEY")
        .or_else(|_| std::env::var("ANTHROPIC_API_KEY"))
        .or_else(|_| std::env::var("GEMINI_API_KEY"))
        .unwrap_or_default();

    // The model and connection string are not used for listing models, but the
    // client requires them. The configured API key is what gets forwarded.
    let client = TextToCypherClient::new("gpt-4o-mini", &api_key, "falkor://127.0.0.1:6379");

    // Method 1: list models for a single provider.
    println!("=== Method 1: list_models for a single provider ===\n");
    let adapter = AdapterKind::OpenAI;
    match client.list_models(adapter).await {
        Ok(models) => {
            println!("{adapter}: found {} models", models.len());
            for model in &models {
                println!("  • {model}");
            }
        }
        Err(e) => eprintln!("  ✗ Error fetching {adapter} models: {e}"),
    }

    // Method 2: list models across all supported providers at once.
    //
    // Note: each provider's live results are merged with a curated static catalog, so
    // providers with a catalog still return their well-known models even without a
    // matching API key. Providers without a catalog (e.g. Ollama) only appear when
    // reachable.
    println!("\n=== Method 2: list_all_models across all providers ===\n");
    match client.list_all_models().await {
        Ok(all_models) => {
            let total: usize = all_models.values().map(Vec::len).sum();
            println!("Total models across all providers: {total}\n");
            for (kind, models) in all_models {
                println!("{kind}: {} models", models.len());
            }
        }
        Err(e) => eprintln!("Error fetching all models: {e}"),
    }

    println!("\n=== Example completed successfully! ===");
    Ok(())
}
