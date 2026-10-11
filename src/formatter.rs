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

use falkordb::{
    FalkorAsyncClient, FalkorClientBuilder, FalkorConnectionInfo, FalkorResult, FalkorValue, RetryPolicy, RowStream,
};
use std::fmt::Write;

/// Builds an asynchronous `FalkorDB` client with the read-only retry policy applied.
///
/// Centralizing client construction here ensures every connection retries only idempotent
/// read operations (queries issued via `ro_query` and schema discovery) on transient failures,
/// using exponential backoff. Writes are never retried, so a failed write is surfaced
/// immediately and can never be duplicated.
///
/// Kept `pub(crate)` so `FalkorDB` types are not exposed in this crate's public API. It lives in
/// this module (compiled into both the library and the binary) so the binary can share it without
/// a public re-export.
///
/// # Errors
///
/// Returns an error if the client cannot be built (for example, when the connection fails).
// `pub(crate)` keeps FalkorDB types out of the public API in the library (this is a `pub mod`); it
// looks redundant only in the binary, where `formatter` is a private `mod`.
#[allow(clippy::redundant_pub_crate)]
pub(crate) async fn build_falkordb_async_client(
    connection_info: FalkorConnectionInfo
) -> FalkorResult<FalkorAsyncClient> {
    FalkorClientBuilder::new_async()
        .with_connection_info(connection_info)
        .with_retry_policy(RetryPolicy::read_only())
        .build()
        .await
}

/// Bridges a query result's rows back to the pre-0.7 `Vec<FalkorValue>` shape.
///
/// `falkordb` 0.7 made `QueryResult::data` header-aware (yielding `Row`) and 0.8 turned it into a
/// `Stream` on the async client. This shim restores the flat `Vec<Vec<FalkorValue>>` that the
/// formatter and schema-discovery code were written against (parse errors collapse to
/// `FalkorValue::Unparseable`), so the REST/JSON output is unchanged across the upgrade. It is the
/// single place that performs this conversion — the one thing to revisit when migrating to the
/// typed, header-aware `Row` API.
#[must_use]
pub fn rows_lossy(data: RowStream) -> Vec<Vec<FalkorValue>> {
    data.into_values_lossy().collect()
}

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
    serde_json::Value::Array(records.iter().map(|record| record_to_json(record)).collect()).to_string()
}

/// Formats a query result as a JSON object that also carries the column names:
/// `{"columns": [...], "data": [[...], ...]}`
#[must_use]
pub fn format_as_json_with_columns(
    columns: &[String],
    records: &[Vec<FalkorValue>],
) -> String {
    serde_json::json!({
        "columns": columns,
        "data": records.iter().map(|record| record_to_json(record)).collect::<Vec<_>>(),
    })
    .to_string()
}

fn record_to_json(record: &[FalkorValue]) -> serde_json::Value {
    serde_json::Value::Array(record.iter().map(falkor_value_to_json).collect())
}

fn properties_to_json(properties: &std::collections::HashMap<String, FalkorValue>) -> serde_json::Value {
    serde_json::Value::Object(properties.iter().map(|(k, v)| (k.clone(), falkor_value_to_json(v))).collect())
}

fn f64_to_json(f: f64) -> serde_json::Value {
    serde_json::Number::from_f64(f).map_or(serde_json::Value::Null, serde_json::Value::Number)
}

fn node_to_json(node: &falkordb::Node) -> serde_json::Value {
    serde_json::json!({
        "type": "node",
        "id": node.entity_id,
        "labels": node.labels,
        "properties": properties_to_json(&node.properties),
    })
}

fn edge_to_json(edge: &falkordb::Edge) -> serde_json::Value {
    serde_json::json!({
        "type": "edge",
        "id": edge.entity_id,
        "relationship_type": edge.relationship_type,
        "src_node_id": edge.src_node_id,
        "dst_node_id": edge.dst_node_id,
        "properties": properties_to_json(&edge.properties),
    })
}

/// Converts a `FalkorValue` to its JSON representation
fn falkor_value_to_json(value: &FalkorValue) -> serde_json::Value {
    use serde_json::Value;
    match value {
        FalkorValue::None => Value::Null,
        FalkorValue::Bool(b) => Value::Bool(*b),
        FalkorValue::I64(i) => Value::from(*i),
        FalkorValue::F64(f) => f64_to_json(*f),
        FalkorValue::String(s) | FalkorValue::Unparseable(s) => Value::String(s.clone()),
        FalkorValue::Node(node) => node_to_json(node),
        FalkorValue::Edge(edge) => edge_to_json(edge),
        FalkorValue::Path(path) => serde_json::json!({
            "type": "path",
            "nodes": path.nodes.iter().map(node_to_json).collect::<Vec<_>>(),
            "relationships": path.relationships.iter().map(edge_to_json).collect::<Vec<_>>(),
        }),
        FalkorValue::Array(arr) => Value::Array(arr.iter().map(falkor_value_to_json).collect()),
        FalkorValue::Map(map) => properties_to_json(map),
        FalkorValue::Vec32(vec) => Value::Array(vec.values.iter().map(|f| f64_to_json(f64::from(*f))).collect()),
        FalkorValue::Point(point) => serde_json::json!({
            "latitude": f64_to_json(point.latitude),
            "longitude": f64_to_json(point.longitude),
        }),
        FalkorValue::DateTime(v) => temporal_to_json(&v.to_string()),
        FalkorValue::Date(v) => temporal_to_json(&v.to_string()),
        FalkorValue::Time(v) => temporal_to_json(&v.to_string()),
        FalkorValue::Duration(v) => temporal_to_json(&v.to_string()),
        // `FalkorValue` is non-exhaustive: fall back to a string for any future variant
        other => Value::String(format!("{other:?}")),
    }
}

