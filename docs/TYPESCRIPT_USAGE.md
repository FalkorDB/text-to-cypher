# Using text-to-cypher from TypeScript

This guide explains how to use the text-to-cypher Rust library from TypeScript/JavaScript applications.

## Overview

Since text-to-cypher is a Rust library published on crates.io, you'll need to use one of these approaches to call it from TypeScript:

## Option 1: Use the REST API (Recommended)

The easiest way to use text-to-cypher from TypeScript is to run the server and make HTTP requests.

### 1. Run the text-to-cypher server

```bash
# Using Docker (easiest)
docker run -p 8080:8080 \
  -e DEFAULT_MODEL=gpt-4o-mini \
  -e DEFAULT_KEY=your-api-key \
  falkordb/text-to-cypher:latest

# Or install and run locally
cargo install text-to-cypher
text-to-cypher
```

### 2. Install TypeScript HTTP client

```bash
npm install axios
# or
npm install node-fetch
```

### 3. Use from TypeScript

```typescript
import axios from 'axios';

interface ChatMessage {
  role: 'user' | 'assistant' | 'system';
  content: string;
}

interface TextToCypherRequest {
  graph_name: string;
  chat_request: {
    messages: ChatMessage[];
  };
  model?: string;
  key?: string;
  falkordb_connection?: string;
  cypher_only?: boolean;
}

interface TextToCypherResponse {
  status: string;
  schema?: string;
  cypher_query?: string;
  cypher_result?: string;
  answer?: string;
  error?: string;
}

class TextToCypherClient {
  constructor(private baseUrl: string = 'http://localhost:8080') {}

  async textToCypher(
    graphName: string,
    question: string,
    options?: {
      model?: string;
      key?: string;
      falkordbConnection?: string;
    }
  ): Promise<TextToCypherResponse> {
    const request: TextToCypherRequest = {
      graph_name: graphName,
      chat_request: {
        messages: [
          {
            role: 'user',
            content: question,
          },
        ],
      },
      model: options?.model,
      key: options?.key,
      falkordb_connection: options?.falkordbConnection,
    };

    const response = await axios.post<TextToCypherResponse>(
      `${this.baseUrl}/text_to_cypher`,
      request
    );

    if (response.data.status === 'error') {
      throw new Error(response.data.error || 'Unknown error');
    }

    return response.data;
  }

  async cypherOnly(
    graphName: string,
    question: string,
    options?: {
      model?: string;
      key?: string;
      falkordbConnection?: string;
    }
  ): Promise<TextToCypherResponse> {
    const request: TextToCypherRequest = {
      graph_name: graphName,
      chat_request: {
        messages: [
          {
            role: 'user',
            content: question,
          },
        ],
      },
      model: options?.model,
      key: options?.key,
      falkordb_connection: options?.falkordbConnection,
      cypher_only: true,
    };

    const response = await axios.post<TextToCypherResponse>(
      `${this.baseUrl}/text_to_cypher`,
      request
    );

    if (response.data.status === 'error') {
      throw new Error(response.data.error || 'Unknown error');
    }

    return response.data;
  }
}

// Example usage
async function main() {
  const client = new TextToCypherClient('http://localhost:8080');

  try {
    // Convert text to Cypher and execute
    const result = await client.textToCypher(
      'movies',
      'Find all actors who appeared in movies released after 2020',
      {
        model: 'gpt-4o-mini',
        key: 'your-api-key',
        falkordbConnection: 'falkor://localhost:6379',
      }
    );

    console.log('Generated Query:', result.cypher_query);
    console.log('Result:', result.cypher_result);
    console.log('Answer:', result.answer);
  } catch (error) {
    console.error('Error:', error);
  }
}

main();
```

## Option 2: Use Node.js Native Bindings (Advanced)

For direct Rust-to-Node.js integration without a server, you can use [neon](https://neon-bindings.com/) or [napi-rs](https://napi.rs/).

### Using napi-rs

1. Create a new Rust project with napi-rs:

```bash
npm init napi
```

2. Add text-to-cypher as a dependency in `Cargo.toml`:

```toml
[dependencies]
text-to-cypher = { version = "0.1", default-features = false }
tokio = { version = "1", features = ["rt", "macros"] }
napi = "2"
napi-derive = "2"
```

3. Create bindings in `src/lib.rs`:

```rust
use napi::bindgen_prelude::*;
use napi_derive::napi;
use text_to_cypher::{TextToCypherClient, ChatRequest, ChatMessage, ChatRole};

#[napi]
pub struct TextToCypherWrapper {
    runtime: tokio::runtime::Runtime,
    client: TextToCypherClient,
}

#[napi]
impl TextToCypherWrapper {
    #[napi(constructor)]
    pub fn new(model: String, api_key: String, falkordb_connection: String) -> Result<Self> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| Error::from_reason(e.to_string()))?;
        
        let client = TextToCypherClient::new(model, api_key, falkordb_connection);
        
        Ok(Self { runtime, client })
    }

    #[napi]
    pub fn text_to_cypher(&self, graph_name: String, question: String) -> Result<String> {
        let request = ChatRequest {
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: question,
            }],
        };

        let result = self.runtime.block_on(async {
            self.client.text_to_cypher(graph_name, request).await
        });

        match result {
            Ok(response) => {
                serde_json::to_string(&response)
                    .map_err(|e| Error::from_reason(e.to_string()))
            }
            Err(e) => Err(Error::from_reason(e.to_string())),
        }
    }
}
```

4. Build and use:

```bash
npm run build
```

```typescript
import { TextToCypherWrapper } from './index';

const client = new TextToCypherWrapper(
  'gpt-4o-mini',
  'your-api-key',
  'falkor://localhost:6379'
);

const result = client.textToCypher('movies', 'Find all actors');
console.log(JSON.parse(result));
```

## Option 3: WebAssembly (WASM)

For browser usage, compile to WebAssembly:

1. Add wasm-bindgen dependencies:

```toml
[dependencies]
text-to-cypher = { version = "0.1", default-features = false }
wasm-bindgen = "0.2"
wasm-bindgen-futures = "0.4"
```

2. Build with wasm-pack:

```bash
wasm-pack build --target web
```

3. Use in browser:

```typescript
import init, { TextToCypherClient } from './pkg';

await init();

const client = new TextToCypherClient(
  'gpt-4o-mini',
  'your-api-key',
  'falkor://localhost:6379'
);

const result = await client.text_to_cypher('movies', 'Find all actors');
console.log(result);
```

## Comparison of Approaches

| Approach | Pros | Cons | Best For |
|----------|------|------|----------|
| REST API | Easy to set up, language agnostic | Network overhead, requires server | Most applications |
| Native Bindings | Fast, no network overhead | Complex setup, platform-specific | High-performance apps |
| WebAssembly | Browser support, no server | Limited async support | Browser apps |

## Recommendation

**Use the REST API approach** for most TypeScript applications. It's the simplest, most maintainable option and works well for both Node.js and browser environments.

## Additional Resources

- [text-to-cypher REST API Documentation](../readme.md)
- [OpenAPI Specification](http://localhost:8080/api-doc/openapi.json) (when server is running)
- [Swagger UI](http://localhost:8080/swagger-ui/) (when server is running)
