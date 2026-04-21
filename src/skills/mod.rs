mod loader;
mod parser;

pub use loader::SkillCatalog;
pub use parser::Skill;

use genai::chat::Tool;
use serde_json::json;
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

/// Configuration for query generation, replacing positional parameters.
pub struct QueryContext<'a> {
    pub schema: &'a str,
    pub skills: Option<&'a SkillCatalog>,
}

impl SkillCatalog {
    /// Load all skill.md files from a directory.
    ///
    /// Each subdirectory should contain a `skill.md` file.
    /// The subdirectory name becomes the stable skill ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be read or skill files fail to parse.
    pub fn from_directory(path: &Path) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let skills = loader::load_skills_from_directory(path)?;
        Ok(Self { skills })
    }

    /// Create an empty catalog (no skills loaded).
    #[must_use]
    pub fn empty() -> Self {
        Self { skills: HashMap::new() }
    }

    /// Render a compact catalog for inclusion in the system prompt.
    ///
    /// Returns one line per skill: `- {id}: {description}`
    #[must_use]
    pub fn render_catalog(&self) -> String {
        if self.skills.is_empty() {
            return String::new();
        }

        let mut lines = vec![
            "Available FalkorDB Cypher Skills (call read_skill with the skill id to load full details when needed):"
                .to_string(),
        ];

        let mut ids: Vec<&String> = self.skills.keys().collect();
        ids.sort_unstable();

        for id in ids {
            if let Some(skill) = self.skills.get(id) {
                lines.push(format!("- {id}: {}", skill.description));
            }
        }

        lines.join("\n")
    }

    /// Get a skill by its stable ID.
    #[must_use]
    pub fn get_skill(
        &self,
        id: &str,
    ) -> Option<&Skill> {
        self.skills.get(id)
    }

    /// Get all skill IDs (sorted).
    #[must_use]
    pub fn skill_ids(&self) -> Vec<&str> {
        let mut ids: Vec<&str> = self.skills.keys().map(String::as_str).collect();
        ids.sort_unstable();
        ids
    }

    /// Build a genai `Tool` definition for the `read_skill` function.
    ///
    /// The tool schema uses an enum constraint on the `id` parameter
    /// to restrict the LLM to valid skill IDs.
    #[must_use]
    pub fn tool_definition(&self) -> Tool {
        let ids = self.skill_ids();

        Tool::new("read_skill")
            .with_description(
                "Load the full content of a FalkorDB Cypher skill by its ID. \
                 Call this when you need detailed instructions, examples, or syntax \
                 for a specific skill listed in the catalog.",
            )
            .with_schema(json!({
                "type": "object",
                "properties": {
                    "id": {
                        "type": "string",
                        "description": "The skill ID from the catalog",
                        "enum": ids,
                    }
                },
                "required": ["id"],
            }))
    }

    /// Render all skill content directly for providers that don't support tool calling.
    #[must_use]
    pub fn render_all_content(&self) -> String {
        if self.skills.is_empty() {
            return String::new();
        }

        let mut sections = vec!["FalkorDB Cypher Skills:".to_string()];
        let mut ids: Vec<&String> = self.skills.keys().collect();
        ids.sort_unstable();

        for id in ids {
            if let Some(skill) = self.skills.get(id) {
                sections.push(format!("\n### {}\n{}", skill.name, skill.content));
            }
        }

        sections.join("\n")
    }

    /// Returns true if the catalog has any skills loaded.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.skills.is_empty()
    }

    /// Returns the number of skills in the catalog.
    #[must_use]
    pub fn len(&self) -> usize {
        self.skills.len()
    }
}

/// Check if a model's provider supports tool calling in genai 0.5.3.
///
/// Handles both prefixed models (e.g., `openai:gpt-4o`, `anthropic::claude-3`)
/// and unprefixed models (e.g., `gpt-4o-mini`, `claude-3-sonnet`).
///
/// `OpenAI`, Anthropic, Gemini, xAI, and `DeepSeek` adapters implement tool support.
/// Groq, Ollama, and Cohere have zero or minimal tool support.
#[must_use]
pub fn supports_tool_calling(model: &str) -> bool {
    use genai::adapter::AdapterKind;

    // First: handle single-colon prefixed models (e.g., "openai:gpt-4o")
    // This must come before from_model() because genai uses :: for namespaces
    // and from_model() would resolve "openai:gpt-4o" to Ollama (the fallback).
    if let Some((prefix, _)) = model.split_once(':') {
        if let Some(kind) = AdapterKind::from_lower_str(prefix) {
            return is_tool_capable_adapter(kind);
        }
    }

    // Then: try genai's built-in resolution (handles unprefixed models)
    AdapterKind::from_model(model).is_ok_and(is_tool_capable_adapter)
}

