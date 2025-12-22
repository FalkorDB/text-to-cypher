# Using text-to-cypher from Python

This guide explains how to use the text-to-cypher Rust library from Python applications.

## Overview

Since text-to-cypher is a Rust library published on crates.io, you can use it from Python in several ways:

## Option 1: Use the REST API (Recommended)

The easiest way to use text-to-cypher from Python is to run the server and make HTTP requests.

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

### 2. Install Python HTTP client

```bash
pip install requests
# or
pip install httpx
```

### 3. Use from Python

```python
import requests
from typing import List, Dict, Optional, Literal
from dataclasses import dataclass

@dataclass
class ChatMessage:
    role: Literal['user', 'assistant', 'system']
    content: str

@dataclass
class ChatRequest:
    messages: List[ChatMessage]

@dataclass
class TextToCypherRequest:
    graph_name: str
    chat_request: ChatRequest
    model: Optional[str] = None
    key: Optional[str] = None
    falkordb_connection: Optional[str] = None
    cypher_only: bool = False

@dataclass
class TextToCypherResponse:
    status: str
    schema: Optional[str] = None
    cypher_query: Optional[str] = None
    cypher_result: Optional[str] = None
    answer: Optional[str] = None
    error: Optional[str] = None

class TextToCypherClient:
    def __init__(self, base_url: str = "http://localhost:8080"):
        self.base_url = base_url.rstrip('/')
        self.session = requests.Session()

    def text_to_cypher(
        self,
        graph_name: str,
        question: str,
        model: Optional[str] = None,
        key: Optional[str] = None,
        falkordb_connection: Optional[str] = None,
    ) -> TextToCypherResponse:
        """
        Convert natural language to Cypher query and execute it.
        
        Args:
            graph_name: Name of the graph to query
            question: Natural language question
            model: AI model to use (optional if set in server)
            key: API key (optional if set in server)
            falkordb_connection: FalkorDB connection string (optional if set in server)
            
        Returns:
            TextToCypherResponse with query, result, and answer
            
        Raises:
            Exception: If the request fails or returns an error
        """
        request_data = {
            "graph_name": graph_name,
            "chat_request": {
                "messages": [
                    {
                        "role": "user",
                        "content": question
                    }
                ]
            }
        }
        
        if model:
            request_data["model"] = model
        if key:
            request_data["key"] = key
        if falkordb_connection:
            request_data["falkordb_connection"] = falkordb_connection

        response = self.session.post(
            f"{self.base_url}/text_to_cypher",
            json=request_data
        )
        response.raise_for_status()
        
        data = response.json()
        
        if data.get("status") == "error":
            raise Exception(data.get("error", "Unknown error"))
        
        return TextToCypherResponse(
            status=data["status"],
            schema=data.get("schema"),
            cypher_query=data.get("cypher_query"),
            cypher_result=data.get("cypher_result"),
            answer=data.get("answer"),
            error=data.get("error")
        )

    def cypher_only(
        self,
        graph_name: str,
        question: str,
        model: Optional[str] = None,
        key: Optional[str] = None,
        falkordb_connection: Optional[str] = None,
    ) -> TextToCypherResponse:
        """
        Generate Cypher query without executing it.
        
        Args:
            graph_name: Name of the graph
            question: Natural language question
            model: AI model to use (optional if set in server)
            key: API key (optional if set in server)
            falkordb_connection: FalkorDB connection string (optional if set in server)
            
        Returns:
            TextToCypherResponse with generated query only
            
        Raises:
            Exception: If the request fails or returns an error
        """
        request_data = {
            "graph_name": graph_name,
            "chat_request": {
                "messages": [
                    {
                        "role": "user",
                        "content": question
                    }
                ]
            },
            "cypher_only": True
        }
        
        if model:
            request_data["model"] = model
        if key:
            request_data["key"] = key
        if falkordb_connection:
            request_data["falkordb_connection"] = falkordb_connection

        response = self.session.post(
            f"{self.base_url}/text_to_cypher",
            json=request_data
        )
        response.raise_for_status()
        
        data = response.json()
        
        if data.get("status") == "error":
            raise Exception(data.get("error", "Unknown error"))
        
        return TextToCypherResponse(
            status=data["status"],
            schema=data.get("schema"),
            cypher_query=data.get("cypher_query"),
            cypher_result=data.get("cypher_result"),
            answer=data.get("answer"),
            error=data.get("error")
        )

    def close(self):
        """Close the session."""
        self.session.close()

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        self.close()


# Example usage
def main():
    # Using context manager
    with TextToCypherClient("http://localhost:8080") as client:
        try:
            # Convert text to Cypher and execute
            result = client.text_to_cypher(
                graph_name="movies",
                question="Find all actors who appeared in movies released after 2020",
                model="gpt-4o-mini",
                key="your-api-key",
                falkordb_connection="falkor://localhost:6379"
            )
            
            print(f"Generated Query: {result.cypher_query}")
            print(f"Result: {result.cypher_result}")
            print(f"Answer: {result.answer}")
            
        except Exception as e:
            print(f"Error: {e}")

        try:
            # Generate query only without executing
            result = client.cypher_only(
                graph_name="social",
                question="Find all people with more than 5 friends",
                model="gpt-4o-mini",
                key="your-api-key"
            )
            
            print(f"Generated Query: {result.cypher_query}")
            
        except Exception as e:
            print(f"Error: {e}")


if __name__ == "__main__":
    main()
```

### Using with async/await (httpx)

For async applications:

