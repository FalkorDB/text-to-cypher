# Text to Cypher

[![build](https://github.com/barakb/text-to-cypher/actions/workflows/build.yml/badge.svg)](https://github.com/barakb/text-to-cypher/actions/workflows/build.yml)

A high-performance Rust-based API service that translates natural language text to Cypher queries for graph databases, featuring integration with genai and FalkorDB. Now with Model Context Protocol (MCP) server support and Docker deployment!

## Features

- **Text to Cypher Translation**: Convert natural language queries to Cypher database queries using AI
- **Graph Schema Discovery**: Automatically discover and analyze graph database schemas
- **RESTful API**: Clean HTTP API with comprehensive OpenAPI/Swagger documentation
- **MCP Server**: Model Context Protocol server for AI assistant integrations
- **Streaming Responses**: Real-time Server-Sent Events (SSE) streaming of query processing results
- **FalkorDB Integration**: Native support for FalkorDB graph database
- **AI Model Integration**: Powered by genai for natural language processing with support for multiple providers
- **Environment Configuration**: Flexible configuration via `.env` file with fallback to request parameters
- **Docker Support**: Minimal Alpine-based Docker container for easy deployment
- **Production Ready**: Comprehensive error handling, logging, and robust architecture

## Quick Start

### Using Docker (Recommended)

The easiest way to get started is using Docker:

```bash
# Clone the repository
git clone https://github.com/barakb/text-to-cypher.git
cd text-to-cypher

# Build the Docker image
./docker-build.sh

# Option 1: Full functionality with both servers (using environment variables)
docker run -p 8080:8080 -p 8081:8081 -e DEFAULT_MODEL=openai:gpt-4 -e DEFAULT_KEY=your-api-key text-to-cypher:latest

# Option 2: Full functionality with both servers (using --env-file)
docker run -p 8080:8080 -p 8081:8081 --env-file .env text-to-cypher:latest

# Option 3: Full functionality with both servers (mount .env file)
docker run -p 8080:8080 -p 8081:8081 -v $(pwd)/.env:/app/.env:ro text-to-cypher:latest
```

### Local Development

If you prefer to run locally:

```bash
# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and run
git clone https://github.com/barakb/text-to-cypher.git
cd text-to-cypher
cp .env.example .env  # Edit with your configuration
cargo run
```

### Access the API

Once running, access the service at:
- **API Base**: `http://localhost:8080`
- **Swagger UI**: `http://localhost:8080/swagger-ui/`
- **OpenAPI Spec**: `http://localhost:8080/api-doc/openapi.json`

## API Documentation

The API includes comprehensive Swagger UI documentation available at `/swagger-ui/` when running the server.

## Configuration

The application supports flexible configuration via environment variables or `.env` file:

- `DEFAULT_MODEL`: Default AI model to use (e.g., "openai:gpt-4")
- `DEFAULT_KEY`: Default API key for the AI service

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
- Use `--env-file .env` for HTTP server only
- Use `-v $(pwd)/.env:/app/.env:ro` to enable MCP server

## Architecture

The application runs two concurrent servers:

### HTTP API Server (Port 8080)
- Main REST API for text-to-cypher conversion
- Swagger UI documentation at `http://localhost:8080/swagger-ui/`
- OpenAPI specification at `http://localhost:8080/api-doc/openapi.json`
- Supports both streaming (SSE) and non-streaming responses

### MCP Server (Port 8081) - Conditional
- Model Context Protocol server for AI assistant integrations
- Provides `text_to_cypher` tool for natural language to Cypher conversion
- **Note**: MCP server only starts if `.env` file exists with both `DEFAULT_MODEL` and `DEFAULT_KEY` configured

## Deployment Options

### Docker Deployment (Production)

The project includes a minimal Alpine-based Dockerfile for production deployment:

```bash
# Build the image
./docker-build.sh

# Option 1: HTTP server only (using --env-file, MCP server disabled)
docker run -d \
  --name text-to-cypher \
  -p 8080:8080 \
  --env-file .env \
  --restart unless-stopped \
  text-to-cypher:latest

# Option 2: Full functionality with MCP server (using volume mount)
docker run -d \
  --name text-to-cypher \
  -p 8080:8080 \
  -p 8081:8081 \
  -v $(pwd)/.env:/app/.env:ro \
  --restart unless-stopped \
  text-to-cypher:latest

# Option 3: Using environment variables (HTTP server only)
docker run -d \
  --name text-to-cypher \
  -p 8080:8080 \
  -e DEFAULT_MODEL=openai:gpt-4 \
  -e DEFAULT_KEY=your-api-key \
  --restart unless-stopped \
  text-to-cypher:latest

# View logs
docker logs -f text-to-cypher
```

### Docker Configuration Options

| Method | HTTP Server | MCP Server | Use Case |
|--------|-------------|------------|----------|
| `--env-file .env` | ✅ | ✅ | Environment-based config |
| `-v $(pwd)/.env:/app/.env:ro` | ✅ | ✅ | Full functionality with MCP |
| `-e DEFAULT_MODEL=... -e DEFAULT_KEY=...` | ✅ | ✅ | Environment-based config |

**Note**: The MCP server will start whenever both `DEFAULT_MODEL` and `DEFAULT_KEY` are configured, regardless of how the environment variables are provided (via `--env-file`, `-e` flags, or mounted `.env` file).

### Docker Features

- **Minimal Size**: Alpine Linux base for small image footprint
- **Multi-stage Build**: Efficient build process with dependency caching
- **Security**: Runs as non-root user
- **Production Ready**: Includes only the executable and required templates

### Environment Variables

Configure the application using environment variables or `.env` file:

- `DEFAULT_MODEL`: Default AI model (e.g., "openai:gpt-4", "anthropic:claude-3")
- `DEFAULT_KEY`: Default API key for the AI service
- `FALKOR_URL`: FalkorDB connection URL (default: "falkor://127.0.0.1:6379")

## MCP Server Usage

The MCP server provides a standardized interface for AI assistants to convert natural language questions into Cypher queries. This enables seamless integration with AI tools that support the Model Context Protocol.

### Using MCP Inspector

To test and interact with the MCP server, you can use the MCP Inspector:

1. **Install MCP Inspector** (if not already installed):
```bash
npx @modelcontextprotocol/inspector
```

2. **Start the text-to-cypher application**:
```bash
cargo run
```

3. **Connect MCP Inspector to the server**:
   - Open MCP Inspector in your browser (typically `http://localhost:6274`)
   - Add a new server connection with these settings:
     - **Transport**: `stdio`
     - **Command**: `nc`
     - **Arguments**: `["localhost", "8081"]`
   
   Or if using a direct connection:
   - **Transport**: `sse`
   - **URL**: `http://localhost:8081/sse`

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

- **For Docker**: Docker installed on your system
- **For Local Development**:
  - Rust (latest stable version)
  - FalkorDB instance running locally or accessible remotely

### Running the Service

#### Using Docker (Recommended)
```bash
# Quick start with Docker
./docker-build.sh
docker run -p 8080:8080 --env-file .env text-to-cypher:latest
```

#### Local Development
```bash
cargo run
```

The service will start at `http://localhost:8080`

- API endpoints: `http://localhost:8080`
- Swagger UI: `http://localhost:8080/swagger-ui/`
- OpenAPI spec: `http://localhost:8080/api-doc/openapi.json`

### Building for Production

#### Docker Build
```bash
./docker-build.sh
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
├── Dockerfile               # Multi-stage Alpine-based build
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
    "model": "openai:gpt-4",
    "key": "your-api-key"
  }'
```

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

## License

This project is licensed under the MIT License.
