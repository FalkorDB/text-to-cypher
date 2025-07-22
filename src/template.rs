use std::collections::HashMap;

pub struct TemplateEngine;

impl TemplateEngine {
    /// Load a template from a file path.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read.
    pub fn load_template(template_path: &str) -> Result<String, std::io::Error> {
        std::fs::read_to_string(template_path)
    }

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
    ///
    /// # Errors
    ///
    /// Returns an error if the template file cannot be read.
    pub fn render_system_prompt(ontology: &str) -> Result<String, std::io::Error> {
        let template = Self::load_template("templates/system_prompt.txt")?;
        let mut variables = HashMap::new();
        variables.insert("ONTOLOGY", ontology);

        Ok(Self::render(&template, &variables))
    }

    /// Render the user prompt template with the given question.
    ///
    /// # Errors
    ///
    /// Returns an error if the template file cannot be read.
    pub fn render_user_prompt(question: &str) -> Result<String, std::io::Error> {
        let template = Self::load_template("templates/user_prompt.txt")?;
        let mut variables = HashMap::new();
        variables.insert("QUESTION", question);

        Ok(Self::render(&template, &variables))
    }

    /// Render the last request prompt template with the given parameters.
    ///
    /// # Errors
    ///
    /// Returns an error if the template file cannot be read.
    pub fn render_last_request_prompt(
        question: &str,
        cypher_query: &str,
        cypher_result: &str,
    ) -> Result<String, std::io::Error> {
        let template = Self::load_template("templates/last_request_prompt.txt")?;
        let mut variables = HashMap::new();
        variables.insert("CYPHER_QUERY", cypher_query);
        variables.insert("CYPHER_RESULT", cypher_result);
        variables.insert("USER_QUESTION", question);

        Ok(Self::render(&template, &variables))
    }
}