```python
import httpx
import asyncio

class AsyncTextToCypherClient:
    def __init__(self, base_url: str = "http://localhost:8080"):
        self.base_url = base_url.rstrip('/')
        self.client = httpx.AsyncClient()

    async def text_to_cypher(
        self,
        graph_name: str,
        question: str,
        model: Optional[str] = None,
        key: Optional[str] = None,
        falkordb_connection: Optional[str] = None,
    ) -> Dict:
        request_data = {
            "graph_name": graph_name,
            "chat_request": {
                "messages": [{"role": "user", "content": question}]
            }
        }
        
        if model:
            request_data["model"] = model
        if key:
            request_data["key"] = key
        if falkordb_connection:
            request_data["falkordb_connection"] = falkordb_connection

        response = await self.client.post(
            f"{self.base_url}/text_to_cypher",
            json=request_data
        )
        response.raise_for_status()
        
        data = response.json()
        if data.get("status") == "error":
            raise Exception(data.get("error", "Unknown error"))
        
        return data

    async def close(self):
        await self.client.aclose()

    async def __aenter__(self):
        return self

    async def __aexit__(self, exc_type, exc_val, exc_tb):
        await self.close()


# Async usage
async def async_main():
    async with AsyncTextToCypherClient() as client:
        result = await client.text_to_cypher(
            "movies",
            "Find all actors",
            model="gpt-4o-mini",
            key="your-api-key"
        )
        print(result)

# Run async
asyncio.run(async_main())
```

## Option 2: Use PyO3 Python Bindings (Advanced)

For direct Rust-to-Python integration without a server, you can use [PyO3](https://pyo3.rs/).

### 1. Create a new Rust project with PyO3

```toml
[package]
name = "text_to_cypher_py"
version = "0.1.0"
edition = "2021"

[lib]
name = "text_to_cypher_py"
crate-type = ["cdylib"]

[dependencies]
text-to-cypher = { version = "0.1", default-features = false }
pyo3 = { version = "0.20", features = ["extension-module"] }
tokio = { version = "1", features = ["rt", "macros"] }
```

### 2. Create Python bindings

```rust
use pyo3::prelude::*;
use pyo3::exceptions::PyException;
use text_to_cypher::{TextToCypherClient, ChatRequest, ChatMessage, ChatRole};

#[pyclass]
struct PyTextToCypherClient {
    runtime: tokio::runtime::Runtime,
    client: TextToCypherClient,
}

#[pymethods]
impl PyTextToCypherClient {
    #[new]
    fn new(model: String, api_key: String, falkordb_connection: String) -> PyResult<Self> {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| PyException::new_err(e.to_string()))?;
        
        let client = TextToCypherClient::new(model, api_key, falkordb_connection);
        
        Ok(Self { runtime, client })
    }

    fn text_to_cypher(&self, graph_name: String, question: String) -> PyResult<String> {
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
                    .map_err(|e| PyException::new_err(e.to_string()))
            }
            Err(e) => Err(PyException::new_err(e.to_string())),
        }
    }

    fn cypher_only(&self, graph_name: String, question: String) -> PyResult<String> {
        let request = ChatRequest {
            messages: vec![ChatMessage {
                role: ChatRole::User,
                content: question,
            }],
        };

        let result = self.runtime.block_on(async {
            self.client.cypher_only(graph_name, request).await
        });

        match result {
            Ok(response) => {
                serde_json::to_string(&response)
                    .map_err(|e| PyException::new_err(e.to_string()))
            }
            Err(e) => Err(PyException::new_err(e.to_string())),
        }
    }
}

#[pymodule]
fn text_to_cypher_py(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyTextToCypherClient>()?;
    Ok(())
}
```

### 3. Build and install

```bash
pip install maturin
maturin develop
```

### 4. Use in Python

```python
import json
from text_to_cypher_py import PyTextToCypherClient

client = PyTextToCypherClient(
    "gpt-4o-mini",
    "your-api-key",
    "falkor://localhost:6379"
)

result_json = client.text_to_cypher("movies", "Find all actors")
result = json.loads(result_json)

print(f"Query: {result['cypher_query']}")
print(f"Answer: {result['answer']}")
```

## Comparison of Approaches

| Approach | Pros | Cons | Best For |
|----------|------|------|----------|
| REST API | Easy to set up, no compilation | Network overhead, requires server | Most applications |
| PyO3 Bindings | Fast, no network overhead | Complex setup, requires Rust toolchain | High-performance apps |

## Recommendation

**Use the REST API approach** for most Python applications. It's the simplest, most maintainable option and doesn't require any Rust knowledge or toolchain setup.

## Creating a Python Package

You can create a Python package wrapper around the REST API:

```python
# pyproject.toml
[build-system]
requires = ["setuptools>=45", "wheel"]
build-backend = "setuptools.build_meta"

[project]
name = "text-to-cypher-client"
version = "0.1.0"
description = "Python client for text-to-cypher"
dependencies = [
    "requests>=2.28.0",
]

# text_to_cypher_client/__init__.py
from .client import TextToCypherClient, TextToCypherResponse

__all__ = ["TextToCypherClient", "TextToCypherResponse"]
```

Install and use:

```bash
pip install .

# Or publish to PyPI
pip install twine
python -m build
twine upload dist/*
```

## Additional Resources

- [text-to-cypher REST API Documentation](../readme.md)
- [OpenAPI Specification](http://localhost:8080/api-doc/openapi.json) (when server is running)
- [Swagger UI](http://localhost:8080/swagger-ui/) (when server is running)
- [PyO3 Documentation](https://pyo3.rs/)
