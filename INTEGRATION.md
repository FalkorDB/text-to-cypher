# Using Text-to-Cypher in Your Projects

This guide shows you how to integrate `text-to-cypher` into your projects and Docker images.

## 🚀 Quick Installation

### One-line Install
```bash
curl -sSL https://github.com/FalkorDB/text-to-cypher/releases/latest/download/install.sh | bash
```

### Install System-wide
```bash
curl -sSL https://github.com/FalkorDB/text-to-cypher/releases/latest/download/install.sh | bash -s -- --install
```

## 🐳 Docker Integration

### Download Binary in Dockerfile
```dockerfile
FROM alpine:latest

# Download the latest binary with templates
ARG TEXT_TO_CYPHER_VERSION=latest
RUN apk add --no-cache ca-certificates wget tar && \
    if [ "$TEXT_TO_CYPHER_VERSION" = "latest" ]; then \
        TEXT_TO_CYPHER_VERSION=$(wget -qO- https://api.github.com/repos/FalkorDB/text-to-cypher/releases/latest | grep '"tag_name":' | sed -E 's/.*"([^"]+)".*/\1/'); \
    fi && \
    wget https://github.com/FalkorDB/text-to-cypher/releases/download/$TEXT_TO_CYPHER_VERSION/packages/text-to-cypher-linux-x86_64-musl.tar.gz -O text-to-cypher.tar.gz && \
    tar -xzf text-to-cypher.tar.gz && \
    mv text-to-cypher-linux-x86_64-musl /usr/local/bin/text-to-cypher && \
    mv templates /usr/local/share/text-to-cypher-templates && \
    rm text-to-cypher.tar.gz && \
    apk del wget tar

ENV TEMPLATES_DIR=/usr/local/share/text-to-cypher-templates
CMD ["text-to-cypher"]
```

### Multi-stage Build with Binary Download
```dockerfile
# Download stage
FROM alpine:latest AS downloader
RUN apk add --no-cache wget jq tar
ARG TEXT_TO_CYPHER_VERSION=latest
RUN if [ "$TEXT_TO_CYPHER_VERSION" = "latest" ]; then \
        TEXT_TO_CYPHER_VERSION=$(wget -qO- https://api.github.com/repos/FalkorDB/text-to-cypher/releases/latest | jq -r '.tag_name'); \
    fi && \
    wget https://github.com/FalkorDB/text-to-cypher/releases/download/$TEXT_TO_CYPHER_VERSION/packages/text-to-cypher-linux-x86_64-musl.tar.gz -O /tmp/text-to-cypher.tar.gz && \
    cd /tmp && tar -xzf text-to-cypher.tar.gz

# Runtime stage
FROM alpine:latest
RUN apk add --no-cache ca-certificates
COPY --from=downloader /tmp/text-to-cypher-linux-x86_64-musl /usr/local/bin/text-to-cypher
COPY --from=downloader /tmp/templates /usr/local/share/text-to-cypher-templates
ENV TEMPLATES_DIR=/usr/local/share/text-to-cypher-templates
EXPOSE 8080 3001
CMD ["text-to-cypher"]
```

## 📋 Docker Compose Integration

```yaml
version: '3.8'
services:
  text-to-cypher:
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "8080:8080"  # HTTP API
      - "3001:3001"  # MCP Server
    environment:
      - DEFAULT_MODEL=your-model
      - DEFAULT_KEY=your-api-key
    volumes:
      - ./templates:/app/templates:ro  # Optional: custom templates
```

## 🔧 GitHub Actions Integration

### Download and Use in CI/CD
```yaml
name: Use Text-to-Cypher
on: [push]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - name: Download text-to-cypher
        run: |
          curl -sSL https://github.com/FalkorDB/text-to-cypher/releases/latest/download/install.sh | bash
          
      - name: Use text-to-cypher
        run: |
          ./text-to-cypher --help
```

### Use in Docker Actions
```yaml
name: Use Text-to-Cypher Docker
on: [push]

jobs:
  test:
    runs-on: ubuntu-latest
    steps:
      - name: Checkout
        uses: actions/checkout@v4
        
      - name: Build and run text-to-cypher
        run: |
          # Download binary with templates
          curl -sSL https://github.com/FalkorDB/text-to-cypher/releases/latest/download/install.sh | bash
          ./text-to-cypher --help
```

## 🏗️ Build from Source Integration

### Cargo Integration
If you want to build from source in your Dockerfile:

```dockerfile
FROM rust:1.85-alpine AS builder

# Install build dependencies
RUN apk add --no-cache musl-dev git

# Clone and build text-to-cypher
RUN git clone https://github.com/FalkorDB/text-to-cypher.git /src
WORKDIR /src
RUN cargo build --release

# Runtime stage
FROM alpine:latest
RUN apk add --no-cache ca-certificates
COPY --from=builder /src/target/release/text-to-cypher /usr/local/bin/
COPY --from=builder /src/templates /app/templates
WORKDIR /app
EXPOSE 8080 3001
CMD ["text-to-cypher"]
```

## 📦 Version Pinning

### Pin to Specific Version
```bash
# Download specific version with templates
VERSION="v2025.01.15-abc1234"
wget https://github.com/FalkorDB/text-to-cypher/releases/download/$VERSION/packages/text-to-cypher-linux-x86_64.tar.gz
tar -xzf text-to-cypher-linux-x86_64.tar.gz
```

### In Dockerfile
```dockerfile
ARG TEXT_TO_CYPHER_VERSION=v2025.01.15-abc1234
RUN wget https://github.com/FalkorDB/text-to-cypher/releases/download/$TEXT_TO_CYPHER_VERSION/packages/text-to-cypher-linux-x86_64-musl.tar.gz && \
    tar -xzf text-to-cypher-linux-x86_64-musl.tar.gz && \
    mv text-to-cypher-linux-x86_64-musl /usr/local/bin/text-to-cypher && \
    mv templates /usr/local/share/text-to-cypher-templates
```

## 🔍 Available Packages

Each release includes complete packages with templates:

### Complete Packages (Binary + Templates)
- `packages/text-to-cypher-linux-x86_64.tar.gz` - Complete package for Linux x86_64
- `packages/text-to-cypher-linux-x86_64-musl.tar.gz` - Complete static package (recommended for Docker)
- `packages/text-to-cypher-linux-aarch64.tar.gz` - Complete package for ARM64

### Additional Files
- `templates/` - Template files directory (also included in packages)
- `checksums.txt` - SHA256 checksums for verification
- `install.sh` - Smart installation script

## 🛡️ Security Verification

```bash
# Download and verify checksums
wget https://github.com/FalkorDB/text-to-cypher/releases/latest/download/packages/text-to-cypher-linux-x86_64.tar.gz
wget https://github.com/FalkorDB/text-to-cypher/releases/latest/download/checksums.txt
sha256sum -c checksums.txt --ignore-missing
```

## 📚 Examples

See the [examples](examples/) directory for complete integration examples.

## 🆘 Support

- 📖 Documentation: [README.md](README.md)
- 🐛 Issues: [GitHub Issues](https://github.com/FalkorDB/text-to-cypher/issues)
- 💬 Discussions: [GitHub Discussions](https://github.com/FalkorDB/text-to-cypher/discussions)
