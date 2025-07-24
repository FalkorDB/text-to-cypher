#!/bin/bash
# build-examples.sh - Script to build all Docker examples

set -e

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

VERSION=${1:-v2025.07.23-d1092dc}

echo -e "${YELLOW}Building Docker examples for text-to-cypher $VERSION${NC}"

# Build simple version
echo -e "${GREEN}Building simple Dockerfile...${NC}"
docker build -f Dockerfile.simple -t text-to-cypher:simple-$VERSION .

# Build multi-stage version
echo -e "${GREEN}Building multi-stage Dockerfile...${NC}"
docker build -f Dockerfile.multistage -t text-to-cypher:production-$VERSION .

# Build versioned with different architectures
echo -e "${GREEN}Building versioned Dockerfile for x86_64...${NC}"
docker build -f Dockerfile.versioned \
  --build-arg VERSION=$VERSION \
  --build-arg ARCH=x86_64-musl \
  -t text-to-cypher:$VERSION-x86_64 .

echo -e "${GREEN}Building versioned Dockerfile for regular x86_64...${NC}"
docker build -f Dockerfile.versioned \
  --build-arg VERSION=$VERSION \
  --build-arg ARCH=x86_64 \
  -t text-to-cypher:$VERSION-x86_64-glibc .

echo -e "${GREEN}Building versioned Dockerfile for ARM64 (musl)...${NC}"
docker build -f Dockerfile.versioned \
  --build-arg VERSION=$VERSION \
  --build-arg ARCH=aarch64-musl \
  -t text-to-cypher:$VERSION-aarch64 .

echo -e "${GREEN}Building versioned Dockerfile for ARM64 (glibc)...${NC}"
docker build -f Dockerfile.versioned \
  --build-arg VERSION=$VERSION \
  --build-arg ARCH=aarch64 \
  -t text-to-cypher:$VERSION-aarch64-glibc .

echo -e "${GREEN}All builds completed successfully!${NC}"
echo -e "${YELLOW}Available images:${NC}"
docker images | grep text-to-cypher

echo -e "${YELLOW}To run an example:${NC}"
echo "docker run -d -p 8080:8080 -p 3001:3001 -e DEFAULT_MODEL=gpt-4o-mini -e DEFAULT_KEY=your-key text-to-cypher:production-$VERSION"
