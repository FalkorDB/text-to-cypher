#!/bin/bash

# Build the Docker image
echo "Building Docker image..."
docker build -t text-to-cypher:latest .

# Check if build was successful
if [ $? -eq 0 ]; then
    echo "✅ Docker image built successfully!"
    echo ""
    echo "To run the container:"
    echo "  docker run -p 8080:8080 text-to-cypher:latest"
    echo ""
    echo "To run with environment variables (enables MCP server):"
    echo "  docker run -p 8080:8080 -p 8081:8081 -e DEFAULT_MODEL=your-model -e DEFAULT_KEY=your-key text-to-cypher:latest"
    echo ""
    echo "To run with .env file mounted (also enables MCP server):"
    echo "  docker run -p 8080:8080 -p 8081:8081 -v \$(pwd)/.env:/app/.env:ro text-to-cypher:latest"
    echo ""
    echo "Note: MCP server will start whenever both DEFAULT_MODEL and DEFAULT_KEY are set."
    echo "      You can provide them via environment variables (-e) or .env file mounting (-v)."
else
    echo "❌ Docker build failed!"
    exit 1
fi
