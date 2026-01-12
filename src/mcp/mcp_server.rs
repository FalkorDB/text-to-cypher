use std::time::Duration;

use rust_mcp_sdk::mcp_server::{HyperServerOptions, hyper_server};

use crate::mcp::server_handler::MyServerHandler;
use rust_mcp_sdk::schema::{
    Implementation, InitializeResult, LATEST_PROTOCOL_VERSION, ServerCapabilities, ServerCapabilitiesResources,
    ServerCapabilitiesTools,
};

use rust_mcp_sdk::error::SdkResult;

/// Run the MCP server.
///
/// # Errors
///
/// Returns an error if the server fails to start or encounters a runtime error.
pub async fn run_mcp_server(port: u16) -> SdkResult<()> {
    // Note: Tracing is already initialized in main, no need to initialize it again

    // STEP 1: Define server details and capabilities
    let server_details = InitializeResult {
        // server name and version
        server_info: Implementation {
            name: "FalkorDB Text-to-Cypher MCP Server".to_string(),
            version: "0.1.0".to_string(),
            title: Some("FalkorDB Text-to-Cypher MCP Server".to_string()),
        },
        capabilities: ServerCapabilities {
            // indicates that server support mcp tools
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            // indicates that server supports resources (graphs)
            resources: Some(ServerCapabilitiesResources {
                subscribe: None,
                list_changed: None,
            }),
            ..Default::default() // Using default values for other fields
        },
        meta: None,
        instructions: Some(
            "FalkorDB Text-to-Cypher MCP Server

This server provides access to FalkorDB graph databases through MCP resources and tools.

RESOURCES:
- Each graph is exposed as a resource with URI: falkordb://graph/{graph_name}
- Resource content contains the complete graph schema in JSON format
- Schema includes entity types (nodes), relationship types (edges), and attributes

RECOMMENDED WORKFLOW:
1. List available resources to discover graphs
2. Read resource content to understand each graph's schema
3. Use the 'talk_with_a_graph' tool with appropriate graph_name and schema-informed questions

TOOLS:
- talk_with_a_graph: Converts natural language questions to Cypher queries and executes them

Example: First check resources, then ask 'Who are all the people?' for a social graph with Person entities."
                .to_string(),
        ),
        protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
    };

    // STEP 2: instantiate our custom handler for handling MCP messages
    let handler = MyServerHandler {};

    // STEP 3: instantiate HyperServer, providing `server_details` , `handler` and HyperServerOptions
    tracing::info!("Starting MCP server on 0.0.0.0:{}", port);

    let server = hyper_server::create_server(
        server_details,
        handler,
        HyperServerOptions {
            host: "0.0.0.0".to_string(),
            port,
            ping_interval: Duration::from_secs(5),
            ..Default::default()
        },
    );

    server.start().await?;

    Ok(())
}
