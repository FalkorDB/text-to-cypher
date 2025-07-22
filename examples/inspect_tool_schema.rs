// Add the crate to access internal modules
use text_to_cypher::mcp::tools::TextToCypherTool;

fn main() {
    // Get the tool schema that will be published to MCP callers
    let tool = TextToCypherTool::tool();
    
    println!("Tool Schema that will be published to callers:");
    println!("===========================================");
    
    // Print the tool information
    println!("Name: {}", tool.name);
    println!("Description: {}", tool.description.unwrap_or_default());
    
    // Print the input schema (this is what callers see)
    let input_schema = &tool.input_schema;
    println!("\nInput Schema:");
    if let Ok(pretty_json) = serde_json::to_string_pretty(input_schema) {
        println!("{}", pretty_json);
    }
    
    println!("\n===========================================");
    println!("This schema provides callers with:");
    println!("- Field names and types");
    println!("- Field descriptions and requirements");
    println!("- Examples for each field");
    println!("- Validation rules (when available)");
}
