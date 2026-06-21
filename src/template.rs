use std::collections::HashMap;

pub struct TemplateEngine;

impl TemplateEngine {
    // Templates embedded at compile time
    const SYSTEM_PROMPT: &'static str = include_str!("../templates/system_prompt.txt");
    const USER_PROMPT: &'static str = include_str!("../templates/user_prompt.txt");
    const LAST_REQUEST_PROMPT: &'static str = include_str!("../templates/last_request_prompt.txt");
    const FALKORDB_REFERENCE: &'static str = include_str!("../templates/falkordb_reference.txt");

    #[must_use]
    pub fn render(
        template: &str,
        variables: &HashMap<&str, &str>,
    ) -> String {
        let mut result = template.to_string();

        for (key, value) in variables {
            let placeholder = format!("{{{{{key}}}}}");
            result = result.replace(&placeholder, value);
        }

        result
    }

    /// Render the system prompt template with ontology.
    // Retained as public API and used by the library/tests; the binary recompiles this module but
    // only calls `render_system_prompt_with_context`, so allow dead_code for the bin build.
    #[allow(dead_code)]
    #[must_use]
    pub fn render_system_prompt(ontology: &str) -> String {
        Self::render_system_prompt_with_skills(ontology, "")
    }

    /// Render the system prompt template with ontology and optional skills catalog.
    /// When `skills_catalog` is empty, renders the prompt without any skills section.
    #[allow(dead_code)]
    #[must_use]
    pub fn render_system_prompt_with_skills(
        ontology: &str,
        skills_catalog: &str,
    ) -> String {
        Self::render_system_prompt_with_context(ontology, skills_catalog, "")
    }

    /// Render the system prompt template with ontology and optional skills catalog and UDF context.
    ///
    /// Empty `skills_catalog` / `udfs` sections are omitted. When no skills are present the leftover
    /// blank lines from empty placeholders are collapsed; when skills are present their content
    /// (which may contain meaningful blank lines) is preserved verbatim.
    #[must_use]
    pub fn render_system_prompt_with_context(
        ontology: &str,
        skills_catalog: &str,
        udfs: &str,
    ) -> String {
        let mut variables = HashMap::new();
        variables.insert("ONTOLOGY", ontology);
        variables.insert("SKILLS_CATALOG", skills_catalog);
        variables.insert("UDFS", udfs);
        variables.insert("FALKORDB_REFERENCE", Self::FALKORDB_REFERENCE);
        let rendered = Self::render(Self::SYSTEM_PROMPT, &variables);

        if !skills_catalog.trim().is_empty() {
            return rendered;
        }

        // Collapse consecutive blank lines left by empty placeholder substitution
        Self::collapse_consecutive_blank_lines(&rendered)
    }

    #[must_use]
    fn collapse_consecutive_blank_lines(rendered: &str) -> String {
        let had_trailing_newline = rendered.ends_with('\n');
        let mut result = String::with_capacity(rendered.len());
        let mut prev_blank = false;
        for line in rendered.lines() {
            let is_blank = line.trim().is_empty();
            if is_blank && prev_blank {
                continue;
            }
            if !result.is_empty() {
                result.push('\n');
            }
            result.push_str(line);
            prev_blank = is_blank;
        }
        if had_trailing_newline {
            result.push('\n');
        }
        result
    }

    /// Render the user prompt template with the given question.
    #[must_use]
    pub fn render_user_prompt(question: &str) -> String {
        let mut variables = HashMap::new();
        variables.insert("QUESTION", question);
        Self::render(Self::USER_PROMPT, &variables)
    }

    /// Render the last request prompt template with the given parameters.
    #[must_use]
    pub fn render_last_request_prompt(
        question: &str,
        cypher_query: &str,
        cypher_result: &str,
    ) -> String {
        let mut variables = HashMap::new();
        variables.insert("CYPHER_QUERY", cypher_query);
        variables.insert("CYPHER_RESULT", cypher_result);
        variables.insert("USER_QUESTION", question);
        Self::render(Self::LAST_REQUEST_PROMPT, &variables)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_prompt_includes_falkordb_reference() {
        let prompt = TemplateEngine::render_system_prompt("{}");
        assert!(prompt.contains("db.idx.fulltext.queryNodes"));
        assert!(prompt.contains("db.idx.vector.queryNodes"));
        assert!(prompt.contains("algo.SPpaths"));
        assert!(!prompt.contains("{{FALKORDB_REFERENCE}}"));
        assert!(!prompt.contains("{{ONTOLOGY}}"));
        assert!(!prompt.contains("{{SKILLS_CATALOG}}"));
        assert!(!prompt.contains("{{UDFS}}"));
    }

    #[test]
    fn system_prompt_with_skills_includes_catalog_and_reference() {
        let prompt = TemplateEngine::render_system_prompt_with_skills("{}", "Available skills:\n- foo: bar");
        assert!(prompt.contains("db.idx.fulltext.queryNodes"));
        assert!(prompt.contains("Available skills:"));
        assert!(!prompt.contains("{{SKILLS_CATALOG}}"));
        assert!(!prompt.contains("{{FALKORDB_REFERENCE}}"));
        assert!(!prompt.contains("{{UDFS}}"));
    }

    #[test]
    fn system_prompt_with_context_includes_udfs() {
        let udfs = "Available User-Defined Functions on this FalkorDB instance.\n- mylib.Foo";
        let prompt = TemplateEngine::render_system_prompt_with_context("{}", "", udfs);
        assert!(prompt.contains("- mylib.Foo"));
        assert!(prompt.contains("db.idx.fulltext.queryNodes"));
        assert!(!prompt.contains("{{UDFS}}"));
        assert!(!prompt.contains("{{SKILLS_CATALOG}}"));
    }

    #[test]
    fn system_prompt_with_context_includes_both_skills_and_udfs() {
        let prompt =
            TemplateEngine::render_system_prompt_with_context("{}", "Available skills:\n- foo: bar", "- mylib.Foo");
        assert!(prompt.contains("Available skills:"));
        assert!(prompt.contains("- mylib.Foo"));
        assert!(!prompt.contains("{{UDFS}}"));
        assert!(!prompt.contains("{{SKILLS_CATALOG}}"));
    }
}
