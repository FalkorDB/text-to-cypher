# Text-to-Cypher Library API Documentation

This document provides detailed information about using text-to-cypher as a library in your Rust applications.

## Testing

The library includes comprehensive unit tests covering all public APIs. See:
- [Library tests](../src/lib.rs#L409) - Tests for `TextToCypherClient` and core types
- [Processor tests](../src/processor.rs#L273) - Tests for `TextToCypherRequest` and `TextToCypherResponse`
- [Validator tests](../src/validator.rs) - Tests for Cypher query validation
- [Formatter tests](../src/formatter.rs) - Tests for result formatting
- [Schema tests](../src/schema/discovery.rs) - Tests for schema discovery

Run tests with:
```bash
cargo test --lib
```

## Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
# For library usage only (minimal dependencies)
text-to-cypher = { version = "0.1", default-features = false }

# For full server capabilities (includes REST API)
text-to-cypher = "0.1"
```

## Quick Start

```rust
use text_to_cypher::{TextToCypherClient, ChatRequest, ChatMessage, ChatRole};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = TextToCypherClient::new(
        "gpt-4o-mini",           // AI model
        "your-api-key",          // API key
        "falkor://localhost:6379" // FalkorDB connection
    );

    let request = ChatRequest {
        messages: vec![
            ChatMessage {
                role: ChatRole::User,
                content: "Find all people nodes".to_string(),
            }
        ]
    };

    let response = client.text_to_cypher("my_graph", request).await?;
    println!("Result: {:?}", response);
    Ok(())
}
```

## API Reference

### `TextToCypherClient`

The high-level client for text-to-cypher operations.

#### Constructor

```rust
pub fn new(
    model: impl Into<String>,
    api_key: impl Into<String>,
    falkordb_connection: impl Into<String>
) -> Self
```

Creates a new client instance.

**Parameters:**
- `model`: AI model identifier (e.g., "gpt-4o-mini", "anthropic:claude-3", "gemini:gemini-2.0-flash-exp")
- `api_key`: API key for the AI service
- `falkordb_connection`: FalkorDB connection string (e.g., "falkor://localhost:6379")

**Example:**
```rust
let client = TextToCypherClient::new(
    "gpt-4o-mini",
    "sk-...",
    "falkor://localhost:6379"
);
```

#### Methods

##### `text_to_cypher`

```rust
pub async fn text_to_cypher(
    &self,
    graph_name: impl Into<String>,
    request: ChatRequest,
) -> Result<TextToCypherResponse, Box<dyn std::error::Error + Send + Sync>>
```

Converts natural language to Cypher, executes the query, and generates a natural language answer.

**Process:**
1. Discovers the graph schema
2. Generates a Cypher query using AI
3. Executes the query against FalkorDB
4. Generates a natural language answer from the results

**Parameters:**
- `graph_name`: Name of the graph to query
- `request`: Chat request containing the user's question

**Returns:**
- `TextToCypherResponse` with schema, query, result, and answer
- Or an error if any step fails

**Example:**
```rust
let request = ChatRequest {
    messages: vec![
        ChatMessage {
            role: ChatRole::User,
            content: "Show me all actors".to_string(),
        }
    ]
};

let response = client.text_to_cypher("movies", request).await?;
println!("Query: {}", response.cypher_query.unwrap());
println!("Answer: {}", response.answer.unwrap());
```

##### `cypher_only`

```rust
pub async fn cypher_only(
    &self,
    graph_name: impl Into<String>,
    request: ChatRequest,
) -> Result<TextToCypherResponse, Box<dyn std::error::Error + Send + Sync>>
```

Generates a Cypher query without executing it.

Use this when you want to:
- Preview the generated query
- Execute the query manually
- Modify the query before execution

**Parameters:**
- `graph_name`: Name of the graph
- `request`: Chat request containing the user's question

**Returns:**
- `TextToCypherResponse` with only the schema and cypher_query fields populated

**Example:**
```rust
let request = ChatRequest {
    messages: vec![
        ChatMessage {
            role: ChatRole::User,
            content: "Find people with more than 5 friends".to_string(),
        }
    ]
};

let response = client.cypher_only("social", request).await?;
println!("Generated query: {}", response.cypher_query.unwrap());
// Now you can review, modify, or execute the query yourself
```

##### `discover_schema`

```rust
pub async fn discover_schema(
    &self,
    graph_name: impl Into<String>,
) -> Result<String, Box<dyn std::error::Error + Send + Sync>>
```

Discovers and returns the schema of a graph as JSON.

**Parameters:**
- `graph_name`: Name of the graph

**Returns:**
- JSON string representing the graph schema

**Example:**
```rust
let schema = client.discover_schema("movies").await?;
println!("Schema: {}", schema);
```

### Core Functions

For more control, you can use the core functions directly:

```rust
use text_to_cypher::{core, ChatRequest, ChatMessage, ChatRole};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // 1. Discover schema
    let schema = core::discover_graph_schema(
        "falkor://localhost:6379",
        "movies"
    ).await?;
    
    // 2. Create GenAI client
    let genai_client = core::create_genai_client(Some("your-api-key"));
    
    // 3. Generate query
    let chat_req = ChatRequest {
        messages: vec![
            ChatMessage {
                role: ChatRole::User,
                content: "Find all actors".to_string(),
            }
        ]
    };
    
    let query = core::generate_cypher_query(
        &chat_req,
        &schema,
        &genai_client,
        "gpt-4o-mini"
    ).await?;
    
    // 4. Execute query
    let result = core::execute_cypher_query(
        &query,
        "movies",
        "falkor://localhost:6379",
        true  // read_only
    ).await?;
    
    // 5. Generate natural language answer
    let answer = core::generate_final_answer(
        &chat_req,
        &query,
        &result,
        &genai_client,
        "gpt-4o-mini"
    ).await?;
    
    println!("Answer: {}", answer);
    Ok(())
}
```

## Data Structures

### `ChatRequest`

```rust
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
}
```

### `ChatMessage`

```rust
pub struct ChatMessage {
    pub role: ChatRole,
    pub content: String,
}
```

### `ChatRole`

```rust
pub enum ChatRole {
    User,
    Assistant,
    System,
}
```

### `TextToCypherResponse`

```rust
pub struct TextToCypherResponse {
    pub status: String,
    pub schema: Option<String>,
    pub cypher_query: Option<String>,
    pub cypher_result: Option<String>,
    pub answer: Option<String>,
    pub error: Option<String>,
}
```

## Supported AI Models

The library uses the [genai](https://crates.io/crates/genai) crate, which supports:

- **OpenAI**: `gpt-4o-mini`, `gpt-4o`, `gpt-4-turbo`, etc.
- **Anthropic**: `anthropic:claude-3-5-sonnet-20241022`, `anthropic:claude-3-opus-20240229`
- **Google Gemini**: `gemini:gemini-2.0-flash-exp`, `gemini:gemini-1.5-pro`
- **And more**: Check [genai documentation](https://docs.rs/genai/latest/genai/) for full list

## Error Handling

All async methods return `Result<T, Box<dyn std::error::Error + Send + Sync>>`.

Common errors:
- Connection failures to FalkorDB
- AI service errors (invalid API key, rate limits, etc.)
- Schema discovery failures
- Query generation or execution failures

Example error handling:

```rust
match client.text_to_cypher("my_graph", request).await {
    Ok(response) => {
        if response.status == "success" {
            println!("Success: {:?}", response.answer);
        }
    }
    Err(e) => {
        eprintln!("Error: {}", e);
        // Handle specific error cases
    }
}
```

## Complete Example

See [examples/library_usage.rs](../examples/library_usage.rs) for a comprehensive example demonstrating:
- Using the high-level client
- Using core functions directly
- Generating queries without execution
- Error handling

Run it with:
```bash
cargo run --example library_usage --no-default-features
```

## Features

The library has two feature sets:

1. **Default (with `server` feature)**: Includes REST API server, Swagger UI, MCP server
   ```toml
   text-to-cypher = "0.1"
   ```

2. **Library-only (without `server` feature)**: Core functionality only
   ```toml
   text-to-cypher = { version = "0.1", default-features = false }
   ```

The library-only mode excludes:
- actix-web and HTTP server dependencies
- Swagger/OpenAPI dependencies
- MCP server dependencies
- Other server-specific dependencies

This results in a smaller binary and faster compile times.

## Best Practices

1. **Reuse the client**: Create one `TextToCypherClient` instance and reuse it for multiple requests
2. **Handle schemas efficiently**: The schema is discovered once per request; consider caching it if needed
3. **Use cypher_only for validation**: Generate queries first to validate them before execution
4. **Error handling**: Always handle errors appropriately in production code
5. **Connection pooling**: The underlying FalkorDB client handles connections efficiently

## License

MIT License - see [LICENSE](../LICENSE) file for details.
