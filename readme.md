# Text to Cypher

[![build](https://github.com/barakb/text-to-cypher/actions/workflows/build.yml/badge.svg)](https://github.com/barakb/text-to-cypher/actions/workflows/build.yml)

A Rust-based API service that translates natural language text to Cypher queries for graph databases, featuring integration with genai and FalkorDB. Now with Model Context Protocol (MCP) server support!

## Features

- **Text to Cypher Translation**: Convert natural language queries to Cypher database queries
- **Graph Schema Discovery**: Automatically discover and analyze graph database schemas
- **RESTful API**: Clean HTTP API with OpenAPI/Swagger documentation
- **MCP Server**: Model Context Protocol server for AI assistant integrations
- **Streaming Responses**: Real-time streaming of query processing results
- **FalkorDB Integration**: Native support for FalkorDB graph database
- **AI Model Integration**: Powered by genai for natural language processing
- **Environment Configuration**: Support for `.env` file configuration

## API Documentation

The API includes Swagger UI documentation available at `/swagger-ui/` when running the server.

## Configuration

The application supports configuration via environment variables or `.env` file:

- `DEFAULT_MODEL`: Default AI model to use (e.g., "openai:gpt-4")
- `DEFAULT_KEY`: Default API key for the AI service

Create a `.env` file from the provided example:
```bash
cp .env.example .env
# Edit .env with your preferred default model and API key
```

## Architecture

The application runs two concurrent servers:

### HTTP API Server (Port 8080)
- Main REST API for text-to-cypher conversion
- Swagger UI documentation at `http://localhost:8080/swagger-ui/`
- Health check endpoint at `http://localhost:8080/health`

### MCP Server (Port 8081)
- Model Context Protocol server for AI assistant integrations
- Provides `text_to_cypher` tool for natural language to Cypher conversion

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

- Rust (latest stable version)
- FalkorDB instance

### Running the Service

```bash
cargo run
```

The service will start at `http://localhost:8080`

- API endpoints: `http://localhost:8080`
- Swagger UI: `http://localhost:8080/swagger-ui/`
- OpenAPI spec: `http://localhost:8080/api-doc/openapi.json`

### Building for Production

```bash
cargo build --release
```

## Development

### Running Tests

```bash
cargo test
```

### Code Formatting

```bash
cargo fmt
```

### Linting

```bash
cargo clippy
```

## License

This project is licensed under the MIT License.
