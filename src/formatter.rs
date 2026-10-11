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

/// Formats a map key the way Cypher writes it: bare when it is a valid identifier,
/// otherwise wrapped in backticks (with any backtick inside doubled).
fn format_map_key(key: &str) -> String {
    let mut chars = key.chars();
    let is_identifier =
        chars.next().is_some_and(|c| c.is_alphabetic() || c == '_') && chars.all(|c| c.is_alphanumeric() || c == '_');
    if is_identifier {
        key.to_string()
    } else {
        format!("`{}`", key.replace('`', "``"))
    }
}

/// Converts days since 1970-01-01 to a (year, month, day) civil date.
const fn civil_from_days(days: i64) -> (i64, i64, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let day = doy - (153 * mp + 2) / 5 + 1;
    let month = if mp < 10 { mp + 3 } else { mp - 9 };
    let year = yoe + era * 400 + if month <= 2 { 1 } else { 0 };
    (year, month, day)
}

/// Formats seconds since the Unix epoch as an ISO-8601 date (`2024-01-02`) or local
/// datetime (`2024-01-02T10:00:00`). Written without quotes or a constructor call so the
/// model reads it as a value, not as Cypher to copy back into a query.
fn format_epoch_seconds(
    seconds: i64,
    with_time: bool,
) -> String {
    let (year, month, day) = civil_from_days(seconds.div_euclid(86_400));
    if with_time {
        format!("{year:04}-{month:02}-{day:02}T{}", format_time_of_day(seconds))
    } else {
        format!("{year:04}-{month:02}-{day:02}")
    }
}

/// Formats the time-of-day part of a seconds value as `HH:MM:SS`.
fn format_time_of_day(seconds: i64) -> String {
    let secs = seconds.rem_euclid(86_400);
    format!("{:02}:{:02}:{:02}", secs / 3600, secs % 3600 / 60, secs % 60)
}

