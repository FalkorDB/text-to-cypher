# Text to Cypher

[![build](https://github.com/barakb/text-to-cypher/actions/workflows/build.yml/badge.svg)](https://github.com/barakb/text-to-cypher/actions/workflows/build.yml)

A Rust-based API service that translates natural language text to Cypher queries for graph databases, featuring integration with genai and FalkorDB.

## Features

- **Text to Cypher Translation**: Convert natural language queries to Cypher database queries
- **Graph Schema Discovery**: Automatically discover and analyze graph database schemas
- **RESTful API**: Clean HTTP API with OpenAPI/Swagger documentation
- **Streaming Responses**: Real-time streaming of query processing results
- **FalkorDB Integration**: Native support for FalkorDB graph database
- **AI Model Integration**: Powered by genai for natural language processing

## API Documentation

The API includes Swagger UI documentation available at `/swagger-ui/` when running the server.

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
