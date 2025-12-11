use rust_mcp_sdk::macros::{mcp_tool, JsonSchema};
use serde::{Deserialize, Serialize};

#[mcp_tool(
    name = "talk_with_a_graph",
    description = "Answer user questions based on the data in the given graph. 

IMPORTANT: Before using this tool, you should:
1. First explore available graph resources to see what graphs are available
2. Use the resource URIs to understand the schema of each graph
3. Resources are available at URIs like 'falkordb://graph/{graph_name}'
4. Each resource contains the graph's schema information in JSON format
5. Use the schema information to understand what entities and relationships exist

Available graph resources can be discovered through the MCP resource system. Each graph resource contains:
- Entity types (nodes) with their attributes and data types
- Relationship types (edges) with source and target entity types
- Complete schema information to help formulate appropriate questions

Example workflow:
1. Check available resources to see graphs like 'falkordb://graph/social', 'falkordb://graph/knowledge_base'
2. Read the resource content to understand the schema
3. Use this tool with an appropriate graph_name and question based on the schema"
)]
#[derive(Debug, Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct TextToCypherTool {
    /// The name of the graph database to query
    ///
    /// This should be the exact name of the graph as it exists in your `FalkorDB` instance.
    /// You can discover available graphs by exploring the MCP resources first.
    ///
    /// How to find available graphs:
    /// 1. Check the available resources in the MCP system
    /// 2. Look for resources with URIs like `<falkordb://graph/{graph_name}>`
    /// 3. The {`graph_name`} part is what you should use here
    ///
    /// Examples: "social", "`knowledge_base`", "`customer_data`", "`product_catalog`"
    ///
    /// IMPORTANT: Always check available resources first to see what graphs exist!
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
    /// IMPORTANT: Before asking questions, read the graph resource to understand the schema:
    /// - What entity types (node labels) exist?
    /// - What attributes do entities have?
    /// - What relationship types connect entities?
    /// - Use this schema information to ask relevant questions
    ///
    /// Examples based on a social graph schema:
    /// - "Who are all the people in the network?" (if Person entities exist)
    /// - "Show me all friendships" (if FRIEND relationships exist)
    /// - "Find people with specific names" (if Person has name attribute)
    ///
    /// Examples for other domains:
    /// - "Who are the top 5 customers by revenue?" (for business graphs)
    /// - "Show me all products in the electronics category" (for e-commerce graphs)
    /// - "Find all users who purchased items in the last 30 days" (for transaction graphs)
    /// - "What are the most connected nodes in the network?" (for network analysis)
    ///
    /// Required: Yes
    /// Type: String
    /// Min length: 5
    /// Max length: 1000
    pub question: String,
}
