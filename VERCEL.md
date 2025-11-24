# Vercel Deployment Guide

This guide explains how to deploy the text-to-cypher API as serverless functions on Vercel.

## Overview

The text-to-cypher project can be deployed to Vercel as serverless functions using the [@vercel/rust](https://github.com/vercel-community/rust) runtime. This allows you to run the API without managing servers while benefiting from Vercel's global edge network.

**Current Status**: This is a minimal implementation demonstrating Vercel serverless support. Currently, only the main `/text_to_cypher` endpoint is implemented as a serverless function. Additional endpoints can be added following the same pattern.

## Prerequisites

- A [Vercel account](https://vercel.com/signup)
- [Vercel CLI](https://vercel.com/docs/cli) (optional, for local testing)
- Git repository connected to Vercel

## Project Structure

The Vercel deployment uses the following structure:

```
text-to-cypher/
├── api/                      # Serverless function endpoints
│   └── text_to_cypher.rs    # Main text-to-cypher endpoint
├── src/                      # Shared library code
│   ├── vercel.rs            # Vercel adapter utilities
│   └── ...                  # Other modules
├── vercel.json              # Vercel configuration
└── Cargo.toml               # Rust dependencies and binary definitions
```

## Configuration

### vercel.json

The `vercel.json` file configures how Vercel builds and routes requests to your serverless functions:

```json
{
  "version": 2,
  "builds": [
    {
      "src": "api/**/*.rs",
      "use": "@vercel/rust"
    }
  ],
  "routes": [
    {
      "src": "/text_to_cypher",
      "dest": "api/text_to_cypher.rs"
    }
  ]
}
```

### Environment Variables

Configure the following environment variables in your Vercel project settings:

- `DEFAULT_MODEL` - Default AI model to use (e.g., "gpt-4o-mini")
- `DEFAULT_KEY` - API key for the AI service
- `FALKORDB_CONNECTION` - FalkorDB connection string (if using an external database)

**Important Notes:**
- For serverless deployments, you'll need to use a hosted FalkorDB instance or cloud Redis service
- The local Docker-based FalkorDB won't be available in Vercel's serverless environment
- Consider using [Upstash Redis](https://upstash.com/) or another cloud Redis provider

## Deployment Steps

### Option 1: Deploy via Vercel Dashboard

1. **Connect Repository**
   - Go to [Vercel Dashboard](https://vercel.com/dashboard)
   - Click "Add New Project"
   - Import your GitHub repository

2. **Configure Project**
   - Vercel should auto-detect the Rust configuration
   - Set environment variables in the project settings
   - Deploy!

### Option 2: Deploy via Vercel CLI

1. **Install Vercel CLI**
   ```bash
   npm install -g vercel
   ```

2. **Login to Vercel**
   ```bash
   vercel login
   ```

3. **Deploy from project directory**
   ```bash
   cd text-to-cypher
   vercel
   ```

4. **Set environment variables**
   ```bash
   vercel env add DEFAULT_MODEL
   vercel env add DEFAULT_KEY
   ```

5. **Deploy to production**
   ```bash
   vercel --prod
   ```

## Testing Your Deployment

Once deployed, you can test your API:

```bash
# Replace YOUR_VERCEL_URL with your actual deployment URL
curl -X POST "https://YOUR_VERCEL_URL.vercel.app/text_to_cypher" \
  -H "Content-Type: application/json" \
  -d '{
    "graph_name": "movies",
    "chat_request": {
      "messages": [
        {
          "role": "User",
          "content": "Find all actors"
        }
      ]
    }
  }'
```

## Limitations

When running on Vercel, be aware of the following limitations:

1. **No Persistent Storage**: Serverless functions are stateless. Use external services for:
   - FalkorDB/Redis database (use a cloud-hosted instance)
   - Schema caching (consider using Vercel KV or external cache)

2. **Execution Time**: Vercel has timeout limits for serverless functions:
   - Hobby: 10 seconds
   - Pro: 60 seconds
   - Enterprise: 900 seconds

3. **Cold Starts**: First requests after idle periods may be slower due to cold starts

4. **MCP Server**: The Model Context Protocol (MCP) server is not available in serverless mode

## Standalone vs. Vercel Deployment

| Feature | Standalone | Vercel Serverless |
|---------|-----------|-------------------|
| Deployment | Self-hosted | Managed by Vercel |
| Database | Integrated FalkorDB | External required |
| Scaling | Manual | Automatic |
| MCP Server | ✅ Available | ❌ Not available |
| Web UI | ✅ Included | ❌ Not included |
| SSE Streaming | ✅ Full support | ⚠️ Limited |
| Endpoints | All endpoints | Main endpoint (extensible) |

## Extending the Implementation

The current implementation provides a foundation for Vercel deployment with the main `/text_to_cypher` endpoint. To add additional endpoints:

1. Create a new Rust file in the `api/` directory (e.g., `api/get_schema.rs`)
2. Implement a `handler` function following the pattern in `api/text_to_cypher.rs`
3. Add a binary target in `Cargo.toml`:
   ```toml
   [[bin]]
   name = "get_schema"
   path = "api/get_schema.rs"
   ```
4. Add a route in `vercel.json`:
   ```json
   {
     "src": "/get_schema",
     "dest": "api/get_schema.rs"
   }
   ```

The `src/vercel.rs` module provides utilities for handling Vercel requests and responses consistently across all endpoints.

## Development

The project maintains compatibility with both standalone and serverless deployments:

- **Standalone mode**: Run `cargo run` for local development with full features
- **Vercel mode**: Build individual functions with `cargo build --bin text_to_cypher`

## Troubleshooting

### Build Failures

If builds fail on Vercel:
1. Check Vercel build logs for specific errors
2. Ensure `@vercel/rust` runtime is correctly configured
3. Verify all dependencies are compatible with serverless environment

### Connection Issues

If you get database connection errors:
1. Verify `FALKORDB_CONNECTION` environment variable is set
2. Ensure your database is accessible from Vercel's servers
3. Check firewall rules on your database host

### Timeout Errors

If requests timeout:
1. Optimize your queries for faster execution
2. Consider upgrading your Vercel plan for longer timeouts
3. Use caching to reduce processing time

## Additional Resources

- [Vercel Documentation](https://vercel.com/docs)
- [Vercel Rust Runtime](https://github.com/vercel-community/rust)
- [FalkorDB Documentation](https://docs.falkordb.com/)
- [Text-to-Cypher GitHub Repository](https://github.com/FalkorDB/text-to-cypher)

## Support

For issues specific to:
- Vercel deployment: Open an issue on the [text-to-cypher repository](https://github.com/FalkorDB/text-to-cypher/issues)
- Vercel platform: Check [Vercel Support](https://vercel.com/support)
- FalkorDB: Visit [FalkorDB Documentation](https://docs.falkordb.com/)
