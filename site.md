
![text-to-cypher](https://github.com/user-attachments/assets/43463290-6d24-46e3-ac51-e23b0f535288)

# FalkorDB Text-to-Cypher

FalkorDB Text-to-Cypher is a Rust microservice that transforms natural language queries into Cypher queries for graph database operations. The containerized deployment bundles FalkorDB graph database, REST API, web interface, and Model Context Protocol (MCP) server.

---

## Main Features
- **Natural language to Cypher translation** using configurable LLM backends
- Automatic **schema discovery** and graph structure analysis
- RESTful API with OpenAPI specification and streaming support
- Multi-provider AI integration (OpenAI, Anthropic, custom endpoints)
- Browser-based graph visualization via integrated web interface
- Cross-platform container images for AMD64 and ARM64
- **MCP server** for AI assistant orchestration

---

## Architecture

| Service                   | Port   | Description                                |
|---------------------------|--------|--------------------------------------------|
| FalkorDB Database         | 6379   | Redis protocol, graph database engine             |
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

### Enterprise Knowledge Graphs
Allow business users to query organizational knowledge graphs without Cypher expertise. Suitable for CRM, ERP or HR systems, large product catalogs, and compliance networks.

### Conversational Graph Analytics
Integrate with chatbots to provide natural language access to graph data. Query customer relationship networks, supply chains, or social graphs through conversational interfaces.

### Rapid Query Construction
Generate Cypher queries from requirements written in plain English. Reduces time from concept to executable query.

---

## Troubleshooting

- Ensure ports `6379`, `3000`, `8080`, `3001` are available.
- Set `DEFAULT_MODEL` and `DEFAULT_KEY` environment variables.
- MCP Server requires both configuration keys.
- View logs: `docker logs -f <container-name>`
- Documentation: [http://localhost:8080/swagger-ui/](http://localhost:8080/swagger-ui/)

### Issue Reporting
Submit technical issues with logs and reproduction steps: https://github.com/FalkorDB/text-to-cypher/issues

---

## License

This project is licensed under the MIT License. See the LICENSE file for details.

---
