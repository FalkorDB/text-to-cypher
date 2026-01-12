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

    /// Render the system prompt template with the given ontology.
    #[must_use]
    pub fn render_system_prompt(ontology: &str) -> Result<String, std::io::Error> {
        let mut variables = HashMap::new();
        variables.insert("ONTOLOGY", ontology);

        Ok(Self::render(Self::SYSTEM_PROMPT, &variables))
    }

    /// Render the user prompt template with the given question.
    #[must_use]
    pub fn render_user_prompt(question: &str) -> Result<String, std::io::Error> {
        let mut variables = HashMap::new();
        variables.insert("QUESTION", question);

        Ok(Self::render(Self::USER_PROMPT, &variables))
    }

    /// Render the last request prompt template with the given parameters.
    #[must_use]
    pub fn render_last_request_prompt(
        question: &str,
        cypher_query: &str,
        cypher_result: &str,
    ) -> Result<String, std::io::Error> {
        let mut variables = HashMap::new();
        variables.insert("CYPHER_QUERY", cypher_query);
        variables.insert("CYPHER_RESULT", cypher_result);
        variables.insert("USER_QUESTION", question);

        Ok(Self::render(Self::LAST_REQUEST_PROMPT, &variables))
    }
}