/// Temporal values are exposed by `falkordb` as a whole number of seconds.
fn temporal_to_json(seconds: &str) -> serde_json::Value {
    seconds.parse::<i64>().map_or_else(
        |_| serde_json::Value::String(seconds.to_string()),
        serde_json::Value::from,
    )
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

    // Customer report: `RETURN null AS x` and `RETURN "None" AS x` both came back as [["None"]]
    #[test]
    fn test_json_null_is_distinct_from_none_string() {
        let null_result = format_as_json(&[vec![FalkorValue::None]]);
        let none_string_result = format_as_json(&[vec![FalkorValue::String("None".to_string())]]);
        assert_eq!(null_result, "[[null]]");
        assert_eq!(none_string_result, "[[\"None\"]]");
        assert_ne!(null_result, none_string_result);
    }

    // Customer report: `RETURN {x: null, y: "None", z: 1} AS row` came back as
    // "Map({\"x\": None, \"y\": String(\"None\"), \"z\": I64(1)})"
    #[test]
    fn test_json_map_is_a_real_json_object() {
        let map = HashMap::from([
            ("x".to_string(), FalkorValue::None),
            ("y".to_string(), FalkorValue::String("None".to_string())),
            ("z".to_string(), FalkorValue::I64(1)),
        ]);
        let result = format_as_json(&[vec![FalkorValue::Map(map)]]);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed, serde_json::json!([[{"x": null, "y": "None", "z": 1}]]));
        assert!(!result.contains("Map("));
        assert!(!result.contains("I64("));
        assert!(!result.contains("String("));
    }

    #[test]
    fn test_json_nested_values_have_no_rust_debug_output() {
        let inner = HashMap::from([("n".to_string(), FalkorValue::None)]);
        let value = FalkorValue::Array(vec![
            FalkorValue::None,
            FalkorValue::Map(inner),
            FalkorValue::F64(1.5),
            FalkorValue::Bool(true),
        ]);
        let result = format_as_json(&[vec![value]]);
        let parsed: serde_json::Value = serde_json::from_str(&result).unwrap();
        assert_eq!(parsed, serde_json::json!([[[null, {"n": null}, 1.5, true]]]));
    }

    // Customer report: no column information, only positional arrays
    #[test]
    fn test_json_with_columns_includes_column_names() {
        let columns = vec!["name".to_string(), "age".to_string()];
        let records = vec![
            vec![FalkorValue::String("Alice".to_string()), FalkorValue::I64(30)],
            vec![FalkorValue::String("Bob".to_string()), FalkorValue::None],
        ];
        let parsed: serde_json::Value = serde_json::from_str(&format_as_json_with_columns(&columns, &records)).unwrap();
        assert_eq!(
            parsed,
            serde_json::json!({
                "columns": ["name", "age"],
                "data": [["Alice", 30], ["Bob", null]]
            })
        );
    }

    #[test]
    fn test_json_with_columns_empty_result_keeps_columns() {
        let columns = vec!["x".to_string()];
        let parsed: serde_json::Value = serde_json::from_str(&format_as_json_with_columns(&columns, &[])).unwrap();
        assert_eq!(parsed, serde_json::json!({"columns": ["x"], "data": []}));
    }

    #[test]
    fn test_json_point_is_an_object() {
        let point = falkordb::Point {
            latitude: 32.07,
            longitude: 34.78,
        };
        let parsed: serde_json::Value =
            serde_json::from_str(&format_as_json(&[vec![FalkorValue::Point(point)]])).unwrap();
        assert_eq!(parsed, serde_json::json!([[{"latitude": 32.07, "longitude": 34.78}]]));
    }

    #[test]
    fn test_json_temporal_values_are_seconds() {
        let records = vec![vec![
            FalkorValue::DateTime(falkordb::DateTime::new(1_700_000_000)),
            FalkorValue::Date(falkordb::Date::new(1_699_920_000)),
            FalkorValue::Time(falkordb::Time::new(3_600)),
            FalkorValue::Duration(falkordb::Duration::new(-60)),
        ]];
        let parsed: serde_json::Value = serde_json::from_str(&format_as_json(&records)).unwrap();
        assert_eq!(parsed, serde_json::json!([[1_700_000_000, 1_699_920_000, 3_600, -60]]));
    }

    #[test]
    fn test_json_non_finite_floats_become_null() {
        let records = vec![vec![
            FalkorValue::F64(f64::NAN),
            FalkorValue::F64(f64::INFINITY),
            FalkorValue::F64(f64::NEG_INFINITY),
        ]];
        let result = format_as_json(&records);
        assert_eq!(result, "[[null,null,null]]");
    }

    #[test]
    fn test_json_unparseable_value_is_a_string() {
        let result = format_as_json(&[vec![FalkorValue::Unparseable("raw".to_string())]]);
        assert_eq!(result, "[[\"raw\"]]");
    }
}
