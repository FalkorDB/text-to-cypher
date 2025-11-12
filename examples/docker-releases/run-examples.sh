#!/bin/bash
# run-examples.sh - Script to run Docker examples

set -e

# Color output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m' # No Color

VERSION=${1:-v2025.07.23-d1092dc}
API_KEY=${2:-"demo-key-change-me"}
MODEL=${3:-"gpt-4o-mini"}
REST_PORT=${REST_PORT:-8080}
MCP_PORT=${MCP_PORT:-3001}

echo -e "${YELLOW}Running text-to-cypher Docker examples${NC}"
echo -e "${BLUE}Version: $VERSION${NC}"
echo -e "${BLUE}Model: $MODEL${NC}"
echo -e "${BLUE}API Key: ${API_KEY:0:10}...${NC}"
echo -e "${BLUE}REST API Port: $REST_PORT${NC}"
echo -e "${BLUE}MCP Server Port: $MCP_PORT${NC}"

# Function to wait for service
wait_for_service() {
    local url=$1
    local name=$2
    local max_attempts=30
    local attempt=1
    
    echo -e "${YELLOW}Waiting for $name to be ready...${NC}"
    while [ $attempt -le $max_attempts ]; do
        if curl -s -f "$url" > /dev/null 2>&1; then
            echo -e "${GREEN}$name is ready!${NC}"
            return 0
        fi
        echo -e "${BLUE}Attempt $attempt/$max_attempts - waiting for $name...${NC}"
        sleep 2
        attempt=$((attempt + 1))
    done
    
    echo -e "${RED}$name failed to start after $max_attempts attempts${NC}"
    return 1
}

# Example 1: Simple run
echo -e "\n${GREEN}=== Example 1: Simple Container ===${NC}"
docker run -d \
  --name text-to-cypher-simple \
  -p $REST_PORT:8080 \
  -p $MCP_PORT:3001 \
  -e DEFAULT_MODEL="$MODEL" \
  -e DEFAULT_KEY="$API_KEY" \
  text-to-cypher:simple-$VERSION

# Wait for service and test
if wait_for_service "http://localhost:$REST_PORT/swagger-ui/" "text-to-cypher-simple"; then
    echo -e "${GREEN}✅ Simple container is running${NC}"
    echo -e "${BLUE}API Documentation: http://localhost:$REST_PORT/swagger-ui/${NC}"
    echo -e "${BLUE}Health check: curl http://localhost:$REST_PORT/swagger-ui/${NC}"
    
    # Test basic functionality
    echo -e "${YELLOW}Testing API endpoint...${NC}"
    if curl -s "http://localhost:$REST_PORT/swagger-ui/" | grep -q "Swagger"; then
        echo -e "${GREEN}✅ Swagger UI is accessible${NC}"
    else
        echo -e "${RED}❌ Swagger UI test failed${NC}"
    fi
else
    echo -e "${RED}❌ Simple container failed to start${NC}"
fi

# Stop simple container
echo -e "${YELLOW}Stopping simple container...${NC}"
docker stop text-to-cypher-simple && docker rm text-to-cypher-simple

# Example 2: Production container
echo -e "\n${GREEN}=== Example 2: Production Container ===${NC}"
PROD_REST_PORT=$((REST_PORT + 1))
PROD_MCP_PORT=$((MCP_PORT + 1))
docker run -d \
  --name text-to-cypher-prod \
  -p $PROD_REST_PORT:8080 \
  -p $PROD_MCP_PORT:3001 \
  -e DEFAULT_MODEL="$MODEL" \
  -e DEFAULT_KEY="$API_KEY" \
  -e RUST_LOG=info \
  text-to-cypher:production-$VERSION

# Wait for service and test
if wait_for_service "http://localhost:$PROD_REST_PORT/swagger-ui/" "text-to-cypher-prod"; then
    echo -e "${GREEN}✅ Production container is running${NC}"
    echo -e "${BLUE}API Documentation: http://localhost:$PROD_REST_PORT/swagger-ui/${NC}"
    echo -e "${BLUE}MCP Server: localhost:$PROD_MCP_PORT${NC}"
    
    # Show container info
    echo -e "${YELLOW}Container information:${NC}"
    docker exec text-to-cypher-prod text-to-cypher --help | head -5
    
    # Show logs
    echo -e "${YELLOW}Recent logs:${NC}"
    docker logs text-to-cypher-prod | tail -5
else
    echo -e "${RED}❌ Production container failed to start${NC}"
fi

# Example 3: Docker Compose
echo -e "\n${GREEN}=== Example 3: Docker Compose Stack ===${NC}"
cat > .env << EOF
OPENAI_API_KEY=$API_KEY
DEFAULT_MODEL=$MODEL
REST_PORT=$REST_PORT
MCP_PORT=$MCP_PORT
EOF

echo -e "${YELLOW}Starting Docker Compose stack...${NC}"
docker-compose up -d

# Wait for the compose service
if wait_for_service "http://localhost:$REST_PORT/swagger-ui/" "docker-compose text-to-cypher"; then
    echo -e "${GREEN}✅ Docker Compose stack is running${NC}"
    echo -e "${BLUE}Text-to-Cypher API: http://localhost:$REST_PORT/swagger-ui/${NC}"
    echo -e "${BLUE}FalkorDB: localhost:6379${NC}"
    
    # Show all running containers
    echo -e "${YELLOW}Running containers:${NC}"
    docker-compose ps
else
    echo -e "${RED}❌ Docker Compose stack failed to start${NC}"
fi

echo -e "\n${GREEN}=== Usage Examples Complete ===${NC}"
echo -e "${YELLOW}To clean up:${NC}"
echo "docker stop text-to-cypher-prod && docker rm text-to-cypher-prod"
echo "docker-compose down"
echo "docker rmi \$(docker images -q text-to-cypher)"

echo -e "\n${YELLOW}To test the API:${NC}"
echo "curl http://localhost:$REST_PORT/swagger-ui/"
echo "curl http://localhost:$PROD_REST_PORT/swagger-ui/"
