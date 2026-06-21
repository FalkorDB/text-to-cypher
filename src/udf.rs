//! User-defined function (UDF) context for the LLM.
//!
//! `FalkorDB` instances can host JavaScript UDFs (managed via the `GRAPH.UDF` command). When
//! present, this module surfaces the available `library.function` call targets to the model so
//! generated Cypher can use them.
//!
//! UDFs are **instance-global** (not graph-scoped): `GRAPH.UDF LIST` takes no graph name, so a
//! discovered catalog reflects every library loaded on the connected server.
//!
//! Discovery is **opt-in** ([`UdfSource`]). The server-side UDF feature is not yet in a stable
//! `FalkorDB` release, so [`UdfCatalog::discover`] degrades to [`UdfError::Unsupported`] (an empty
//! catalog) on servers that do not recognize `GRAPH.UDF LIST`, rather than failing the request.

use falkordb::{FalkorAsyncClient, FalkorDBError};

/// A single user-defined function exposed to the model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdfFunction {
    /// Function name as registered in its library; called as `library.name(...)`.
    pub name: String,
    /// Optional signature hint (e.g. `"(x, y)"`). Discovery leaves this `None`
    /// (`FalkorDB` does not expose signatures); caller-supplied catalogs may populate it.
    pub signature_hint: Option<String>,
    /// Optional human-readable description. Discovery leaves this `None`.
    pub description: Option<String>,
}

impl UdfFunction {
    /// Create a names-only function entry (no signature/description).
    #[must_use]
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            signature_hint: None,
            description: None,
        }
    }
}

/// A UDF library: a named namespace grouping one or more functions.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UdfLibrary {
    /// Library name; the left side of a `library.function(...)` call.
    pub name: String,
    /// Functions registered in this library.
    pub functions: Vec<UdfFunction>,
}

/// A catalog of UDF libraries available on a `FalkorDB` instance.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct UdfCatalog {
    libraries: Vec<UdfLibrary>,
}

/// Why UDF discovery produced no catalog.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UdfError {
    /// The server does not support the `GRAPH.UDF` command (older `FalkorDB`). Treated as "no UDFs".
    Unsupported,
    /// A genuine transport/parse failure (connection, auth, protocol). The message is preserved.
    Transport(String),
}

/// Whether (and how) a client surfaces UDF context to the model.
///
/// The default is [`UdfSource::Off`]: UDF context is opt-in, because the server-side UDF feature
/// is not yet in a stable `FalkorDB` release.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum UdfSource {
    /// No UDF context (default).
    #[default]
    Off,
    /// Discover UDFs from the connected `FalkorDB` instance on each request, degrading to an empty
    /// catalog when the server does not support `GRAPH.UDF LIST`.
    Discover,
    /// Use a caller-supplied catalog (e.g. cached, or for `cypher_only` without a live database).
    Provided(UdfCatalog),
}

impl UdfCatalog {
    /// Create an empty catalog.
    #[must_use]
    pub const fn empty() -> Self {
        Self { libraries: Vec::new() }
    }

    /// Build a catalog from explicit libraries (e.g. caller-supplied UDF metadata).
    #[must_use]
    pub const fn from_libraries(libraries: Vec<UdfLibrary>) -> Self {
        Self { libraries }
    }

    /// The libraries in this catalog.
    #[must_use]
    pub fn libraries(&self) -> &[UdfLibrary] {
        &self.libraries
    }

