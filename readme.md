# Text-to-Cypher

[![build](https://github.com/FalkorDB/text-to-cypher/actions/workflows/build.yml/badge.svg)](https://github.com/FalkorDB/text-to-cypher/actions/workflows/build.yml)

Text-to-Cypher is a Rust library and API service that translates natural language text into Cypher queries for graph databases. It integrates with genai and FalkorDB. Run the Docker image to start FalkorDB, the web UI, the REST API, and the MCP server.

![Text-to-Cypher hero GIF](docs/hero-gif/out/hero.gif)

## Who This Is For

- Developers who need natural language to Cypher conversion for graph databases
- Teams that use FalkorDB and need a library or HTTP service for query generation
- Integrators who need MCP server support for AI assistant tooling

## What It Solves

Text-to-Cypher turns natural language questions into Cypher queries, validates them, and can execute them against FalkorDB. It also exposes a REST API and an MCP server for tool integration.

## Core Capabilities

- Translate natural language into Cypher
- Discover graph schema with example values
- Validate Cypher before execution
- Retry failed queries with error feedback
- Provide a Rust library and a REST API
- Stream progress updates over Server-Sent Events

## Quick Start

### Docker

```bash
docker run -p 6379:6379 -p 3000:3000 -p 8080:8080 -p 3001:3001 \
  -e DEFAULT_MODEL=gpt-4o-mini -e DEFAULT_KEY=your-api-key \
  ghcr.io/falkordb/text-to-cypher:latest
```

### Rust Library

Add to `Cargo.toml`:

```toml
[dependencies]
text-to-cypher = { version = "0.1", default-features = false }
```

Basic example:

```rust
use text_to_cypher::{ChatMessage, ChatRequest, ChatRole, TextToCypherClient};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = TextToCypherClient::new(
        "gpt-4o-mini",
        "your-api-key",
        "falkor://127.0.0.1:6379",
    );

    let request = ChatRequest {
        messages: vec![ChatMessage {
            role: ChatRole::User,
            content: "Find all actors who appeared in movies released after 2020".to_string(),
        }],
    };

    let response = client.text_to_cypher("movies", request).await?;

    println!("Generated query: {}", response.cypher_query.unwrap());
    println!("Result: {}", response.cypher_result.unwrap());
    println!("Answer: {}", response.answer.unwrap());

    Ok(())
}
```

More examples live in [examples/library_usage.rs](examples/library_usage.rs).

### Language Clients

- TypeScript and JavaScript: [docs/TYPESCRIPT_USAGE.md](docs/TYPESCRIPT_USAGE.md)
- Python: [docs/PYTHON_USAGE.md](docs/PYTHON_USAGE.md)

## Services and Ports

| Service | Port | Description |
| --- | --- | --- |
| FalkorDB | 6379 | Redis protocol access to graph database |
| FalkorDB Web UI | 3000 | Graph browser and query interface |
| Text-to-Cypher API | 8080 | REST API and Swagger UI |
| MCP Server | 3001 | Model Context Protocol server |

Access the Swagger UI at `http://localhost:8080/swagger-ui/` and the OpenAPI spec at `http://localhost:8080/api-doc/openapi.json`.

## Configuration

Set configuration with environment variables or a `.env` file.

### Core Settings

- `DEFAULT_MODEL`: default AI model
- `DEFAULT_KEY`: default API key

### Ports

- `REST_PORT`: REST API port, default `8080`
- `MCP_PORT`: MCP server port, default `3001`

### FalkorDB

- `FALKORDB_CONNECTION`: FalkorDB connection string, default `falkor://127.0.0.1:6379`

### MCP Server Start Conditions

The MCP server starts when `DEFAULT_MODEL` and `DEFAULT_KEY` are set. A `.env` file is optional and only loads values into the environment.

## Local Development

```bash
docker run -d -p 6379:6379 falkordb/falkordb:latest

git clone https://github.com/FalkorDB/text-to-cypher.git
cd text-to-cypher
cp .env.example .env
cargo run
```

## API Usage Examples

### Basic Request

```bash
curl -X POST "http://localhost:8080/text_to_cypher" \
  -H "Content-Type: application/json" \
  -d '{
    "graph_name": "movies",
    "chat_request": {
      "messages": [
        {
          "role": "user",
          "content": "Find all actors who appeared in movies released after 2020"
        }
      ]
    },
    "model": "gpt-4o-mini",
    "key": "your-api-key"
  }'
```

### Streaming With SSE

```bash
curl -N -X POST "http://localhost:8080/text_to_cypher" \
  -H "Accept: text/event-stream" \
  -H "Content-Type: application/json" \
  -d '{
    "graph_name": "social_network",
    "chat_request": {
      "messages": [
        {"role": "user", "content": "Who are John\'s friends?"}
      ]
    }
  }'
```

## MCP Server Usage

### MCP Inspector

1. Start the stack:

```bash
docker run -p 6379:6379 -p 3000:3000 -p 8080:8080 -p 3001:3001 \
  -e DEFAULT_MODEL=gpt-4o-mini -e DEFAULT_KEY=your-api-key \
  ghcr.io/falkordb/text-to-cypher:latest
```

1. Install the inspector:

```bash
npx -y @modelcontextprotocol/inspector
```

1. Connect:

- Transport: `stdio`
- Command: `nc`
- Arguments: `["localhost", "3001"]`

Or use the SSE transport at `http://localhost:3001/sse`.

### Tool: `text_to_cypher`

Parameters:

- `graph_name` (required)
- `question` (required)

Example:

```json
{
  "graph_name": "movies",
  "question": "Find all actors who appeared in movies released after 2020"
}
```

## Testing

Run tests:

```bash
cargo test --lib
cargo test
cargo test -- --nocapture
```

## Development

```bash
cargo fmt
cargo clippy -- -W clippy::pedantic -W clippy::nursery -W clippy::cargo -A clippy::missing-errors-doc -A clippy::missing-panics-doc -A clippy::multiple-crate-versions
cargo test
```

## Project Structure

```text
text-to-cypher/
├── src/
│   ├── main.rs              # Main application and HTTP server
│   ├── chat.rs              # Chat message types and handling
│   ├── error.rs             # Error types and handling
│   ├── formatter.rs         # Query result formatting
│   ├── mcp/                 # Model Context Protocol server
│   ├── schema/              # Graph schema discovery
│   └── template.rs          # Template engine for prompts
├── templates/               # AI prompt templates
│   ├── system_prompt.txt    # System prompt for AI
│   ├── user_prompt.txt      # User query template
│   └── last_request_prompt.txt # Final response template
├── Dockerfile               # Docker image with FalkorDB
├── supervisord.conf         # Process management configuration
├── entrypoint.sh            # Container startup script
├── .dockerignore            # Docker build context filtering
└── docker-build.sh          # Docker build script
```

## Publishing to crates.io

Maintainership steps live in [RELEASES.md](RELEASES.md).

## FAQ

| Question | Answer |
| --- | --- |
| Do I need FalkorDB separately when I use the Docker image? | No. The image includes FalkorDB and the web UI. |
| Can I use the library without running the server? | Yes. Set `default-features = false` in `Cargo.toml`. |
| How do I enable the MCP server? | Set `DEFAULT_MODEL` and `DEFAULT_KEY`. You can use environment variables or a `.env` file. |

## Troubleshooting

| Issue | Resolution |
| --- | --- |
| Services not starting | Check that ports 6379, 3000, 8080, and 3001 are free. Set `DEFAULT_MODEL` and `DEFAULT_KEY`. View logs with `docker logs -f <container-name>`. |
| MCP server not starting | Set `DEFAULT_MODEL` and `DEFAULT_KEY`. |
| FalkorDB connection issues | Use `FALKORDB_CONNECTION` or pass `falkordb_connection` in requests. Connect to `localhost:6379` when running the container locally. |
| Web UI not accessible | Map port 3000 with `-p 3000:3000`. Open `http://localhost:3000`. |

## References

- [docs/IMPROVEMENTS.md](docs/IMPROVEMENTS.md)
- [docs/BEST_PRACTICES.md](docs/BEST_PRACTICES.md)

## License

This project uses the MIT License.
