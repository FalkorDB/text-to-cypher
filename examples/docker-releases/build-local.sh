#!/bin/bash
# build-local.sh - Build Docker examples for current platform only (for local testing)

set -e

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

VERSION=${1:-v0.1.0-alpha.3}

echo -e "${YELLOW}Building Docker examples for text-to-cypher $VERSION (current platform only)${NC}"

# Detect current platform
CURRENT_PLATFORM=$(docker version --format '{{.Server.Os}}/{{.Server.Arch}}')
echo -e "${BLUE}Current platform: $CURRENT_PLATFORM${NC}"

# Build simple version (current platform)
echo -e "${GREEN}Building simple Dockerfile...${NC}"
docker build -f Dockerfile.simple -t text-to-cypher:simple-$VERSION .

# Build multi-stage version (current platform)
echo -e "${GREEN}Building multi-stage Dockerfile...${NC}"
docker build -f Dockerfile.multistage -t text-to-cypher:production-$VERSION .

# Build versioned version (current platform)
echo -e "${GREEN}Building versioned Dockerfile...${NC}"
docker build -f Dockerfile.versioned \
  --build-arg VERSION=$VERSION \
  -t text-to-cypher:$VERSION .

echo -e "${GREEN}All builds completed successfully!${NC}"
echo -e "${YELLOW}Available images:${NC}"
docker images | grep text-to-cypher

echo -e "${YELLOW}To run an example:${NC}"
echo "docker run -d -p 8080:8080 -p 3001:3001 -e DEFAULT_MODEL=gpt-4o-mini -e DEFAULT_KEY=your-key text-to-cypher:production-$VERSION"

echo -e "${BLUE}Note: Images are built for current platform only: $CURRENT_PLATFORM${NC}"
echo -e "${BLUE}Use build-examples.sh for multi-architecture builds${NC}"