    /// Whether the catalog exposes no callable functions.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.libraries.iter().all(|lib| lib.functions.is_empty())
    }

    /// Parse the reply of `GRAPH.UDF LIST` into a catalog.
    ///
    /// Tolerant of both RESP2 (each library is a flat array of alternating key/value entries) and
    /// RESP3 (each library is a map), and of missing/extra fields. Anything unrecognized yields an
    /// empty catalog rather than an error. The optional `library_code` field (returned only with
    /// `WITHCODE`) is intentionally ignored — the JS source is never stored or rendered.
    #[must_use]
    pub fn parse_redis_value(value: &redis::Value) -> Self {
        let entries: &[redis::Value] = match value {
            redis::Value::Array(items) | redis::Value::Set(items) => items,
            // A single library returned as a bare map (RESP3): treat as one entry.
            redis::Value::Map(_) => std::slice::from_ref(value),
            _ => &[],
        };
        let libraries = entries.iter().filter_map(Self::parse_library).collect();
        Self { libraries }
    }

    /// Parse one library entry (RESP2 array or RESP3 map). Returns `None` without a library name.
    fn parse_library(entry: &redis::Value) -> Option<UdfLibrary> {
        let mut name: Option<String> = None;
        let mut functions: Vec<UdfFunction> = Vec::new();

        for (key, val) in Self::key_value_pairs(entry) {
            match key.as_str() {
                "library_name" => name = redis_string(val),
                "functions" => {
                    if let redis::Value::Array(items) | redis::Value::Set(items) = val {
                        functions = items.iter().filter_map(redis_string).map(UdfFunction::new).collect();
                    }
                }
                // "library_code" (WITHCODE) intentionally ignored — never store/render JS source.
                _ => {}
            }
        }

        name.map(|name| UdfLibrary { name, functions })
    }

    /// Yield `(key, value)` pairs from a library entry in either RESP3 map or RESP2 flat-array form.
    fn key_value_pairs(entry: &redis::Value) -> Vec<(String, &redis::Value)> {
        match entry {
            redis::Value::Map(pairs) => pairs.iter().filter_map(|(k, v)| redis_string(k).map(|k| (k, v))).collect(),
            redis::Value::Array(items) | redis::Value::Set(items) => items
                .chunks_exact(2)
                .filter_map(|pair| redis_string(&pair[0]).map(|k| (k, &pair[1])))
                .collect(),
            _ => Vec::new(),
        }
    }

    /// Render a compact prompt block listing `library.function` call targets.
    ///
    /// Returns an empty string for an empty catalog so the prompt placeholder collapses cleanly.
    /// The block constrains the model to the listed functions. Discovered catalogs are names-only and
    /// note that signatures are unknown; caller-supplied catalogs may add inline signatures and
    /// descriptions, in which case that note is omitted.
    #[must_use]
    pub fn render(&self) -> String {
        if self.is_empty() {
            return String::new();
        }

        let has_signatures = self
            .libraries
            .iter()
            .flat_map(|library| &library.functions)
            .any(|function| function.signature_hint.is_some());

        let mut lines = vec![
            "Available User-Defined Functions on this FalkorDB instance.".to_string(),
            "Call them as library.function(...) inside RETURN/WHERE clauses.".to_string(),
        ];
        if !has_signatures {
            lines.push(
                "Signatures are not provided; infer arguments from the question and do not assume a fixed arity."
                    .to_string(),
            );
        }
        lines.push(
            "Use ONLY the functions listed below; never invent a UDF. If none is clearly relevant to the question, write normal Cypher."
                .to_string(),
        );

        let mut libraries: Vec<&UdfLibrary> = self.libraries.iter().filter(|lib| !lib.functions.is_empty()).collect();
        libraries.sort_by(|a, b| a.name.cmp(&b.name));

        for library in libraries {
            let mut functions: Vec<&UdfFunction> = library.functions.iter().collect();
            functions.sort_by(|a, b| a.name.cmp(&b.name));
            for function in functions {
                let mut line = format!("- {}.{}", library.name, function.name);
                if let Some(signature) = &function.signature_hint {
                    line.push(' ');
                    line.push_str(signature);
                }
                if let Some(description) = &function.description {
                    line.push_str(" — ");
                    line.push_str(description);
                }
                lines.push(line);
            }
        }

        lines.join("\n")
    }

    /// Discover UDFs from a connected `FalkorDB` instance via `GRAPH.UDF LIST`.
    ///
    /// Uses names only (`WITHCODE` is not requested), so no JavaScript source crosses into the
    /// catalog.
    ///
    /// # Errors
    ///
    /// Returns [`UdfError::Unsupported`] when the server does not recognize `GRAPH.UDF` (older
    /// `FalkorDB`) — callers should treat this as "no UDFs" — and [`UdfError::Transport`] for genuine
    /// connection/protocol failures.
    pub async fn discover(client: &FalkorAsyncClient) -> Result<Self, UdfError> {
        match client.udf_list(None, false).await {
            Ok(value) => Ok(Self::parse_redis_value(&value)),
            Err(error) => Err(classify_udf_error(&error)),
        }
    }
}

/// Convert a redis string-ish value to a `String` (UTF-8 lossy for bulk strings).
fn redis_string(value: &redis::Value) -> Option<String> {
    match value {
        redis::Value::BulkString(bytes) => Some(String::from_utf8_lossy(bytes).into_owned()),
        redis::Value::SimpleString(text) | redis::Value::VerbatimString { text, .. } => Some(text.clone()),
        _ => None,
    }
}

