//! Core shared functionality for text-to-cypher processing
//!
//! This module contains common logic used by both the standalone server and serverless functions.

use crate::schema::discovery::Schema;
use falkordb::{FalkorClientBuilder, FalkorConnectionInfo};

/// Discover schema from a FalkorDB graph
///
/// This is the core schema discovery logic shared between standalone and serverless modes.
pub async fn discover_graph_schema(
    falkordb_connection: &str,
    graph_name: &str,
) -> Result<Schema, String> {
    let connection_info: FalkorConnectionInfo = falkordb_connection
        .try_into()
        .map_err(|e| format!("Invalid connection info: {e}"))?;

    let client = FalkorClientBuilder::new_async()
        .with_connection_info(connection_info)
        .build()
        .await
        .map_err(|e| format!("Failed to build client: {e}"))?;

    let mut graph = client.select_graph(graph_name);
    let schema = Schema::discover_from_graph(&mut graph, 100)
        .await
        .map_err(|e| format!("Failed to discover schema: {e}"))?;

    Ok(schema)
}