/// Formats a duration in seconds as ISO-8601 (`PT1H30M`, `P2DT5S`, `-PT60S`).
fn format_duration(seconds: i64) -> String {
    let sign = if seconds < 0 { "-" } else { "" };
    let mut rest = seconds.unsigned_abs();
    let days = rest / 86_400;
    rest %= 86_400;
    let (hours, minutes, secs) = (rest / 3600, rest % 3600 / 60, rest % 60);
    let mut out = format!("{sign}P");
    if days > 0 {
        write!(out, "{days}D").unwrap();
    }
    if hours > 0 || minutes > 0 || secs > 0 || days == 0 {
        out.push('T');
        if hours > 0 {
            write!(out, "{hours}H").unwrap();
        }
        if minutes > 0 {
            write!(out, "{minutes}M").unwrap();
        }
        if secs > 0 || (hours == 0 && minutes == 0) {
            write!(out, "{secs}S").unwrap();
        }
    }
    out
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
                    .map(|(k, v)| format!("{}: {}", format_map_key(k), format_falkor_value(v)))
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
                    .map(|(k, v)| format!("{}: {}", format_map_key(k), format_falkor_value(v)))
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
        FalkorValue::None => "null".to_string(),
        FalkorValue::Map(map) => {
            let mut entries: Vec<String> = map
                .iter()
                .map(|(k, v)| format!("{}: {}", format_map_key(k), format_falkor_value(v)))
                .collect();
            entries.sort();
            format!("{{{}}}", entries.join(", "))
        }
        FalkorValue::Point(point) => format!(
            "point({{latitude: {}, longitude: {}}})",
            point.latitude, point.longitude
        ),
        FalkorValue::Vec32(vec) => {
            let elements: Vec<String> = vec.values.iter().map(ToString::to_string).collect();
            format!("[{}]", elements.join(", "))
        }
        FalkorValue::DateTime(v) => format_epoch_seconds(v.seconds().get(), true),
        FalkorValue::Date(v) => format_epoch_seconds(v.seconds().get(), false),
        FalkorValue::Time(v) => format_time_of_day(v.seconds().get()),
        FalkorValue::Duration(v) => format_duration(v.seconds().get()),
        FalkorValue::Unparseable(s) => format!("<unparseable: {s}>"),
        _ => {
            // For any other types, use the debug representation
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

/// Converts one result row into a JSON array of values.
fn record_to_json(record: &[FalkorValue]) -> serde_json::Value {
    serde_json::Value::Array(record.iter().map(falkor_value_to_json).collect())
}

/// Converts a property or map collection into a JSON object.
fn properties_to_json(properties: &std::collections::HashMap<String, FalkorValue>) -> serde_json::Value {
    serde_json::Value::Object(properties.iter().map(|(k, v)| (k.clone(), falkor_value_to_json(v))).collect())
}

/// Converts a float to a JSON number.
///
/// JSON has no representation for NaN or infinity, so those are emitted as the strings
/// `"NaN"`, `"Infinity"` and `"-Infinity"` to keep them distinguishable from `null`.
fn f64_to_json(f: f64) -> serde_json::Value {
    serde_json::Number::from_f64(f).map_or_else(
        || {
            let text = if f.is_nan() {
                "NaN"
            } else if f.is_sign_positive() {
                "Infinity"
            } else {
                "-Infinity"
            };
            serde_json::Value::String(text.to_string())
        },
        serde_json::Value::Number,
    )
}

/// Converts a 32-bit float (used by vectors) to a JSON number.
///
/// Widening `f32` to `f64` directly exposes binary rounding noise (`0.1` becomes
/// `0.10000000149011612`), so the value goes through its shortest decimal form first.
#[must_use]
fn f32_to_json(f: f32) -> serde_json::Value {
    f.to_string()
        .parse::<f64>()
        .map_or_else(|_| f64_to_json(f64::from(f)), f64_to_json)
}

/// Converts a node into a JSON object tagged with `"type": "node"`.
fn node_to_json(node: &falkordb::Node) -> serde_json::Value {
    serde_json::json!({
        "type": "node",
        "id": node.entity_id,
        "labels": node.labels,
        "properties": properties_to_json(&node.properties),
    })
}

/// Converts an edge into a JSON object tagged with `"type": "edge"`.
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
        FalkorValue::String(s) => Value::String(s.clone()),
        FalkorValue::Unparseable(error) => serde_json::json!({ "type": "unparseable", "error": error }),
        FalkorValue::Node(node) => node_to_json(node),
        FalkorValue::Edge(edge) => edge_to_json(edge),
        FalkorValue::Path(path) => serde_json::json!({
            "type": "path",
            "nodes": path.nodes.iter().map(node_to_json).collect::<Vec<_>>(),
            "relationships": path.relationships.iter().map(edge_to_json).collect::<Vec<_>>(),
        }),
        FalkorValue::Array(arr) => Value::Array(arr.iter().map(falkor_value_to_json).collect()),
        FalkorValue::Map(map) => properties_to_json(map),
        FalkorValue::Vec32(vec) => Value::Array(vec.values.iter().map(|f| f32_to_json(*f)).collect()),
        FalkorValue::Point(point) => serde_json::json!({
            "latitude": f64_to_json(point.latitude),
            "longitude": f64_to_json(point.longitude),
        }),
        FalkorValue::DateTime(v) => temporal_to_json("datetime", &v.to_string()),
        FalkorValue::Date(v) => temporal_to_json("date", &v.to_string()),
        FalkorValue::Time(v) => temporal_to_json("time", &v.to_string()),
        FalkorValue::Duration(v) => temporal_to_json("duration", &v.to_string()),
        // `FalkorValue` is non-exhaustive: fall back to a string for any future variant
        other => Value::String(format!("{other:?}")),
    }
}

