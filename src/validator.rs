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
            // Simplified pattern to catch dangerous operations more reliably
            // Matches any DROP or DELETE (with or without DETACH, with any following content)
            dangerous_ops: Regex::new(r"(?i)(DROP\s|DELETE\s)").unwrap(),
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
        if patterns.dangerous_ops.is_match(query) {
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
    fn test_balanced_parentheses() {
        assert!(CypherValidator::check_balanced_parentheses("()"));
        assert!(CypherValidator::check_balanced_parentheses("(())"));
        assert!(CypherValidator::check_balanced_parentheses("(()())"));
        assert!(!CypherValidator::check_balanced_parentheses("(()"));
        assert!(!CypherValidator::check_balanced_parentheses("())"));
    }
}
