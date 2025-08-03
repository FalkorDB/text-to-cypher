use rust_mcp_sdk::macros::{JsonSchema, mcp_tool};
use serde::{Deserialize, Serialize};

#[mcp_tool(
    name = "talk_with_a_graph",
    description = "Answer user questions based on the data in the given graph"
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TextToCypherTool {
    /// The name of the graph database to query
    ///
    /// This should be the exact name of the graph as it exists in your FalkorDB instance.
    /// Examples: "knowledge_graph", "customer_data", "product_catalog"
    ///
    /// Required: Yes
    /// Type: String
    /// Min length: 1
    /// Max length: 100
    #[serde(rename = "graph_name")]
    pub graph_name: String,

    /// Natural language question to be converted to Cypher and answered using the graph data
    ///
    /// Provide a clear, specific question about the data in your graph. The AI will convert
    /// this to a Cypher query and execute it against your graph database.
    ///
    /// Examples:
    /// - "Who are the top 5 customers by revenue?"
    /// - "Show me all products in the electronics category"
    /// - "Find all users who purchased items in the last 30 days"
    /// - "What are the most connected nodes in the network?"
    ///
    /// Required: Yes
    /// Type: String
    /// Min length: 5
    /// Max length: 1000
    pub question: String,
}
