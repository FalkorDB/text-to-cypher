# Docker Multi-Architecture Examples

This directory contains Docker examples for building multi-architecture images of text-to-cypher using Docker Buildx.

## 🏗️ Multi-Architecture Support

All Dockerfiles automatically detect the target architecture and download the appropriate binary:
- **AMD64 (x86_64)**: Downloads `text-to-cypher-linux-x86_64-musl.tar.gz`
- **ARM64 (aarch64)**: Downloads `text-to-cypher-linux-aarch64-musl.tar.gz`

## 📋 Available Dockerfiles

### 1. `Dockerfile.simple` - Basic Single-Stage Build
Simple multi-architecture build that downloads and runs text-to-cypher.

```bash
docker buildx build --platform linux/amd64,linux/arm64 -f Dockerfile.simple -t text-to-cypher:simple .
```

### 2. `Dockerfile.multistage` - Production Multi-Stage Build
Optimized production build with multi-stage architecture and proper security.

```bash
docker buildx build --platform linux/amd64,linux/arm64 -f Dockerfile.multistage -t text-to-cypher:production .
```

### 3. `Dockerfile.versioned` - Parameterized Version Build
Allows specifying the version at build time.

```bash
docker buildx build --platform linux/amd64,linux/arm64 -f Dockerfile.versioned \
  --build-arg VERSION=v1.0.0 \
  -t text-to-cypher:v1.0.0 .
```

## 🚀 Quick Start

### Prerequisites
- Docker with Buildx enabled  
- Internet connection to download releases from GitHub

### Build All Examples (Local)
```bash
./build-examples.sh v1.0.0
```

### Build and Push to Registry
```bash
./build-and-push.sh v1.0.0 ghcr.io/your-username/text-to-cypher
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
