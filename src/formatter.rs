//! Query Result Formatter Module
//!
//! This module provides functionality to format `FalkorDB` query results in a compact,
//! LLM-friendly format. The formatting is optimized for:
//!
//! - **Compactness**: Minimizes unnecessary whitespace and verbose labels
//! - **Readability**: Uses familiar Cypher-like syntax for nodes and edges
//! - **LLM Parsing**: Structured output that AI models can easily understand and reference
//!
//! ## Examples
//!
//! - Single value: `"John Doe"`
//! - Single record: `[(:Person {name: "John"}), 25, "Engineer"]`
//! - Multiple records: `1. (:Person {name: "John"})\n2. (:Person {name: "Jane"})`

use falkordb::FalkorValue;
use std::fmt::Write;

/// Formats query results in a compact, LLM-friendly format
pub fn format_query_records(records: &[Vec<FalkorValue>]) -> String {
    if records.is_empty() {
        return "No results returned.".to_string();
    }

    if records.len() == 1 {
        // Single record: compact inline format
        let record = &records[0];
        if record.len() == 1 {
            // Single field: just return the value
            format_falkor_value(&record[0])
        } else {
            // Multiple fields: array format
            let values: Vec<String> = record.iter().map(format_falkor_value).collect();
            format!("[{}]", values.join(", "))
        }
    } else {
        // Multiple records: numbered list format
        let mut res = String::new();

        for (idx, record) in records.iter().enumerate() {
            write!(res, "{}. ", idx + 1).unwrap();

            if record.len() == 1 {
                // Single field: just the value
                writeln!(res, "{}", format_falkor_value(&record[0])).unwrap();
            } else {
                // Multiple fields: array format
                let values: Vec<String> = record.iter().map(format_falkor_value).collect();
                writeln!(res, "[{}]", values.join(", ")).unwrap();
            }
        }

        res.trim_end().to_string()
    }
}

/// Formats a single `FalkorDB` value in a readable, compact format
fn format_falkor_value(value: &FalkorValue) -> String {
    match value {
        FalkorValue::Bool(b) => b.to_string(),
        FalkorValue::I64(i) => i.to_string(),
        FalkorValue::F64(f) => f.to_string(),
        FalkorValue::Node(node) => {
            let labels = if node.labels.is_empty() {
                String::new()
            } else {
                format!(":{}", node.labels.join(":"))
            };

            let props = if node.properties.is_empty() {
                String::new()
            } else {
                let prop_strings: Vec<String> = node
                    .properties
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, format_falkor_value(v)))
                    .collect();
                format!(" {{{}}}", prop_strings.join(", "))
            };

            format!("({labels}{props})")
        }
        FalkorValue::Edge(edge) => {
            let props = if edge.properties.is_empty() {
                String::new()
            } else {
                let prop_strings: Vec<String> = edge
                    .properties
                    .iter()
                    .map(|(k, v)| format!("{}: {}", k, format_falkor_value(v)))
                    .collect();
                format!(" {{{}}}", prop_strings.join(", "))
            };

            format!("-[:{}{props}]-", edge.relationship_type)
        }
        FalkorValue::Path(path) => {
            let mut path_str = String::new();
            for (i, node) in path.nodes.iter().enumerate() {
                if i > 0 {
                    if let Some(edge) = path.relationships.get(i - 1) {
                        path_str.push_str(&format_falkor_value(&FalkorValue::Edge(edge.clone())));
                    }
                }
                path_str.push_str(&format_falkor_value(&FalkorValue::Node(node.clone())));
            }
            path_str
        }
        FalkorValue::Array(arr) => {
            let elements: Vec<String> = arr.iter().map(format_falkor_value).collect();
            format!("[{}]", elements.join(", "))
        }
        _ => {
            // For all other types (strings, maps, etc.), use the debug representation
            // but clean it up for better readability
            let debug_str = format!("{value:?}");

            // If it's a string-like value, try to extract just the content
            if debug_str.starts_with("SimpleString(") && debug_str.ends_with(')') {
                let content = &debug_str[13..debug_str.len() - 1];
                format!("\"{}\"", content.trim_matches('"'))
            } else if debug_str.starts_with("BulkString(") && debug_str.ends_with(')') {
                let content = &debug_str[11..debug_str.len() - 1];
                format!("\"{}\"", content.trim_matches('"'))
            } else {
                debug_str
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use falkordb::{Edge, Node};
    use std::collections::HashMap;

    #[test]
    fn test_empty_records() {
        let records: Vec<Vec<FalkorValue>> = vec![];
        assert_eq!(format_query_records(&records), "No results returned.");
    }

    #[test]
    fn test_single_value() {
        let records = vec![vec![FalkorValue::I64(42)]];
        assert_eq!(format_query_records(&records), "42");
    }

    #[test]
    fn test_single_record_multiple_fields() {
        let records = vec![vec![FalkorValue::I64(42), FalkorValue::Bool(true), FalkorValue::F64(3.14)]];
        assert_eq!(format_query_records(&records), "[42, true, 3.14]");
    }

    #[test]
    fn test_multiple_records() {
        let records = vec![vec![FalkorValue::I64(1)], vec![FalkorValue::I64(2)]];
        let expected = "1. 1\n2. 2";
        assert_eq!(format_query_records(&records), expected);
    }

    #[test]
    fn test_node_formatting() {
        let mut properties = HashMap::new();
        properties.insert("name".to_string(), FalkorValue::I64(42)); // Using I64 for simplicity in test

        let node = Node {
            entity_id: 1,
            labels: vec!["Person".to_string()],
            properties,
        };

        let value = FalkorValue::Node(node);
        let formatted = format_falkor_value(&value);
        assert!(formatted.contains("(:Person"));
        assert!(formatted.contains("name: 42"));
    }

    #[test]
    fn test_edge_formatting() {
        let edge = Edge {
            entity_id: 1,
            relationship_type: "KNOWS".to_string(),
            src_node_id: 1,
            dst_node_id: 2,
            properties: HashMap::new(),
        };

        let value = FalkorValue::Edge(edge);
        let formatted = format_falkor_value(&value);
        assert_eq!(formatted, "-[:KNOWS]-");
    }

    #[test]
    fn test_array_formatting() {
        let array = vec![FalkorValue::I64(1), FalkorValue::I64(2), FalkorValue::I64(3)];

        let value = FalkorValue::Array(array);
        let formatted = format_falkor_value(&value);
        assert_eq!(formatted, "[1, 2, 3]");
    }
}
