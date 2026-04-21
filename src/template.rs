use std::collections::HashMap;

pub struct TemplateEngine;

impl TemplateEngine {
    // Templates embedded at compile time
    const SYSTEM_PROMPT: &'static str = include_str!("../templates/system_prompt.txt");
    const USER_PROMPT: &'static str = include_str!("../templates/user_prompt.txt");
    const LAST_REQUEST_PROMPT: &'static str = include_str!("../templates/last_request_prompt.txt");

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

    /// Render the system prompt template with ontology and optional skills catalog.
    /// When `skills_catalog` is empty, renders the prompt without any skills section.
    #[must_use]
    pub fn render_system_prompt_with_skills(
        ontology: &str,
        skills_catalog: &str,
    ) -> String {
        let mut variables = HashMap::new();
        variables.insert("ONTOLOGY", ontology);
        variables.insert("SKILLS_CATALOG", skills_catalog);
        let rendered = Self::render(Self::SYSTEM_PROMPT, &variables);

        // Collapse consecutive blank lines left by empty placeholder substitution
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
