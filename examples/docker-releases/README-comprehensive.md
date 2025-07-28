# Docker Examples with GitHub Releases

This directory contains comprehensive Docker examples that use the GitHub release binaries of text-to-cypher.

## 🚀 Quick Start

```bash
# 1. Build all Docker examples
./build-examples.sh

# 2. Run examples with different configurations
./run-examples.sh [version] [api-key] [model]

# Example with your API key:
./run-examples.sh v0.1.0-alpha.1 "sk-your-key" "gpt-4o-mini"
```

## 📦 Available Examples

### Simple Container (`Dockerfile.simple`)
- Single-stage build for quick testing
- Minimal configuration
- Good for development and testing

### Production Container (`Dockerfile.multistage`)
- Multi-stage build for smaller image size
- Non-root user for security
- Optimized for production deployment

### Versioned Container (`Dockerfile.versioned`)
- Parameterized version selection
- Flexible GitHub release targeting
- Build-time version configuration

### Complete Stack (`docker-compose.yml`)
- Text-to-Cypher API server
- FalkorDB graph database
- Pre-configured networking
- Environment file support

## 🔧 Configuration

### Environment Variables

| Variable | Description | Required | Default |
|----------|-------------|----------|---------|
| `DEFAULT_MODEL` | OpenAI model (e.g., gpt-4o-mini) | Yes* | - |
| `DEFAULT_KEY` | OpenAI API key | Yes* | - |
| `RUST_LOG` | Log level (trace, debug, info, warn, error) | No | warn |

*Required for MCP server functionality. The HTTP API works without these.

### Port Configuration

| Port | Service | Description |
|------|---------|-------------|
| `8080` | HTTP API | REST API with Swagger UI |
| `3001` | MCP Server | Model Context Protocol server |
| `6379` | FalkorDB | Graph database (docker-compose only) |

## 🏃 Usage Examples

### 1. Simple Development Setup

```bash
# Build simple container
docker build -f Dockerfile.simple -t text-to-cypher:simple .

# Run with your OpenAI credentials
docker run -d \
  --name text-to-cypher-dev \
  -p 8080:8080 \
  -p 3001:3001 \
  -e DEFAULT_MODEL="gpt-4o-mini" \
  -e DEFAULT_KEY="your-openai-api-key" \
  -e RUST_LOG=debug \
  text-to-cypher:simple

# Access Swagger UI
open http://localhost:8080/swagger-ui/
```

### 2. Production Deployment

```bash
# Build production container
docker build -f Dockerfile.multistage -t text-to-cypher:prod .

# Run with production settings
docker run -d \
  --name text-to-cypher-prod \
  -p 8080:8080 \
  -p 3001:3001 \
  -e DEFAULT_MODEL="gpt-4o" \
  -e DEFAULT_KEY="your-openai-api-key" \
  -e RUST_LOG=info \
  --restart unless-stopped \
  text-to-cypher:prod
```

### 3. Complete Stack with Database

```bash
# Create environment file
cat > .env << EOF
OPENAI_API_KEY=your-openai-api-key
DEFAULT_MODEL=gpt-4o-mini
EOF

# Start the complete stack
docker-compose up -d

# Check status
docker-compose ps

# View logs
docker-compose logs text-to-cypher
```

### 4. Custom Version Build

```bash
# Build with specific version
docker build \
  -f Dockerfile.versioned \
  --build-arg RELEASE_VERSION=v0.1.0-alpha.1 \
  -t text-to-cypher:v0.1.0-alpha.1 \
  .
```

## 🧪 Testing & Validation

### Health Checks

```bash
# Check if API is running
curl http://localhost:8080/swagger-ui/

# Test with environment info (if available)
curl http://localhost:8080/api/environment

# Check MCP server (if enabled)
# The MCP server runs on port 3001 and uses Server-Sent Events
```

### Container Logs

```bash
# View real-time logs
docker logs -f text-to-cypher-dev

# Check startup logs
docker logs text-to-cypher-dev 2>&1 | grep -E "(Starting|MCP|HTTP)"
```

### Container Health

```bash
# Inspect container
docker inspect text-to-cypher-dev

# Check resource usage
docker stats text-to-cypher-dev
```

## 🐛 Troubleshooting

### Common Issues

1. **Port Already in Use**
   ```bash
   # Find what's using the port
   lsof -i :8080
   
   # Use different ports
   docker run -p 8081:8080 -p 3002:3001 ...
   ```

2. **MCP Server Not Starting**
   - Ensure both `DEFAULT_MODEL` and `DEFAULT_KEY` are set
   - Check logs for API key validation errors
   - Verify model name is supported by OpenAI

3. **Template Errors**
   - Templates are embedded in the binary from GitHub releases
   - If using local builds, ensure templates are copied correctly

4. **Network Access Issues**
   ```bash
   # Check if container can reach the internet
   docker exec text-to-cypher-dev curl -I https://api.openai.com
   ```

### Debugging Commands

```bash
# Interactive shell in container
docker exec -it text-to-cypher-dev /bin/sh

# Check binary version
docker exec text-to-cypher-dev text-to-cypher --version

# List templates (if debugging)
docker exec text-to-cypher-dev find /app -name "*.tera" -o -name "*.html"
```

## 🧹 Cleanup

```bash
# Stop and remove all containers
docker stop text-to-cypher-dev text-to-cypher-prod
docker rm text-to-cypher-dev text-to-cypher-prod

# Stop docker-compose stack
docker-compose down

# Remove images
docker rmi text-to-cypher:simple text-to-cypher:prod

# Complete cleanup (removes all text-to-cypher images)
docker rmi $(docker images -q text-to-cypher)
```

## 📚 Integration Examples

For more detailed integration examples and production deployment guides, see:

- [INTEGRATION.md](../../INTEGRATION.md) - Complete integration guide
- [RELEASES.md](../../RELEASES.md) - Version management and releases

## 🔗 Related Resources

- [GitHub Releases](https://github.com/FalkorDB/text-to-cypher/releases)
- [Swagger API Documentation](http://localhost:8080/swagger-ui/) (when running)
- [Model Context Protocol](https://modelcontextprotocol.io/)

## 🎯 Example Use Cases

### Development & Testing
Use the simple container for quick development cycles and API testing.

### Production Deployment
Use the multi-stage container with proper security settings for production.

### Complete Development Stack
Use docker-compose for full-stack development with database integration.

### CI/CD Integration
Use versioned containers in your CI/CD pipelines with specific release versions.