/// Converts a temporal value into a JSON object tagged with its type.
///
/// `falkordb` exposes temporal values as a whole number of seconds, so the output is
/// `{"type": "<kind>", "seconds": <n>}`; the tag keeps a datetime, date, time or duration
/// distinguishable from a plain integer and from each other.
fn temporal_to_json(
    kind: &str,
    seconds: &str,
) -> serde_json::Value {
    let seconds = seconds.parse::<i64>().map_or_else(
        |_| serde_json::Value::String(seconds.to_string()),
        serde_json::Value::from,
    );
    serde_json::json!({ "type": kind, "seconds": seconds })
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

    /// Customer report: `RETURN null AS x` and `RETURN "None" AS x` both came back as `[["None"]]`.
    #[test]
    fn test_json_null_is_distinct_from_none_string() {
        let null_result = format_as_json(&[vec![FalkorValue::None]]);
        let none_string_result = format_as_json(&[vec![FalkorValue::String("None".to_string())]]);
        assert_eq!(null_result, "[[null]]");
        assert_eq!(none_string_result, "[[\"None\"]]");
        assert_ne!(null_result, none_string_result);
    }

    /// Customer report: `RETURN {x: null, y: "None", z: 1} AS row` came back as
    /// `"Map({\"x\": None, \"y\": String(\"None\"), \"z\": I64(1)})"`.
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

    /// Nested lists and maps are serialized recursively as JSON, without Rust Debug output.
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

    /// Customer report: no column information, only positional arrays.
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

    /// An empty result still reports its column names.
    #[test]
    fn test_json_with_columns_empty_result_keeps_columns() {
        let columns = vec!["x".to_string()];
        let parsed: serde_json::Value = serde_json::from_str(&format_as_json_with_columns(&columns, &[])).unwrap();
        assert_eq!(parsed, serde_json::json!({"columns": ["x"], "data": []}));
    }

    /// Points are serialized as `{"latitude": .., "longitude": ..}`.
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

    /// Temporal values are tagged with their type so they are not confused with plain integers.
    #[test]
    fn test_json_temporal_values_are_tagged() {
        let records = vec![vec![
            FalkorValue::DateTime(falkordb::DateTime::new(1_700_000_000)),
            FalkorValue::Date(falkordb::Date::new(1_699_920_000)),
            FalkorValue::Time(falkordb::Time::new(3_600)),
            FalkorValue::Duration(falkordb::Duration::new(-60)),
            FalkorValue::I64(3_600),
        ]];
        let parsed: serde_json::Value = serde_json::from_str(&format_as_json(&records)).unwrap();
        assert_eq!(
            parsed,
            serde_json::json!([[
                {"type": "datetime", "seconds": 1_700_000_000},
                {"type": "date", "seconds": 1_699_920_000},
                {"type": "time", "seconds": 3_600},
                {"type": "duration", "seconds": -60},
                3_600
            ]])
        );
    }

    /// NaN and infinity are emitted as strings so they stay distinguishable from `null`.
    #[test]
    fn test_json_non_finite_floats_are_distinguishable_from_null() {
        let records = vec![vec![
            FalkorValue::F64(f64::NAN),
            FalkorValue::F64(f64::INFINITY),
            FalkorValue::F64(f64::NEG_INFINITY),
            FalkorValue::None,
        ]];
        let result = format_as_json(&records);
        assert_eq!(result, "[[\"NaN\",\"Infinity\",\"-Infinity\",null]]");
    }

    /// Values the client could not parse are tagged so they are not mistaken for real strings.
    #[test]
    fn test_json_unparseable_value_is_tagged() {
        let records = vec![vec![
            FalkorValue::Unparseable("raw".to_string()),
            FalkorValue::String("raw".to_string()),
        ]];
        let parsed: serde_json::Value = serde_json::from_str(&format_as_json(&records)).unwrap();
        assert_eq!(
            parsed,
            serde_json::json!([[{"type": "unparseable", "error": "raw"}, "raw"]])
        );
    }

    /// Vector values keep their short decimal form instead of `f32` to `f64` rounding noise.
    #[test]
    fn test_json_vector_has_no_rounding_noise() {
        assert_eq!(f32_to_json(0.1), serde_json::json!(0.1));
        assert_eq!(f32_to_json(0.2), serde_json::json!(0.2));
        assert_eq!(f32_to_json(1.5), serde_json::json!(1.5));
        assert_eq!(f32_to_json(f32::NAN), serde_json::json!("NaN"));
    }

    /// The LLM-friendly formatter shows `null` instead of Rust's `None`.
    #[test]
    fn test_null_formatting() {
        assert_eq!(format_falkor_value(&FalkorValue::None), "null");
        assert_eq!(
            format_falkor_value(&FalkorValue::String("None".to_string())),
            "\"None\""
        );
    }

    /// The LLM-friendly formatter shows maps in Cypher map syntax, without Rust Debug output.
    #[test]
    fn test_map_formatting() {
        let map = HashMap::from([
            ("x".to_string(), FalkorValue::None),
            ("y".to_string(), FalkorValue::String("None".to_string())),
            ("z".to_string(), FalkorValue::I64(1)),
        ]);
        assert_eq!(
            format_falkor_value(&FalkorValue::Map(map)),
            "{x: null, y: \"None\", z: 1}"
        );
    }

    /// Points, temporal values and unparseable values are readable in the LLM-friendly format.
    #[test]
    fn test_other_value_formatting() {
        let point = falkordb::Point {
            latitude: 1.5,
            longitude: 2.0,
        };
        assert_eq!(
            format_falkor_value(&FalkorValue::Point(point)),
            "point({latitude: 1.5, longitude: 2})"
        );
        assert_eq!(
            format_falkor_value(&FalkorValue::Date(falkordb::Date::new(86_400))),
            "1970-01-02"
        );
        assert_eq!(
            format_falkor_value(&FalkorValue::Unparseable("bad".to_string())),
            "<unparseable: bad>"
        );
    }

    /// Map keys that are not valid identifiers are wrapped in backticks, as in Cypher.
    #[test]
    fn test_map_key_escaping() {
        let map = HashMap::from([
            ("first name".to_string(), FalkorValue::String("Ann".to_string())),
            ("age".to_string(), FalkorValue::I64(30)),
            ("a`b".to_string(), FalkorValue::None),
        ]);
        assert_eq!(
            format_falkor_value(&FalkorValue::Map(map)),
            "{`a``b`: null, `first name`: \"Ann\", age: 30}"
        );
    }

    /// Node and edge property keys are escaped the same way as map keys.
    #[test]
    fn test_node_and_edge_property_key_escaping() {
        let node = Node {
            entity_id: 1,
            labels: vec!["Person".to_string()],
            properties: HashMap::from([("first name".to_string(), FalkorValue::String("Ann".to_string()))]),
        };
        assert_eq!(
            format_falkor_value(&FalkorValue::Node(node)),
            "(:Person {`first name`: \"Ann\"})"
        );
        let edge = Edge {
            entity_id: 1,
            relationship_type: "KNOWS".to_string(),
            src_node_id: 1,
            dst_node_id: 2,
            properties: HashMap::from([("since year".to_string(), FalkorValue::I64(2020))]),
        };
        assert_eq!(
            format_falkor_value(&FalkorValue::Edge(edge)),
            "-[:KNOWS {`since year`: 2020}]-"
        );
    }

    /// Temporal values are shown as ISO-8601 values, not as constructor calls.
    #[test]
    fn test_temporal_formatting_is_iso() {
        assert_eq!(
            format_falkor_value(&FalkorValue::DateTime(falkordb::DateTime::new(1_700_000_000))),
            "2023-11-14T22:13:20"
        );
        assert_eq!(
            format_falkor_value(&FalkorValue::Date(falkordb::Date::new(1_704_153_600))),
            "2024-01-02"
        );
        // FalkorDB stores a time of day on 1900-01-01: localtime('10:00:00') is -2208952800
        assert_eq!(
            format_falkor_value(&FalkorValue::Time(falkordb::Time::new(-2_208_952_800))),
            "10:00:00"
        );
        assert_eq!(
            format_falkor_value(&FalkorValue::Duration(falkordb::Duration::new(3600))),
            "PT1H"
        );
        assert_eq!(
            format_falkor_value(&FalkorValue::Duration(falkordb::Duration::new(90_061))),
            "P1DT1H1M1S"
        );
        assert_eq!(
            format_falkor_value(&FalkorValue::Duration(falkordb::Duration::new(0))),
            "PT0S"
        );
        assert_eq!(
            format_falkor_value(&FalkorValue::Duration(falkordb::Duration::new(-60))),
            "-PT1M"
        );
        assert_eq!(
            format_falkor_value(&FalkorValue::Date(falkordb::Date::new(-86_400))),
            "1969-12-31"
        );
    }
}
