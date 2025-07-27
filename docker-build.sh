#!/bin/bash

# Function to show usage
show_usage() {
    echo "Usage: $0 [OPTIONS]"
    echo ""
    echo "OPTIONS:"
    echo "  --version VERSION       Set the version tag (default: v0.1.0-alpha.1)"
    echo "  --platforms PLATFORMS   Set target platforms (default: linux/amd64,linux/arm64)"
    echo "  --image-name NAME       Set image name (default: text-to-cypher)"
    echo "  --push                  Push to registry (default: load locally)"
    echo "  --local                 Load locally only (default behavior)"
    echo "  --help                  Show this help message"
    echo ""
    echo "Examples:"
    echo "  $0 --version v1.0.0 --push"
    echo "  $0 --local --version latest"
    echo "  $0 --platforms linux/amd64 --version v1.0.0"
}

# Default values
VERSION="v0.1.0-alpha.1"
PLATFORMS="linux/amd64,linux/arm64"
IMAGE_NAME="text-to-cypher"
PUSH=false

# Parse command line arguments
while [[ $# -gt 0 ]]; do
    case $1 in
        --version)
            VERSION="$2"
            shift 2
            ;;
        --platforms)
            PLATFORMS="$2"
            shift 2
            ;;
        --image-name)
            IMAGE_NAME="$2"
            shift 2
            ;;
        --push)
            PUSH=true
            shift
            ;;
        --local)
            PUSH=false
            shift
            ;;
        --help)
            show_usage
            exit 0
            ;;
        *)
            echo "Unknown option: $1"
            show_usage
            exit 1
            ;;
    esac
done

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

echo -e "${YELLOW}Building Docker image with buildx...${NC}"
echo -e "${BLUE}Version: ${VERSION}${NC}"
echo -e "${BLUE}Platforms: ${PLATFORMS}${NC}"
echo -e "${BLUE}Image: ${IMAGE_NAME}${NC}"

# Ensure buildx is available
if ! docker buildx version >/dev/null 2>&1; then
    echo -e "${RED}Docker buildx is required for multi-architecture builds${NC}"
    echo "Please install Docker Desktop or enable buildx"
    exit 1
fi

# Create a builder instance if it doesn't exist
BUILDER_NAME="text-to-cypher-builder"
if ! docker buildx inspect ${BUILDER_NAME} >/dev/null 2>&1; then
    echo -e "${YELLOW}Creating buildx builder instance...${NC}"
    docker buildx create --name ${BUILDER_NAME} --driver docker-container --bootstrap
fi

echo -e "${YELLOW}Using buildx builder: ${BUILDER_NAME}${NC}"
docker buildx use ${BUILDER_NAME}

# Build the Docker image
if [ "$PUSH" = "true" ]; then
    echo -e "${GREEN}Building and pushing multi-platform Docker image...${NC}"
    docker buildx build \
        --platform ${PLATFORMS} \
        --build-arg VERSION=${VERSION} \
        -t ${IMAGE_NAME}:${VERSION} \
        -t ${IMAGE_NAME}:latest \
        --push \
        .
else
    echo -e "${GREEN}Building multi-platform Docker image (local only)...${NC}"
    docker buildx build \
        --platform ${PLATFORMS} \
        --build-arg VERSION=${VERSION} \
        -t ${IMAGE_NAME}:${VERSION} \
        -t ${IMAGE_NAME}:latest \
        --load \
        .
fi

# Check if build was successful
if [ $? -eq 0 ]; then
    echo -e "${GREEN}✅ Docker image built successfully!${NC}"
    echo ""
    echo -e "${YELLOW}To run the container:${NC}"
    echo "  docker run -p 6379:6379 -p 3000:3000 -p 8080:8080 -p 3001:3001 ${IMAGE_NAME}:${VERSION}"
    echo ""
    echo -e "${YELLOW}Available ports:${NC}"
    echo "  - 6379: FalkorDB (Redis protocol)"
    echo "  - 3000: FalkorDB web interface"
    echo "  - 8080: text-to-cypher HTTP API"
    echo "  - 3001: text-to-cypher additional port"
    echo ""
    echo -e "${YELLOW}To run with environment variables:${NC}"
    echo "  docker run -p 6379:6379 -p 3000:3000 -p 8080:8080 -p 3001:3001 \\"
    echo "    -e DEFAULT_MODEL=gpt-4o-mini -e DEFAULT_KEY=your-key ${IMAGE_NAME}:${VERSION}"
    echo ""
    echo -e "${BLUE}Built for platforms: ${PLATFORMS}${NC}"
    
    if [ "$PUSH" = "true" ]; then
        echo -e "${BLUE}Image pushed to registry${NC}"
    else
        echo -e "${BLUE}Image available locally (use --push to push to registry)${NC}"
    fi
else
    echo -e "${RED}❌ Docker build failed!${NC}"
    exit 1
fi