const fn is_tool_capable_adapter(kind: genai::adapter::AdapterKind) -> bool {
    use genai::adapter::AdapterKind;

    matches!(
        kind,
        AdapterKind::OpenAI
            | AdapterKind::OpenAIResp
            | AdapterKind::Anthropic
            | AdapterKind::Gemini
            | AdapterKind::Xai
            | AdapterKind::DeepSeek
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_empty_catalog() {
        let catalog = SkillCatalog::empty();
        assert!(catalog.is_empty());
        assert_eq!(catalog.len(), 0);
        assert_eq!(catalog.render_catalog(), "");
        assert_eq!(catalog.render_all_content(), "");
        assert!(catalog.skill_ids().is_empty());
    }

    #[test]
    fn test_catalog_with_skills() {
        let mut skills = HashMap::new();
        skills.insert(
            "test-skill".to_string(),
            Skill {
                id: "test-skill".to_string(),
                name: "Test Skill".to_string(),
                description: "A test skill".to_string(),
                content: "# Test\nSome content".to_string(),
            },
        );
        let catalog = SkillCatalog { skills };

        assert!(!catalog.is_empty());
        assert_eq!(catalog.len(), 1);
        assert!(catalog.render_catalog().contains("test-skill: A test skill"));
        assert!(catalog.get_skill("test-skill").is_some());
        assert!(catalog.get_skill("nonexistent").is_none());
        assert_eq!(catalog.skill_ids(), vec!["test-skill"]);
    }

    #[test]
    fn test_tool_definition() {
        let mut skills = HashMap::new();
        skills.insert(
            "skill-a".to_string(),
            Skill {
                id: "skill-a".to_string(),
                name: "Skill A".to_string(),
                description: "First skill".to_string(),
                content: "Content A".to_string(),
            },
        );
        skills.insert(
            "skill-b".to_string(),
            Skill {
                id: "skill-b".to_string(),
                name: "Skill B".to_string(),
                description: "Second skill".to_string(),
                content: "Content B".to_string(),
            },
        );
        let catalog = SkillCatalog { skills };
        let tool = catalog.tool_definition();

        assert_eq!(tool.name, "read_skill");
        assert!(tool.description.is_some());
        let schema = tool.schema.unwrap();
        let enum_values = &schema["properties"]["id"]["enum"];
        assert!(enum_values.as_array().unwrap().contains(&json!("skill-a")));
        assert!(enum_values.as_array().unwrap().contains(&json!("skill-b")));
    }

    #[test]
    fn test_supports_tool_calling() {
        // Prefixed model names
        assert!(supports_tool_calling("openai:gpt-4o"));
        assert!(supports_tool_calling("anthropic:claude-3-sonnet"));
        assert!(supports_tool_calling("gemini:gemini-pro"));
        assert!(supports_tool_calling("xai:grok-2"));
        assert!(supports_tool_calling("deepseek:deepseek-chat"));
        assert!(!supports_tool_calling("ollama:llama3"));

        // Unprefixed model names (common usage)
        assert!(supports_tool_calling("gpt-4o-mini"));
        assert!(supports_tool_calling("gpt-4o"));
        assert!(supports_tool_calling("claude-3-sonnet-20241022"));
        assert!(supports_tool_calling("gemini-2.0-flash-exp"));
        assert!(supports_tool_calling("grok-2"));
    }

    #[test]
    fn test_render_all_content() {
        let mut skills = HashMap::new();
        skills.insert(
            "my-skill".to_string(),
            Skill {
                id: "my-skill".to_string(),
                name: "My Skill".to_string(),
                description: "Does things".to_string(),
                content: "Detailed instructions here".to_string(),
            },
        );
        let catalog = SkillCatalog { skills };
        let rendered = catalog.render_all_content();

        assert!(rendered.contains("FalkorDB Cypher Skills:"));
        assert!(rendered.contains("### My Skill"));
        assert!(rendered.contains("Detailed instructions here"));
    }
}
