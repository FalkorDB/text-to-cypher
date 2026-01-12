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
        FalkorValue::String(s) => format!("\"{s}\""),
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
                if i > 0
                    && let Some(edge) = path.relationships.get(i - 1)
                {
                    path_str.push_str(&format_falkor_value(&FalkorValue::Edge(edge.clone())));
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

/// Formats a query result as JSON for programmatic consumption
#[must_use]
pub fn format_as_json(records: &[Vec<FalkorValue>]) -> String {
    if records.is_empty() {
        return "[]".to_string();
    }

    let mut result = String::from("[");
    for (i, record) in records.iter().enumerate() {
        if i > 0 {
            result.push(',');
        }

        result.push('[');
        for (j, value) in record.iter().enumerate() {
            if j > 0 {
                result.push(',');
            }
            result.push_str(&falkor_value_to_json(value));
        }
        result.push(']');
    }
    result.push(']');

    result
}

/// Converts a `FalkorValue` to its JSON representation
fn falkor_value_to_json(value: &FalkorValue) -> String {
    match value {
        FalkorValue::Bool(b) => b.to_string(),
        FalkorValue::I64(i) => i.to_string(),
        FalkorValue::F64(f) => f.to_string(),
        FalkorValue::String(s) => format!("\"{}\"", escape_json_string(s)),
        FalkorValue::Node(node) => {
            let mut json = String::from("{\"type\":\"node\",\"id\":");
            json.push_str(&node.entity_id.to_string());

            json.push_str(",\"labels\":[");
            for (i, label) in node.labels.iter().enumerate() {
                if i > 0 {
                    json.push(',');
                }
                write!(json, "\"{}\"", escape_json_string(label)).unwrap();
            }
            json.push_str("],\"properties\":{");

            for (i, (k, v)) in node.properties.iter().enumerate() {
                if i > 0 {
                    json.push(',');
                }
                write!(json, "\"{}\":{}", escape_json_string(k), falkor_value_to_json(v)).unwrap();
            }
            json.push_str("}}");
            json
        }
        FalkorValue::Edge(edge) => {
            let mut json = String::from("{\"type\":\"edge\",\"id\":");
            json.push_str(&edge.entity_id.to_string());
            json.push_str(",\"relationship_type\":\"");
            json.push_str(&escape_json_string(&edge.relationship_type));
            json.push_str("\",\"src_node_id\":");
            json.push_str(&edge.src_node_id.to_string());
            json.push_str(",\"dst_node_id\":");
            json.push_str(&edge.dst_node_id.to_string());
            json.push_str(",\"properties\":{");

            for (i, (k, v)) in edge.properties.iter().enumerate() {
                if i > 0 {
                    json.push(',');
                }
                write!(json, "\"{}\":{}", escape_json_string(k), falkor_value_to_json(v)).unwrap();
            }
            json.push_str("}}");
            json
        }
        FalkorValue::Path(path) => {
            let mut json = String::from("{\"type\":\"path\",\"nodes\":[");
            for (i, node) in path.nodes.iter().enumerate() {
                if i > 0 {
                    json.push(',');
                }
                json.push_str(&falkor_value_to_json(&FalkorValue::Node(node.clone())));
            }
            json.push_str("],\"relationships\":[");
            for (i, edge) in path.relationships.iter().enumerate() {
                if i > 0 {
                    json.push(',');
                }
                json.push_str(&falkor_value_to_json(&FalkorValue::Edge(edge.clone())));
            }
            json.push_str("]}");
            json
        }
        FalkorValue::Array(arr) => {
            let mut json = String::from("[");
            for (i, item) in arr.iter().enumerate() {
                if i > 0 {
                    json.push(',');
                }
                json.push_str(&falkor_value_to_json(item));
            }
            json.push(']');
            json
        }
        _ => {
            // For other types, serialize as string representation
            let debug_str = format!("{value:?}");
            format!("\"{}\"", escape_json_string(&debug_str))
        }
    }
}

/// Escapes a string for JSON format
fn escape_json_string(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '"' => "\\\"".to_string(),
            '\\' => "\\\\".to_string(),
            '\n' => "\\n".to_string(),
            '\r' => "\\r".to_string(),
            '\t' => "\\t".to_string(),
            c if c.is_control() => format!("\\u{:04x}", c as u32),
            c => c.to_string(),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use falkordb::{Edge, Node};
    use std::collections::HashMap;

    #[test]
    fn test_string_formatting() {
        let value = FalkorValue::String("Hello, World!".to_string());
        assert_eq!(format_falkor_value(&value), "\"Hello, World!\"");
    }

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
        let records = vec![vec![FalkorValue::I64(42), FalkorValue::Bool(true), FalkorValue::F64(3.12)]];
        assert_eq!(format_query_records(&records), "[42, true, 3.12]");
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
