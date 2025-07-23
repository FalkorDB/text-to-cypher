# Example Docker Usage with GitHub Release v0.1.0-alpha.1

This directory contains examples of how to use the `text-to-cypher` binary from GitHub releases in Docker containers.

## 🚀 Quick Start

### Simple Download Example

```dockerfile
FROM alpine:latest

# Install dependencies
RUN apk add --no-cache ca-certificates wget tar

# Download the specific release
RUN wget https://github.com/barakb/text-to-cypher/releases/download/v0.1.0-alpha.1/packages/text-to-cypher-linux-x86_64-musl.tar.gz && \
    tar -xzf text-to-cypher-linux-x86_64-musl.tar.gz && \
    mv text-to-cypher-linux-x86_64-musl /usr/local/bin/text-to-cypher && \
    mv templates /usr/local/share/text-to-cypher-templates && \
    rm text-to-cypher-linux-x86_64-musl.tar.gz

# Set environment variables
ENV TEMPLATES_DIR=/usr/local/share/text-to-cypher-templates

# Expose ports
EXPOSE 8080 3001

# Run the application
CMD ["text-to-cypher"]
```

### Build and Run

```bash
# Build the Docker image
docker build -t text-to-cypher:v0.1.0-alpha.1 .

# Run with environment variables
docker run -d \
  --name text-to-cypher \
  -p 8080:8080 \
  -p 3001:3001 \
  -e DEFAULT_MODEL="gpt-4o-mini" \
  -e DEFAULT_KEY="your-api-key-here" \
  text-to-cypher:v0.1.0-alpha.1

# Check if it's running
curl http://localhost:8080/swagger-ui/
```

## 📁 Example Files

- `Dockerfile.simple` - Basic single-stage build
- `Dockerfile.multistage` - Production-ready multi-stage build
- `Dockerfile.versioned` - Parameterized version selection
- `docker-compose.yml` - Complete stack with database
- `README.md` - This file

## 🔧 Configuration Options

All examples support these environment variables:
- `DEFAULT_MODEL` - AI model to use (e.g., "gpt-4o-mini")
- `DEFAULT_KEY` - API key for the AI service
- `TEMPLATES_DIR` - Path to templates (pre-configured in images)

## 🐳 Production Usage

For production, use the multi-stage build example which includes:
- Health checks
- Non-root user
- Minimal attack surface
- Proper logging configuration
