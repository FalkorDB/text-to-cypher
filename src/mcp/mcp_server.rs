use std::time::Duration;

use rust_mcp_sdk::mcp_server::{HyperServerOptions, hyper_server};

use crate::mcp::server_handler::MyServerHandler;
use rust_mcp_sdk::schema::{
    Implementation, InitializeResult, LATEST_PROTOCOL_VERSION, ServerCapabilities, ServerCapabilitiesTools,
};

use rust_mcp_sdk::error::SdkResult;

/// Run the MCP server.
///
/// # Errors
///
/// Returns an error if the server fails to start or encounters a runtime error.
pub async fn run_mcp_server() -> SdkResult<()> {
    // Note: Tracing is already initialized in main, no need to initialize it again

    // STEP 1: Define server details and capabilities
    let server_details = InitializeResult {
        // server name and version
        server_info: Implementation {
            name: "Hello World MCP Server SSE".to_string(),
            version: "0.1.0".to_string(),
            title: Some("Hello World MCP Server SSE".to_string()),
        },
        capabilities: ServerCapabilities {
            // indicates that server support mcp tools
            tools: Some(ServerCapabilitiesTools { list_changed: None }),
            ..Default::default() // Using default values for other fields
        },
        meta: None,
        instructions: Some("server instructions...".to_string()),
        protocol_version: LATEST_PROTOCOL_VERSION.to_string(),
    };

    // STEP 2: instantiate our custom handler for handling MCP messages
    let handler = MyServerHandler {};

    // STEP 3: instantiate HyperServer, providing `server_details` , `handler` and HyperServerOptions
    let server = hyper_server::create_server(
        server_details,
        handler,
        HyperServerOptions {
            host: "0.0.0.0".to_string(),
            port: 3001,
            ping_interval: Duration::from_secs(5),
            ..Default::default()
        },
    );

    server.start().await?;

    Ok(())
}
