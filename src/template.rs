use std::collections::HashMap;

pub struct TemplateEngine;

impl TemplateEngine {
    pub fn load_template(template_path: &str) -> Result<String, std::io::Error> {
        std::fs::read_to_string(template_path)
    }

    pub fn render(template: &str, variables: &HashMap<&str, &str>) -> String {
        let mut result = template.to_string();
        
        for (key, value) in variables {
            let placeholder = format!("{{{{{key}}}}}");
            result = result.replace(&placeholder, value);
        }
        
        result
    }

    pub fn render_system_prompt(ontology: &str) -> Result<String, std::io::Error> {
        let template = Self::load_template("templates/system_prompt.txt")?;
        let mut variables = HashMap::new();
        variables.insert("ONTOLOGY", ontology);
        
        Ok(Self::render(&template, &variables))
    }

    pub fn render_user_prompt(question: &str) -> Result<String, std::io::Error> {
        let template = Self::load_template("templates/user_prompt.txt")?;
        let mut variables = HashMap::new();
        variables.insert("QUESTION", question);
        
        Ok(Self::render(&template, &variables))
    }
}
