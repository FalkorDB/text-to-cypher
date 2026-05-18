# Docker Release Process

This document explains how the automated Docker image build and release process works.

## Overview

When a new release is published on GitHub, the Docker Release Build workflow automatically:

1. **Builds multi-platform Docker images** for both `linux/amd64` and `linux/arm64`
2. **Uses the published release binaries** instead of rebuilding from source
3. **Pins the baked-in FalkorDB Cypher skills** to a reproducible `FalkorDB/skills` commit
4. **Pushes images to Docker Hub** with proper versioning
5. **Verifies the images work** on both platforms
6. **Provides deployment instructions** in the workflow summary

## Triggered Events

The workflow is triggered when:
- A new release is **published** (not just created as draft)
- The release can be either stable or pre-release

## Generated Images

For each release, the following Docker images are created:

### Docker Hub
- `docker.io/falkordb/text-to-cypher:v1.0.0` (release tag)
- `docker.io/falkordb/text-to-cypher:latest`
- `falkordb/text-to-cypher:v1.0.0` (Docker Hub compatibility alias)
- `falkordb/text-to-cypher:latest` (Docker Hub compatibility alias)

## Architecture Support

All images support both:
- **linux/amd64** (Intel/AMD x64)
- **linux/arm64** (Apple Silicon, ARM servers)

Docker automatically pulls the correct architecture based on your system.

## How It Works

### 1. Release Detection
```yaml
on:
  release:
    types: [published]
```

### 2. Binary Download Strategy
The Dockerfile downloads pre-built binaries from the GitHub release:
```dockerfile
ARG VERSION=v0.1.0-alpha.1
wget -O /tmp/text-to-cypher.tar.gz \
  "https://github.com/FalkorDB/text-to-cypher/releases/download/${VERSION}/text-to-cypher-linux-${RUST_ARCH}.tar.gz"
```

### 3. Multi-Platform Build
Uses Docker Buildx with the enhanced `docker-build.sh` script:
```bash
./docker-build.sh \
  --version "v1.0.0" \
  --skills-ref "172978316e493c48ca352a0be6fb668a9f728855" \
  --platforms "linux/amd64,linux/arm64" \
  --registry "docker.io/falkordb" \
  --push
```

### 4. Image Verification
Each platform image is tested to ensure:
- Image can be pulled successfully
- Binary exists and is executable
- Basic functionality works

## Manual Usage

You can also use the `docker-build.sh` script manually:

### Local Development Build
```bash
./docker-build.sh --version v1.0.0 --local
```

### Push to Custom Registry
```bash
./docker-build.sh \
  --version v1.0.0 \
  --skills-ref <falkordb-skills-commit-sha> \
  --registry my-registry.com/my-org \
  --push
```

### Single Platform Build
```bash
./docker-build.sh \
  --version v1.0.0 \
  --skills-ref <falkordb-skills-commit-sha> \
  --platforms linux/amd64 \
  --push
```

## Configuration

The workflow requires:
- **DOCKER_USERNAME / DOCKER_PASSWORD**: Docker Hub credentials used by the release workflow
- **Docker Buildx**: Automatically set up in the workflow
- **Release binaries**: Must exist in the GitHub release assets
- **CYPHER_SKILLS_REF**: Pinned `FalkorDB/skills` commit SHA baked into release images

## Troubleshooting

### Image Not Found
If you get "image not found" errors:
1. Check that the release was **published** (not draft)
2. Verify the release contains the required binary assets
3. Wait a few minutes for the build to complete

### Platform Issues
If you need a specific platform:
```bash
docker pull --platform linux/amd64 docker.io/falkordb/text-to-cypher:v1.0.0
```

### Build Failures
Check the GitHub Actions logs:
1. Go to the repository's "Actions" tab
2. Find the "Docker Release Build" workflow
3. Check the failed job logs

## Services and Ports

The Docker image includes both FalkorDB and text-to-cypher:

- **Port 6379**: FalkorDB (Redis protocol)
- **Port 3000**: FalkorDB web interface
- **Port 8080**: text-to-cypher HTTP API
- **Port 3001**: text-to-cypher MCP server

## Quick Start

```bash
# Run the latest release
docker run -p 6379:6379 -p 3000:3000 -p 8080:8080 -p 3001:3001 \
  -e DEFAULT_MODEL=gpt-4o-mini -e DEFAULT_KEY=your-key \
  docker.io/falkordb/text-to-cypher:latest

# Run a specific version
docker run -p 6379:6379 -p 3000:3000 -p 8080:8080 -p 3001:3001 \
  -e DEFAULT_MODEL=gpt-4o-mini -e DEFAULT_KEY=your-key \
  docker.io/falkordb/text-to-cypher:v1.0.0
```
