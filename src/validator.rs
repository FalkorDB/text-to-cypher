use regex::Regex;
use std::sync::OnceLock;

/// Validates Cypher queries for common syntax errors and security issues
pub struct CypherValidator;

static PATTERNS: OnceLock<ValidationPatterns> = OnceLock::new();

struct ValidationPatterns {
    /// Pattern to detect basic Cypher syntax
    basic_cypher: Regex,
    /// Pattern to detect dangerous operations - matches DROP and various DELETE patterns
    dangerous_ops: Regex,
    /// Pattern to check for balanced parentheses
    match_clause: Regex,
    /// Pattern to check return clause exists
    return_clause: Regex,
}

impl ValidationPatterns {
    fn get() -> &'static Self {
        PATTERNS.get_or_init(|| Self {
            basic_cypher: Regex::new(r"(?i)(MATCH|CREATE|MERGE|DELETE|SET|REMOVE|RETURN|WITH|UNWIND|CALL)").unwrap(),
            // Matches the DROP or DELETE keyword (DETACH DELETE included) as a whole word.
            // Applied to the query with string literals, quoted identifiers and comments
            // masked out, so data such as 'Drop off package' is not mistaken for a clause.
            dangerous_ops: Regex::new(r"(?i)\b(DROP|DELETE)\b").unwrap(),
            match_clause: Regex::new(r"(?i)MATCH\s+").unwrap(),
            return_clause: Regex::new(r"(?i)RETURN\s+").unwrap(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct ValidationResult {
    pub is_valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
}

impl CypherValidator {
    /// Validates a Cypher query for syntax and safety
    ///
    /// # Arguments
    ///
    /// * `query` - The Cypher query to validate
    ///
    /// # Returns
    ///
    /// A `ValidationResult` containing validation status and any errors/warnings
    #[must_use]
    pub fn validate(query: &str) -> ValidationResult {
        let mut errors = Vec::new();
        let mut warnings = Vec::new();

        let query = query.trim();
        let patterns = ValidationPatterns::get();

        // Check if query is empty
        if query.is_empty() {
            errors.push("Query is empty".to_string());
            return ValidationResult {
                is_valid: false,
                errors,
                warnings,
            };
        }

        // Check if query contains basic Cypher keywords
        if !patterns.basic_cypher.is_match(query) {
            errors.push("Query does not contain valid Cypher keywords".to_string());
        }

        // Check for dangerous operations
        if Self::contains_dangerous_clause(&patterns.dangerous_ops, &Self::mask_literals_and_comments(query)) {
            errors.push("Query contains potentially dangerous operations (DROP, DELETE ALL)".to_string());
        }

        // Check for MATCH clause (most queries should have one)
        // Allow queries that start with other valid statements that don't require MATCH
        let query_upper = query.to_uppercase();
        let starts_with_non_match = query_upper.starts_with("CREATE")
            || query_upper.starts_with("MERGE")
            || query_upper.starts_with("CALL")
            || query_upper.starts_with("UNWIND");

        if !patterns.match_clause.is_match(query) && !starts_with_non_match {
            warnings.push("Query does not contain a MATCH clause".to_string());
        }

        // Check for RETURN clause
        if !patterns.return_clause.is_match(query) {
            warnings.push("Query does not contain a RETURN clause".to_string());
        }

        // Check for balanced parentheses
        if !Self::check_balanced_parentheses(query) {
            errors.push("Unbalanced parentheses in query".to_string());
        }

        // Check for balanced brackets
        if !Self::check_balanced_brackets(query) {
            errors.push("Unbalanced brackets in query".to_string());
        }

        ValidationResult {
            is_valid: errors.is_empty(),
            errors,
            warnings,
        }
    }

    /// Returns whether `masked` contains DROP or DELETE used as a clause keyword.
    ///
    /// The same words are legal symbolic names, so a match is skipped when it is a property
    /// name (`n.delete`, `{.drop}`), a label or relationship type (`:Drop`), a parameter
    /// (`$delete`) or a map key (`{drop: 1}`).
    fn contains_dangerous_clause(
        dangerous_ops: &Regex,
        masked: &str,
    ) -> bool {
        dangerous_ops.find_iter(masked).any(|m| {
            let before = masked[..m.start()].trim_end().chars().next_back();
            let after = masked[m.end()..].trim_start().chars().next();
            !matches!(before, Some('.' | ':' | '$')) && after != Some(':')
        })
    }

    /// Returns `query` with the contents of string literals, backtick-quoted identifiers and
    /// comments replaced by spaces, so keyword checks only see executable Cypher.
    fn mask_literals_and_comments(query: &str) -> String {
        let mut out = String::with_capacity(query.len());
        let mut chars = query.chars().peekable();
        while let Some(c) = chars.next() {
            match c {
                '\'' | '"' | '`' => {
                    out.push(c);
                    while let Some(inner) = chars.next() {
                        if inner == '\\' && c != '`' {
                            chars.next();
                            out.push_str("  ");
                        } else if inner == c {
                            out.push(c);
                            break;
                        } else {
                            out.push(' ');
                        }
                    }
                }
                '/' if chars.peek() == Some(&'/') => {
                    for inner in chars.by_ref() {
                        if inner == '\n' {
                            out.push('\n');
                            break;
                        }
                    }
                }
                '/' if chars.peek() == Some(&'*') => {
                    chars.next();
                    let mut prev = ' ';
                    for inner in chars.by_ref() {
                        if prev == '*' && inner == '/' {
                            break;
                        }
                        prev = inner;
                    }
                    out.push(' ');
                }
                _ => out.push(c),
            }
        }
        out
    }

    /// Checks if parentheses are balanced in the query
    fn check_balanced_parentheses(query: &str) -> bool {
        let mut count = 0;
        for c in query.chars() {
            match c {
                '(' => count += 1,
                ')' => {
                    count -= 1;
                    if count < 0 {
                        return false;
                    }
                }
                _ => {}
            }
        }
        count == 0
    }

    /// Checks if brackets are balanced in the query
    fn check_balanced_brackets(query: &str) -> bool {
        let mut count = 0;
        for c in query.chars() {
            match c {
                '[' => count += 1,
                ']' => {
                    count -= 1;
                    if count < 0 {
                        return false;
                    }
                }
                _ => {}
            }
        }
        count == 0
    }

    /// Suggests fixes for common query errors
    ///
    /// # Arguments
    ///
    /// * `query` - The query to analyze
    /// * `error` - The error message from query execution
    ///
    /// # Returns
    ///
    /// A suggested fixed query, if applicable
    ///
    /// # Note
    ///
    /// This function is available for future use in direct query fixing.
    /// Currently, self-healing uses LLM-based regeneration which is more flexible.
    #[allow(dead_code)]
    pub fn suggest_fix(
        query: &str,
        error: &str,
    ) -> Option<String> {
        let query = query.trim();
        let error_lower = error.to_lowercase();

        // Common error patterns and fixes
        if error_lower.contains("syntax error") || error_lower.contains("invalid syntax") {
            // Try to fix common syntax issues

            // Missing RETURN clause
            if !query.to_uppercase().contains("RETURN") {
                return Some(format!("{query}\nRETURN *"));
            }
        }

        // Missing WHERE keyword before condition
        if (error_lower.contains("syntax error") || error_lower.contains("invalid syntax"))
            && query.contains('=')
            && !query.to_uppercase().contains("WHERE")
            && query.to_uppercase().contains("MATCH")
            && let Some(fixed) = Self::try_add_where_clause(query)
        {
            return Some(fixed);
        }

        // Property not found - suggest using toLower() or different property
        if error_lower.contains("property") && error_lower.contains("not found") {
            tracing::info!(
                "Property not found error, consider checking schema or using toLower() for case-insensitive matching"
            );
        }

        None
    }

    /// Attempts to add WHERE clause to a query that might need it
    fn try_add_where_clause(query: &str) -> Option<String> {
        // Look for pattern like: MATCH (n:Label) n.prop = value
        let re = Regex::new(r"(?i)(MATCH\s+\([^)]+\))\s+([a-zA-Z_][a-zA-Z0-9_]*\.[a-zA-Z_][a-zA-Z0-9_]*\s*=)").ok()?;

        if let Some(caps) = re.captures(query) {
            let _match_part = caps.get(1)?.as_str();
            let condition_start = caps.get(2)?.start();

            let before = &query[..condition_start];
            let after = &query[condition_start..];

            return Some(format!("{before}\nWHERE {after}"));
        }

        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_query() {
        let query = "MATCH (n:Person) WHERE n.name = 'John' RETURN n";
        let result = CypherValidator::validate(query);
        assert!(result.is_valid, "Query should be valid");
        assert!(result.errors.is_empty(), "Should have no errors");
    }

    #[test]
    fn test_empty_query() {
        let query = "";
        let result = CypherValidator::validate(query);
        assert!(!result.is_valid, "Empty query should be invalid");
        assert!(!result.errors.is_empty(), "Should have errors");
    }

    #[test]
    fn test_unbalanced_parentheses() {
        let query = "MATCH (n:Person WHERE n.name = 'John' RETURN n";
        let result = CypherValidator::validate(query);
        assert!(!result.is_valid, "Query with unbalanced parentheses should be invalid");
    }

    #[test]
    fn test_dangerous_operations() {
        let query = "MATCH (n) DROP n";
        let result = CypherValidator::validate(query);
        assert!(!result.is_valid, "Query with DROP should be invalid");
    }

    #[test]
    fn test_dangerous_keywords_inside_literals_are_ignored() {
        let queries = [
            "MATCH (t:Task) WHERE t.name = 'Drop off package' RETURN t",
            r#"MATCH (t:Task) WHERE t.note = "delete me later" RETURN t"#,
            "MATCH (t:Task) WHERE t.name = 'it\\'s a drop zone' RETURN t",
            "MATCH (n:`Drop Zone`) RETURN n",
            "MATCH (n) // drop this later\nRETURN n",
            "MATCH (n) /* delete */ RETURN n",
            "MATCH (b:Backdrop) RETURN b.deleted_at",
            "MATCH (n) RETURN n.delete, n.drop",
            "MATCH (n) RETURN n . DELETE",
            "MATCH (n) RETURN n {.drop, .delete}",
            "MATCH (n:Drop)-[:DELETE]->(m) RETURN n, m",
            "MATCH (n) WHERE n.name = $drop RETURN n",
            "MATCH (n) RETURN {drop: n.name, delete : 1}",
        ];
        for query in queries {
            let result = CypherValidator::validate(query);
            assert!(result.is_valid, "{query} should be valid: {:?}", result.errors);
        }
    }

    #[test]
    fn test_dangerous_keywords_are_still_detected() {
        let queries = [
            "MATCH (n) DELETE n",
            "MATCH (n) DETACH DELETE n",
            "MATCH (n {name: 'x'}) DELETE n",
            "DROP INDEX ON :Person(name)",
            "MATCH (n) WHERE n.name = 'a' DELETE n",
            "MATCH (n) WITH n.delete AS d, n DELETE n",
            "MATCH (n:Drop) DELETE n",
        ];
        for query in queries {
            let result = CypherValidator::validate(query);
            assert!(!result.is_valid, "{query} should be flagged");
        }
    }

    #[test]
    fn test_balanced_parentheses() {
        assert!(CypherValidator::check_balanced_parentheses("()"));
        assert!(CypherValidator::check_balanced_parentheses("(())"));
        assert!(CypherValidator::check_balanced_parentheses("(()())"));
        assert!(!CypherValidator::check_balanced_parentheses("(()"));
        assert!(!CypherValidator::check_balanced_parentheses("())"));
    }
}
