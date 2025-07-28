#!/bin/bash
# build-and-push.sh - Build and push multi-architecture images to registry

set -e

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

VERSION=${1:-v0.1.0-alpha.3}
REGISTRY=${2:-""}
PLATFORMS=${3:-"linux/amd64,linux/arm64"}

if [ -z "$REGISTRY" ]; then
    echo -e "${RED}Usage: $0 <version> <registry> [platforms]${NC}"
    echo -e "${YELLOW}Example: $0 v1.0.0 ghcr.io/falkordb/text-to-cypher${NC}"
    echo -e "${YELLOW}Example: $0 v1.0.0 docker.io/myuser/text-to-cypher linux/amd64,linux/arm64${NC}"
    exit 1
fi

echo -e "${YELLOW}Building and pushing multi-architecture images for text-to-cypher $VERSION${NC}"
echo -e "${BLUE}Registry: $REGISTRY${NC}"
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

# Build and push simple version (multi-arch)
echo -e "${GREEN}Building and pushing simple multi-architecture image...${NC}"
docker buildx build --platform $PLATFORMS -f Dockerfile.simple \
  -t $REGISTRY:simple-$VERSION \
  -t $REGISTRY:simple-latest \
  --push .

# Build and push multi-stage version (multi-arch)
echo -e "${GREEN}Building and pushing production multi-architecture image...${NC}"
docker buildx build --platform $PLATFORMS -f Dockerfile.multistage \
  -t $REGISTRY:production-$VERSION \
  -t $REGISTRY:production-latest \
  --push .

# Build and push versioned version (multi-arch)
echo -e "${GREEN}Building and pushing versioned multi-architecture image...${NC}"
docker buildx build --platform $PLATFORMS -f Dockerfile.versioned \
  --build-arg VERSION=$VERSION \
  -t $REGISTRY:$VERSION \
  -t $REGISTRY:latest \
  --push .

echo -e "${GREEN}All multi-architecture images pushed successfully!${NC}"

echo -e "${YELLOW}Available images in registry:${NC}"
echo "  $REGISTRY:simple-$VERSION (and :simple-latest)"
echo "  $REGISTRY:production-$VERSION (and :production-latest)"
echo "  $REGISTRY:$VERSION (and :latest)"

echo -e "${YELLOW}To inspect multi-arch manifest:${NC}"
echo "docker buildx imagetools inspect $REGISTRY:$VERSION"

echo -e "${YELLOW}To run from registry:${NC}"
echo "docker run -d -p 8080:8080 -p 3001:3001 -e DEFAULT_MODEL=gpt-4o-mini -e DEFAULT_KEY=your-key $REGISTRY:latest"

echo -e "${BLUE}Images support platforms: $PLATFORMS${NC}"
