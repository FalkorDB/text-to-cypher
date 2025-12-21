# Text to Cypher

[![build](https://github.com/FalkorDB/text-to-cypher/actions/workflows/build.yml/badge.svg)](https://github.com/FalkorDB/text-to-cypher/actions/workflows/build.yml)

A high-performance Rust library and API service that translates natural language text to Cypher queries for graph databases, featuring integration with genai and FalkorDB. Use as a library in your Rust applications or deploy the all-in-one Docker solution with integrated FalkorDB database, web browser interface, text-to-cypher API, and Model Context Protocol (MCP) server support!

## ✨ What's New

**Library Support**: Now available as a Rust library! Use text-to-cypher directly in your Rust applications without the REST API overhead.

**All-in-One Docker Solution**: Our Docker image includes everything you need in a single container:

- 🗄️ **FalkorDB Database** (port 6379) - Full graph database with Redis protocol
- 🌐 **FalkorDB Web Interface** (port 3000) - Interactive graph browser and query builder  
- 🚀 **Text-to-Cypher API** (port 8080) - Natural language to Cypher conversion
- 🤖 **MCP Server** (port 3001) - AI assistant integration support

## Features

### Core Capabilities
- **Text to Cypher Translation**: Convert natural language queries to Cypher database queries using AI
- **Enhanced Schema Discovery**: Automatically discover and analyze graph database schemas with example values
- **Query Validation**: Built-in validation system to catch syntax errors before execution
- **Self-Healing Queries**: Automatic retry with error feedback when queries fail
- **Library & API Modes**: Use as a Rust library or REST API
- **RESTful API**: Clean HTTP API with comprehensive OpenAPI/Swagger documentation
- **MCP Server**: Model Context Protocol server for AI assistant integrations
- **Streaming Responses**: Real-time Server-Sent Events (SSE) streaming of query processing results

### Infrastructure
- **Rust Library**: Integrate directly into your Rust applications
- **Integrated FalkorDB**: Built-in FalkorDB graph database with web browser interface
- **All-in-One Docker Solution**: Complete stack in a single container - database, web UI, API, and MCP server
- **Multi-Platform Support**: Docker images available for both AMD64 and ARM64 architectures

### AI & Quality
- **AI Model Integration**: Powered by genai for natural language processing with support for multiple providers
- **Schema-Aware Generation**: Uses schema with example values for better query accuracy
- **Production Ready**: Comprehensive error handling, logging, and robust architecture
- **Environment Configuration**: Flexible configuration via `.env` file with fallback to request parameters

## Quick Start

### Using as a Rust Library

Add text-to-cypher to your `Cargo.toml`:

```toml
[dependencies]
# For library usage only (without REST server)
text-to-cypher = { version = "0.1", default-features = false }

# For full server capabilities
text-to-cypher = "0.1"
```

**Basic Example:**

```rust
use text_to_cypher::{TextToCypherClient, ChatRequest, ChatMessage, ChatRole};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a client
    let client = TextToCypherClient::new(
        "gpt-4o-mini",
        "your-api-key",
        "falkor://127.0.0.1:6379"
    );

    // Create a chat request
    let request = ChatRequest {
        messages: vec![
            ChatMessage {
                role: ChatRole::User,
                content: "Find all actors who appeared in movies released after 2020".to_string(),
            }
        ]
    };

    // Convert text to Cypher and execute
    let response = client.text_to_cypher("movies", request).await?;
    
    println!("Generated query: {}", response.cypher_query.unwrap());
    println!("Result: {}", response.cypher_result.unwrap());
    println!("Answer: {}", response.answer.unwrap());

    Ok(())
}
```

**More Examples:**

See the [library usage example](examples/library_usage.rs) for comprehensive examples including:
- Using the high-level `TextToCypherClient`
- Using core functions directly for more control
- Generating Cypher queries without execution

Run the example:
```bash
# Ensure FalkorDB is running
docker run -d -p 6379:6379 falkordb/falkordb:latest

# Set your API key
export OPENAI_API_KEY=your-key-here

# Run the example (library mode - no server dependencies)
cargo run --example library_usage --no-default-features
```

### Using Docker (Recommended for Server)

The easiest way to get started is using our all-in-one Docker image that includes FalkorDB database, web browser interface, text-to-cypher API, and MCP server:

```bash
# Run the complete stack with all services
docker run -p 6379:6379 -p 3000:3000 -p 8080:8080 -p 3001:3001 \
  -e DEFAULT_MODEL=gpt-4o-mini -e DEFAULT_KEY=your-api-key \
  falkordb/text-to-cypher:latest

# Or using environment file
docker run -p 6379:6379 -p 3000:3000 -p 8080:8080 -p 3001:3001 \
  --env-file .env \
  falkordb/text-to-cypher:latest

# Or mounting .env file for full MCP server functionality
docker run -p 6379:6379 -p 3000:3000 -p 8080:8080 -p 3001:3001 \
  -v $(pwd)/.env:/app/.env:ro \
  falkordb/text-to-cypher:latest

# Custom ports using environment variables
docker run -p 6379:6379 -p 3000:3000 -p 9090:9090 -p 4001:4001 \
  -e REST_PORT=9090 -e MCP_PORT=4001 \
  -e DEFAULT_MODEL=gpt-4o-mini -e DEFAULT_KEY=your-api-key \
  falkordb/text-to-cypher:latest
```

### Available Services

Once running, access the services at:

- **FalkorDB Database**: `localhost:6379` (Redis protocol)
- **FalkorDB Web Interface**: `http://localhost:3000` (Interactive graph database browser)
- **Text-to-Cypher API**: `http://localhost:8080` (REST API)
- **Swagger UI**: `http://localhost:8080/swagger-ui/` (API documentation)
- **MCP Server**: `localhost:3001` (Model Context Protocol server)
- **OpenAPI Spec**: `http://localhost:8080/api-doc/openapi.json`

### Local Development

If you prefer to run locally without Docker:

```bash
# Prerequisites: You'll need FalkorDB running separately
docker run -d -p 6379:6379 falkordb/falkordb:latest

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and run the text-to-cypher service
git clone https://github.com/FalkorDB/text-to-cypher.git
cd text-to-cypher
cp .env.example .env  # Edit with your configuration
cargo run
```

The local development setup requires:

- **FalkorDB instance**: Running on port 6379 (can be Docker or native)
- **Rust environment**: For building and running the text-to-cypher service

## API Documentation

The API includes comprehensive Swagger UI documentation available at `/swagger-ui/` when running the server.

## Configuration

The application supports flexible configuration via environment variables or `.env` file:

### Core Settings

- `DEFAULT_MODEL`: Default AI model to use (e.g., "openai:gpt-4")
- `DEFAULT_KEY`: Default API key for the AI service

### Port Configuration

- `REST_PORT`: REST API server port (default: 8080)
- `MCP_PORT`: MCP server port for AI assistant integrations (default: 3001)
  - The MCP server provides an SSE endpoint at `/sse` on this port

### Optional Settings

- `FALKORDB_CONNECTION`: FalkorDB connection string (default: "falkor://127.0.0.1:6379")

Create a `.env` file from the provided example:

```bash
cp .env.example .env
# Edit .env with your preferred default model and API key
```

### MCP Server Configuration

**Important**: The MCP server will only start if:

1. Both `DEFAULT_MODEL` and `DEFAULT_KEY` are configured
2. The `.env` file physically exists (not just environment variables)

For Docker deployments:

- Use `--env-file .env` or `-e` flags for HTTP server only (MCP server also starts if both MODEL and KEY are provided)
- Use `-v $(pwd)/.env:/app/.env:ro` to ensure MCP server starts with mounted `.env` file

## Architecture

The integrated Docker solution runs four concurrent services:

### FalkorDB Database (Port 6379)

- Graph database server with Redis protocol compatibility
- Stores and manages graph data structures
- Accessible via Redis clients and graph query languages

### FalkorDB Web Interface (Port 3000)

- Interactive web-based graph database browser
- Visual query builder and result visualization
- Database administration and monitoring tools
- Graph data exploration interface

### Text-to-Cypher HTTP API (Port 8080)

- Main REST API for text-to-cypher conversion
- Swagger UI documentation at `http://localhost:8080/swagger-ui/`
- OpenAPI specification at `http://localhost:8080/api-doc/openapi.json`
- Supports both streaming (SSE) and non-streaming responses

### MCP Server (Port 3001) - Conditional

- Model Context Protocol server for AI assistant integrations
- Provides `text_to_cypher` tool for natural language to Cypher conversion
- **Note**: MCP server only starts if both `DEFAULT_MODEL` and `DEFAULT_KEY` are configured

## Deployment Options

### Docker Deployment (Production)

The project provides an all-in-one Docker image that includes FalkorDB database, web browser interface, text-to-cypher API, and MCP server:

```bash
# Pull the latest image
docker pull ghcr.io/falkordb/text-to-cypher:latest

# Option 1: Complete stack with all services (recommended)
docker run -d \
  --name text-to-cypher-stack \
  -p 6379:6379 \
  -p 3000:3000 \
  -p 8080:8080 \
  -p 3001:3001 \
  -e DEFAULT_MODEL=gpt-4o-mini \
  -e DEFAULT_KEY=your-api-key \
  --restart unless-stopped \
  ghcr.io/falkordb/text-to-cypher:latest

# Option 2: Using environment file
docker run -d \
  --name text-to-cypher-stack \
  -p 6379:6379 \
  -p 3000:3000 \
  -p 8080:8080 \
  -p 3001:3001 \
  --env-file .env \
  --restart unless-stopped \
  ghcr.io/falkordb/text-to-cypher:latest

# Option 3: Mount .env file for full MCP functionality
docker run -d \
  --name text-to-cypher-stack \
  -p 6379:6379 \
  -p 3000:3000 \
  -p 8080:8080 \
  -p 3001:3001 \
  -v $(pwd)/.env:/app/.env:ro \
  --restart unless-stopped \
  ghcr.io/falkordb/text-to-cypher:latest

# View logs from all services
docker logs -f text-to-cypher-stack
```

### Docker Configuration Options

| Method | All Services | Use Case |
|--------|-------------|----------|
| `-e DEFAULT_MODEL=... -e DEFAULT_KEY=...` | ✅ | Environment-based config |
| `--env-file .env` | ✅ | File-based configuration |
| `-v $(pwd)/.env:/app/.env:ro` | ✅ | Mounted configuration file |

**Note**: All four services (FalkorDB database, web interface, text-to-cypher API, and MCP server) will start when both `DEFAULT_MODEL` and `DEFAULT_KEY` are configured, regardless of how the environment variables are provided.

### Service Ports

| Service | Port | Description |
|---------|------|-------------|
| FalkorDB Database | 6379 | Redis protocol access to graph database |
| FalkorDB Web Interface | 3000 | Interactive web browser for graph exploration |
| Text-to-Cypher HTTP API | 8080 | REST API with Swagger documentation |
| MCP Server | 3001 | Model Context Protocol server for AI integrations |

### Docker Features

- **All-in-One Solution**: Complete graph database stack in a single container
- **Multi-Platform**: Support for both AMD64 and ARM64 architectures
- **Minimal Size**: Optimized Alpine Linux base for efficient deployment
- **Production Ready**: Includes supervisord for process management and logging
- **Security**: Services run with appropriate user permissions

### Environment Variables

Configure the application using environment variables or `.env` file:

- `DEFAULT_MODEL`: Default AI model (e.g., "gpt-4o-mini", "anthropic:claude-3")
- `DEFAULT_KEY`: Default API key for the AI service
- `FALKOR_URL`: FalkorDB connection URL (default: "falkor://127.0.0.1:6379")

## MCP Server Usage

The MCP server provides a standardized interface for AI assistants to convert natural language questions into Cypher queries. This enables seamless integration with AI tools that support the Model Context Protocol.

### Using MCP Inspector

To test and interact with the MCP server, you can use the MCP Inspector:

1. **Start the text-to-cypher stack**:

```bash
docker run -p 6379:6379 -p 3000:3000 -p 8080:8080 -p 3001:3001 \
  -e DEFAULT_MODEL=gpt-4o-mini -e DEFAULT_KEY=your-api-key \
  ghcr.io/falkordb/text-to-cypher:latest
```

2. **Install MCP Inspector** (if not already installed):

```bash
npx -y @modelcontextprotocol/inspector
```

3. **Connect MCP Inspector to the server**:
   - Open MCP Inspector in your browser (typically `http://localhost:6274`)
   - Add a new server connection with these settings:
     - **Transport**: `stdio`
     - **Command**: `nc`
     - **Arguments**: `["localhost", "3001"]`

   Or if using a direct connection:
   - **Transport**: `sse`
   - **URL**: `http://localhost:3001/sse`

4. **Available Tools**:
   The MCP server exposes the following tool:

   #### `text_to_cypher`

   Converts natural language questions into Cypher queries for graph databases.

   **Parameters**:
   - `graph_name` (required): Name of the graph database to query
   - `question` (required): Natural language question to convert to Cypher

   **Example Usage in MCP Inspector**:

   ```json
   {
     "graph_name": "movies",
     "question": "Find all actors who appeared in movies released after 2020"
   }
   ```

5. **Example Workflow**:
   - Select the `text_to_cypher` tool in MCP Inspector
   - Fill in the parameters:
     - Graph name: `"social_network"`
     - Question: `"Who are the friends of John with more than 5 mutual connections?"`
   - Execute the tool
   - View the generated Cypher query and execution results

**Pro Tip**: You can also interact with the FalkorDB directly through the web interface at `http://localhost:3000` to create and explore graphs visually!

### Integration with AI Assistants

The MCP server enables AI assistants to:

- Convert natural language to Cypher queries
- Execute queries against FalkorDB graphs
- Provide structured responses with query results
- Handle complex graph database interactions seamlessly

### MCP Server Benefits

- **Standardized Interface**: Uses the Model Context Protocol for consistent AI tool integration
- **Streaming Support**: Real-time processing and response streaming
- **Error Handling**: Comprehensive error messages and validation
- **Documentation**: Auto-generated tool documentation with parameter descriptions and examples

## Getting Started

### Prerequisites

- **For Docker (Recommended)**: Docker installed on your system
- **For Local Development**:
  - Rust (latest stable version)
  - FalkorDB instance (can be run via Docker: `docker run -d -p 6379:6379 falkordb/falkordb:latest`)

### Running the Complete Stack

#### Using Docker (Recommended)

```bash
# Run the complete integrated stack
docker run -p 6379:6379 -p 3000:3000 -p 8080:8080 -p 3001:3001 \
  -e DEFAULT_MODEL=gpt-4o-mini -e DEFAULT_KEY=your-api-key \
  ghcr.io/falkordb/text-to-cypher:latest
```

#### Local Development

```bash
# Start FalkorDB separately
docker run -d -p 6379:6379 falkordb/falkordb:latest

# Run text-to-cypher service
cargo run
```

### Access the Services

Once running, access the services at:

- **FalkorDB Web Interface**: `http://localhost:3000` (Interactive graph browser)
- **Text-to-Cypher API**: `http://localhost:8080`
- **Swagger UI**: `http://localhost:8080/swagger-ui/`
- **OpenAPI spec**: `http://localhost:8080/api-doc/openapi.json`
- **FalkorDB Database**: `localhost:6379` (Redis protocol)
- **MCP Server**: `localhost:3001` (Model Context Protocol)

### Building for Production

#### Docker Build (Local)

```bash
# Build locally using the build script
./docker-build.sh

# Or build manually
docker build -t text-to-cypher:latest .
```

#### Using Pre-built Images

```bash
# Pull from GitHub Container Registry
docker pull ghcr.io/falkordb/text-to-cypher:latest

# Available tags: latest, v1.0.0, v0.1.0-beta.x, etc.
```

#### Native Build

```bash
cargo build --release
```

## Development

### Code Quality

The project maintains high code quality standards:

```bash
# Format code
cargo fmt

# Run linting (with pedantic and nursery clippy rules)
cargo clippy -- -W clippy::pedantic -W clippy::nursery -W clippy::cargo -A clippy::missing-errors-doc -A clippy::missing-panics-doc -A clippy::multiple-crate-versions

# Run tests
cargo test
```

### Project Structure

```
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
├── Dockerfile               # All-in-one Docker image with FalkorDB
├── supervisord.conf         # Process management configuration
├── entrypoint.sh           # Docker container startup script
├── .dockerignore           # Docker build context filtering
└── docker-build.sh         # Convenient Docker build script
```

## API Usage Examples

### Basic Text-to-Cypher Request

```bash
curl -X POST "http://localhost:8080/text_to_cypher" \
  -H "Content-Type: application/json" \
  -d '{
    "graph_name": "movies",
    "chat_request": {
      "messages": [
        {
          "role": "User",
          "content": "Find all actors who appeared in movies released after 2020"
        }
      ]
    },
    "model": "gpt-4o-mini",
    "key": "your-api-key"
  }'
```

### Using the FalkorDB Web Interface

1. **Access the web interface**: Open `http://localhost:3000` in your browser
2. **Connect to database**: The interface automatically connects to the local FalkorDB instance
3. **Create sample data**: Use the visual interface to create nodes and relationships
4. **Run queries**: Test Cypher queries directly in the web interface
5. **Export/Import**: Save your graph data or load sample datasets

### Using Server-Sent Events (SSE)

The API supports streaming responses for real-time progress updates:

```javascript
const eventSource = new EventSource('/text_to_cypher', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    graph_name: "social_network",
    chat_request: {
      messages: [{ role: "User", content: "Who are John's friends?" }]
    }
  })
});

eventSource.onmessage = (event) => {
  const progress = JSON.parse(event.data);
  console.log('Progress:', progress);
};
```

### Complete Workflow Example

```bash
# 1. Start the complete stack
docker run -p 6379:6379 -p 3000:3000 -p 8080:8080 -p 3001:3001 \
  -e DEFAULT_MODEL=gpt-4o-mini -e DEFAULT_KEY=your-api-key \
  ghcr.io/falkordb/text-to-cypher:latest

# 2. Create a graph using FalkorDB web interface (http://localhost:3000)
# Add some sample data: people, relationships, etc.

# 3. Query using natural language via the API
curl -X POST "http://localhost:8080/text_to_cypher" \
  -H "Content-Type: application/json" \
  -d '{
    "graph_name": "social_network",
    "chat_request": {
      "messages": [
        {
          "role": "User", 
          "content": "Find all people who have more than 3 friends"
        }
      ]
    },
    "model": "gpt-4o-mini",
    "key": "your-api-key"
  }'

# 4. Use MCP server for AI assistant integrations (port 3001)
# Connect your AI assistant to http://localhost:3001
```

## Publishing to crates.io

This library is designed to be published to [crates.io](https://crates.io/), making it easy to use in any Rust project.

### For Maintainers

To publish a new version:

1. **Update the version** in `Cargo.toml`:
   ```toml
   [package]
   version = "0.1.1"  # Increment as needed
   ```

2. **Ensure all tests pass**:
   ```bash
   cargo test
   ```

3. **Build and test both library and server modes**:
   ```bash
   # Test library-only mode
   cargo build --lib --no-default-features
   
   # Test with server features (default)
   cargo build
   ```

4. **Publish to crates.io**:
   ```bash
   cargo publish
   ```

### For Users

Once published, users can easily add text-to-cypher to their projects:

```toml
[dependencies]
# Library-only usage (no REST server)
text-to-cypher = { version = "0.1", default-features = false }

# With REST server capabilities
text-to-cypher = "0.1"
```

The library is published with:
- **default features**: Includes REST API server, Swagger UI, MCP server
- **no-default-features**: Core library only (schema discovery, query generation, execution)

## Troubleshooting

### Common Issues

**Services not starting**:

- Ensure all required ports (6379, 3000, 8080, 3001) are available
- Check that `DEFAULT_MODEL` and `DEFAULT_KEY` are properly configured
- View logs: `docker logs -f <container-name>`

**MCP Server not starting**:

- Verify both `DEFAULT_MODEL` and `DEFAULT_KEY` environment variables are set
- For local builds, ensure `.env` file exists in the working directory

**FalkorDB connection issues**:

- The integrated FalkorDB automatically starts with the container
- No external FalkorDB instance needed when using the Docker image
- Database is accessible at `localhost:6379` (Redis protocol)

**Web interface not accessible**:

- Ensure port 3000 is properly mapped: `-p 3000:3000`
- Try accessing `http://localhost:3000` directly
- Check firewall settings if running on a remote server

### Getting Help

- **API Documentation**: `http://localhost:8080/swagger-ui/`
- **Web Interface**: `http://localhost:3000` for graph exploration
- **Logs**: Use `docker logs -f <container-name>` to view all service logs
- **Issues**: Report problems at [GitHub Issues](https://github.com/FalkorDB/text-to-cypher/issues)

## Recent Improvements

This project implements best practices from current research and industry leaders:

### Query Quality & Reliability
- **Query Validation**: Automatic syntax and safety validation before execution
- **Self-Healing**: Failed queries are automatically regenerated with error feedback
- **Enhanced Schema**: Schema discovery now includes example values for better context

### Based on Research From
- [Neo4j Labs Text2Cypher](https://github.com/neo4j-labs/text2cypher) - Industry best practices
- [arXiv 2412.10064](https://arxiv.org/abs/2412.10064) - Text2Cypher academic research
- [GraphRAG](https://graphrag.com/reference/graphrag/text2cypher/) - Microsoft's approach
- [MDPI Research](https://www.mdpi.com/2076-3417/15/15/8206) - Reinforcement learning techniques

### Documentation
- [Improvements Guide](docs/IMPROVEMENTS.md) - Detailed technical improvements
- [Best Practices](docs/BEST_PRACTICES.md) - Usage guidelines and optimization tips

## License

This project is licensed under the MIT License.
