# FalkorDB Text-to-Cypher

FalkorDB Text-to-Cypher is a high-performance Rust-based API service that converts natural language queries into Cypher database queries for graph analytics. Delivered as an all-in-one Docker solution, it bundles the FalkorDB graph database, REST API, interactive web browser, and Model Context Protocol (MCP) server for AI assistant integration.

---

## Features

- **Natural Language to Cypher:** Translate human questions to Cypher queries using AI.
- **Schema Discovery:** Automatically analyze and visualize graph schema.
- **RESTful API:** HTTP endpoints with OpenAPI/Swagger docs and SSE streaming.
- **Integrated FalkorDB:** High-performance graph database with Redis protocol compatibility.
- **Web Interface:** Visual graph explorer and query builder.
- **AI Model Integration:** Flexible configuration for GenAI and multi-provider support.
- **All-in-One Docker:** Deployment for AMD64 and ARM64 in a single container.
- **Production Ready:** Robust error handling, logging, and multi-platform support.

---

## Architecture

| Service                   | Port   | Description                                |
|---------------------------|--------|--------------------------------------------|
| FalkorDB Database         | 6379   | Redis protocol, graph database             |
| FalkorDB Web Interface    | 3000   | Browser UI for data visualization          |
| Text-to-Cypher API        | 8080   | REST API, OpenAPI docs                     |
| MCP Server                | 3001   | AI assistant protocol for integrations     |

---

## Quick Start

Deploy the complete stack with Docker (AMD64 or ARM64):

```bash
docker run -p 6379:6379 -p 3000:3000 -p 8080:8080 -p 3001:3001 \
  -e DEFAULT_MODEL=gpt-4o-mini \
  -e DEFAULT_KEY=your-api-key \
  ghcr.io/falkordb/text-to-cypher:latest
```

Service addresses:

- Web UI: `http://localhost:3000`
- REST API: `http://localhost:8080`
- Swagger docs: `http://localhost:8080/swagger-ui/`
- MCP Server: `http://localhost:3001`
- Database: `localhost:6379`

---

## API Usage

Example: Translate Text to Cypher

```bash
curl -X POST "http://localhost:8080/text_to_cypher" \
  -H "Content-Type: application/json" \
  -d '{
    "graph_name": "movies",
    "chat_request": {
      "messages": [
        { "role": "User", "content": "Find all actors who appeared in movies released after 2020" }
      ]
    },
    "model": "gpt-4o-mini",
    "key": "your-api-key"
  }'
```

---

## Real-Time Streaming (SSE)

```javascript
const eventSource = new EventSource('/text_to_cypher', {
  method: 'POST',
  headers: { 'Content-Type': 'application/json' },
  body: JSON.stringify({
    graph_name: "social_network",
    chat_request: { 
      messages: [
        { role: "User", content: "Who are John's friends?" }
      ]
    }
  })
});

eventSource.onmessage = (event) => {
  const data = JSON.parse(event.data);
  console.log('Progress:', data);
};
```

---

## MCP Server Integration

Integrate with AI assistants using MCP Inspector or direct protocol, and use the `text_to_cypher` tool for conversion.

**Parameters:**

- `graph_name`: (string) Target graph database
- `question`: (string) Natural language query

**Example JSON:**

```json
{
  "graph_name": "social_network",
  "question": "Who are the friends of John with more than 5 mutual connections?"
}
```

---

## Use Cases

- **Enterprise Knowledge Graphs:** Democratize graph data access.
- **Conversational AI:** Natural language graph exploration via chatbots or assistants.
- **Graph Analytics:** Build queries without knowing Cypher.

---

## Troubleshooting

- Ensure ports `6379`, `3000`, `8080`, `3001` are available.
- Set `DEFAULT_MODEL` and `DEFAULT_KEY` environment variables.
- MCP Server requires both configuration keys.
- View logs: `docker logs -f <container-name>`
- Documentation: [http://localhost:8080/swagger-ui/](http://localhost:8080/swagger-ui/)
- Issues: [GitHub Issues](https://github.com/FalkorDB/text-to-cypher/issues)

---

## License

MIT License

---

FalkorDB Text-to-Cypher  
Unlock intuitive graph analytics and natural language access to your data!