/// Classify a `udf_list` error.
///
/// An unknown `GRAPH.UDF` command or subcommand (an older `FalkorDB` without UDF support) maps to
/// [`UdfError::Unsupported`]; everything else (connection, auth, protocol) maps to
/// [`UdfError::Transport`].
#[must_use]
pub fn classify_udf_error(error: &FalkorDBError) -> UdfError {
    let message = error.to_string().to_lowercase();
    if message.contains("unknown command")
        || message.contains("unknown subcommand")
        || message.contains("unknown sub command")
    {
        UdfError::Unsupported
    } else {
        UdfError::Transport(error.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn bulk(s: &str) -> redis::Value {
        redis::Value::BulkString(s.as_bytes().to_vec())
    }

    /// RESP2 library entry: flat array of alternating key/value, optionally with code.
    fn resp2_library(
        name: &str,
        functions: &[&str],
        code: Option<&str>,
    ) -> redis::Value {
        let mut items = vec![
            bulk("library_name"),
            bulk(name),
            bulk("functions"),
            redis::Value::Array(functions.iter().map(|f| bulk(f)).collect()),
        ];
        if let Some(code) = code {
            items.push(bulk("library_code"));
            items.push(bulk(code));
        }
        redis::Value::Array(items)
    }

    #[test]
    fn udf_function_new_is_names_only() {
        let function = UdfFunction::new("Foo");
        assert_eq!(function.name, "Foo");
        assert!(function.signature_hint.is_none());
        assert!(function.description.is_none());
    }

    #[test]
    fn empty_and_from_libraries() {
        assert!(UdfCatalog::empty().is_empty());
        assert!(UdfCatalog::empty().libraries().is_empty());

        let catalog = UdfCatalog::from_libraries(vec![UdfLibrary {
            name: "lib".to_string(),
            functions: vec![UdfFunction::new("Foo")],
        }]);
        assert!(!catalog.is_empty());
        assert_eq!(catalog.libraries().len(), 1);
    }

    #[test]
    fn is_empty_when_library_has_no_functions() {
        let catalog = UdfCatalog::from_libraries(vec![UdfLibrary {
            name: "lib".to_string(),
            functions: vec![],
        }]);
        assert!(catalog.is_empty());
    }

    #[test]
    fn parse_resp2_single_library_no_code() {
        let reply = redis::Value::Array(vec![resp2_library("mylib", &["Foo", "Bar"], None)]);
        let catalog = UdfCatalog::parse_redis_value(&reply);

        assert_eq!(catalog.libraries().len(), 1);
        let lib = &catalog.libraries()[0];
        assert_eq!(lib.name, "mylib");
        assert_eq!(lib.functions, vec![UdfFunction::new("Foo"), UdfFunction::new("Bar")]);
    }

    #[test]
    fn parse_resp2_with_code_ignores_source() {
        let reply = redis::Value::Array(vec![resp2_library(
            "mylib",
            &["Foo"],
            Some("function Foo() { return 1; }"),
        )]);
        let catalog = UdfCatalog::parse_redis_value(&reply);

        let lib = &catalog.libraries()[0];
        assert_eq!(lib.name, "mylib");
        assert_eq!(lib.functions, vec![UdfFunction::new("Foo")]);
        // The catalog never stores the JS source anywhere.
        assert!(!catalog.render().contains("function Foo"));
    }

    #[test]
    fn parse_resp2_multiple_libraries() {
        let reply = redis::Value::Array(vec![
            resp2_library("liba", &["A1"], None),
            resp2_library("libb", &["B1", "B2"], None),
        ]);
        let catalog = UdfCatalog::parse_redis_value(&reply);
        assert_eq!(catalog.libraries().len(), 2);
    }

    #[test]
    fn parse_resp3_map_entries() {
        let library = redis::Value::Map(vec![
            (bulk("library_name"), bulk("mylib")),
            (bulk("functions"), redis::Value::Array(vec![bulk("Foo")])),
        ]);
        // RESP3: array of maps.
        let reply = redis::Value::Array(vec![library.clone()]);
        let catalog = UdfCatalog::parse_redis_value(&reply);
        assert_eq!(catalog.libraries().len(), 1);
        assert_eq!(catalog.libraries()[0].name, "mylib");

        // A bare map (single library, not wrapped in an array) is also accepted.
        let bare = UdfCatalog::parse_redis_value(&library);
        assert_eq!(bare.libraries().len(), 1);
    }

    #[test]
    fn parse_set_container_and_simple_strings() {
        // Top-level Set, SimpleString keys/values, functions as a Set.
        let library = redis::Value::Array(vec![
            redis::Value::SimpleString("library_name".to_string()),
            redis::Value::SimpleString("mylib".to_string()),
            redis::Value::SimpleString("functions".to_string()),
            redis::Value::Set(vec![redis::Value::SimpleString("Foo".to_string())]),
        ]);
        let catalog = UdfCatalog::parse_redis_value(&redis::Value::Set(vec![library]));
        assert_eq!(catalog.libraries()[0].name, "mylib");
        assert_eq!(catalog.libraries()[0].functions, vec![UdfFunction::new("Foo")]);
    }

    #[test]
    fn parse_empty_and_malformed_yields_empty_catalog() {
        assert!(UdfCatalog::parse_redis_value(&redis::Value::Nil).is_empty());
        assert!(UdfCatalog::parse_redis_value(&redis::Value::Int(7)).is_empty());
        assert!(UdfCatalog::parse_redis_value(&redis::Value::Array(vec![])).is_empty());
        // Entry without a library_name is skipped.
        let nameless = redis::Value::Array(vec![redis::Value::Array(vec![
            bulk("functions"),
            redis::Value::Array(vec![bulk("Foo")]),
        ])]);
        assert!(UdfCatalog::parse_redis_value(&nameless).is_empty());
        // Odd-length flat array (dangling key) parses what it can without panicking.
        let odd = redis::Value::Array(vec![redis::Value::Array(vec![
            bulk("library_name"),
            bulk("mylib"),
            bulk("functions"),
        ])]);
        let catalog = UdfCatalog::parse_redis_value(&odd);
        assert_eq!(catalog.libraries()[0].name, "mylib");
        assert!(catalog.libraries()[0].functions.is_empty());

        // A top-level entry that is neither a map nor an array contributes nothing.
        let non_entry = redis::Value::Array(vec![redis::Value::Int(5)]);
        assert!(UdfCatalog::parse_redis_value(&non_entry).is_empty());
    }

    #[test]
    fn render_empty_is_blank() {
        assert_eq!(UdfCatalog::empty().render(), "");
        assert_eq!(
            UdfCatalog::from_libraries(vec![UdfLibrary {
                name: "lib".to_string(),
                functions: vec![]
            }])
            .render(),
            ""
        );
    }

    #[test]
    fn render_lists_sorted_call_targets_with_guardrails() {
        let catalog = UdfCatalog::from_libraries(vec![
            UdfLibrary {
                name: "zlib".to_string(),
                functions: vec![UdfFunction::new("Z")],
            },
            UdfLibrary {
                name: "alib".to_string(),
                functions: vec![UdfFunction::new("B"), UdfFunction::new("A")],
            },
        ]);
        let rendered = catalog.render();

        assert!(rendered.contains("Use ONLY the functions listed below; never invent a UDF."));
        assert!(rendered.contains("Signatures are not provided"));
        // Libraries and functions are sorted; alib before zlib, A before B.
        let a_pos = rendered.find("- alib.A").unwrap();
        let b_pos = rendered.find("- alib.B").unwrap();
        let z_pos = rendered.find("- zlib.Z").unwrap();
        assert!(a_pos < b_pos && b_pos < z_pos);
        // An empty library contributes no lines.
        assert!(!rendered.contains("emptylib"));
    }

    #[test]
    fn render_includes_signature_and_description_when_present() {
        let catalog = UdfCatalog::from_libraries(vec![UdfLibrary {
            name: "lib".to_string(),
            functions: vec![
                UdfFunction {
                    name: "Sig".to_string(),
                    signature_hint: Some("(x, y)".to_string()),
                    description: None,
                },
                UdfFunction {
                    name: "Desc".to_string(),
                    signature_hint: None,
                    description: Some("does a thing".to_string()),
                },
            ],
        }]);
        let rendered = catalog.render();
        assert!(rendered.contains("- lib.Desc — does a thing"));
        assert!(rendered.contains("- lib.Sig (x, y)"));
        // When a signature is present the "signatures unknown" guardrail is omitted.
        assert!(!rendered.contains("Signatures are not provided"));
    }

    #[test]
    fn redis_string_handles_string_variants_only() {
        assert_eq!(redis_string(&bulk("a")).as_deref(), Some("a"));
        assert_eq!(
            redis_string(&redis::Value::SimpleString("b".to_string())).as_deref(),
            Some("b")
        );
        assert_eq!(
            redis_string(&redis::Value::VerbatimString {
                format: redis::VerbatimFormat::Text,
                text: "c".to_string(),
            })
            .as_deref(),
            Some("c")
        );
        assert!(redis_string(&redis::Value::Int(1)).is_none());
    }

    #[test]
    fn classify_unknown_command_is_unsupported() {
        let error = FalkorDBError::RedisError(
            "An error was signalled by the server: ERR unknown command 'GRAPH.UDF'".to_string(),
        );
        assert_eq!(classify_udf_error(&error), UdfError::Unsupported);

        let subcommand = FalkorDBError::RedisError("ERR Unknown subcommand 'LIST'".to_string());
        assert_eq!(classify_udf_error(&subcommand), UdfError::Unsupported);

        let spaced = FalkorDBError::RedisError("ERR unknown sub command".to_string());
        assert_eq!(classify_udf_error(&spaced), UdfError::Unsupported);
    }

    #[test]
    fn classify_other_errors_are_transport() {
        let result = classify_udf_error(&FalkorDBError::ConnectionDown);
        assert!(matches!(&result, UdfError::Transport(message) if !message.is_empty()));
    }

    #[test]
    fn udf_source_default_is_off() {
        assert_eq!(UdfSource::default(), UdfSource::Off);
    }
}
