#!/bin/bash
# build-examples.sh - Script to build all Docker examples with multi-architecture support

set -e

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

VERSION=${1:-v0.1.0-alpha.1}
PLATFORMS=${2:-"linux/amd64,linux/arm64"}

echo -e "${YELLOW}Building Multi-Architecture Docker examples for text-to-cypher $VERSION${NC}"
echo -e "${BLUE}Platforms: $PLATFORMS${NC}"

# Ensure buildx is available
if ! docker buildx version >/dev/null 2>&1; then
    echo -e "${RED}Docker buildx is required for multi-architecture builds${NC}"
    exit 1
fi

# Create a builder instance if it doesn't exist
if ! docker buildx inspect multiarch-builder >/dev/null 2>&1; then
    echo -e "${YELLOW}Creating buildx builder instance...${NC}"
    docker buildx create --name multiarch-builder --driver docker-container --bootstrap
fi

echo -e "${YELLOW}Using buildx builder: multiarch-builder${NC}"
docker buildx use multiarch-builder

# Build simple version (multi-arch)
echo -e "${GREEN}Building simple multi-architecture Dockerfile...${NC}"
docker buildx build --platform $PLATFORMS --build-arg VERSION=$VERSION -f Dockerfile.simple -t text-to-cypher:simple-$VERSION .

# Build multi-stage version (multi-arch)
echo -e "${GREEN}Building multi-stage multi-architecture Dockerfile...${NC}"
docker buildx build --platform $PLATFORMS --build-arg TEXT_TO_CYPHER_VERSION=$VERSION -f Dockerfile.multistage -t text-to-cypher:production-$VERSION .

# Build versioned version (multi-arch)
echo -e "${GREEN}Building versioned multi-architecture Dockerfile...${NC}"
docker buildx build --platform $PLATFORMS -f Dockerfile.versioned \
  --build-arg VERSION=$VERSION \
  -t text-to-cypher:$VERSION .

echo -e "${GREEN}All multi-architecture builds completed successfully!${NC}"
echo -e "${YELLOW}Built images are stored in buildx cache (not loaded locally)${NC}"

# Build single platform version for local testing
echo -e "${GREEN}Building single platform version for local testing...${NC}"
docker buildx build --platform linux/amd64 --build-arg VERSION=$VERSION -f Dockerfile.simple -t text-to-cypher:simple-$VERSION-local --load .
echo -e "${YELLOW}Available images:${NC}"
docker images | grep text-to-cypher

echo -e "${YELLOW}To inspect multi-arch manifest (if pushed to registry):${NC}"
echo "docker buildx imagetools inspect text-to-cypher:$VERSION"

echo -e "${YELLOW}To run the local test image:${NC}"
echo "docker run -d -p 8080:8080 -p 3001:3001 -e DEFAULT_MODEL=gpt-4o-mini -e DEFAULT_KEY=your-key text-to-cypher:simple-$VERSION-local"

echo -e "${BLUE}Note: Multi-arch images are built for platforms: $PLATFORMS${NC}"
echo -e "${BLUE}Use build-and-push.sh to push multi-arch images to a registry${NC}"
