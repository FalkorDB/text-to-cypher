mod builtin;
mod loader;
mod parser;

pub use builtin::SkillProfile;
pub use loader::SkillCatalog;
pub use parser::Skill;

use genai::chat::{Tool, ToolCall, ToolResponse};
use serde_json::json;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::path::Path;

/// Maximum number of skill tool calls answered in a single LLM round.
pub const MAX_SKILL_TOOL_CALLS_PER_ROUND: usize = 4;

/// Maximum number of `read_skill` tool-call rounds before forcing a final answer.
pub const MAX_TOOL_ROUNDS: usize = 3;

impl SkillCatalog {
    /// Load all skill.md files from a directory.
    ///
    /// Each subdirectory should contain a `skill.md` file.
    /// The subdirectory name becomes the stable skill ID.
    ///
    /// # Errors
    ///
    /// Returns an error if the directory cannot be read. Individual skill read
    /// or parse failures are logged and skipped.
    pub fn from_directory(path: &Path) -> Result<Self, Box<dyn Error + Send + Sync>> {
        let skills = loader::load_skills_from_directory(path)?;
        Ok(Self { skills })
    }

    /// Create an empty catalog (no skills loaded).
    #[must_use]
    pub fn empty() -> Self {
        Self { skills: HashMap::new() }
    }

    /// Build a catalog from the curated, read-only `FalkorDB` Cypher skills embedded in the binary.
    ///
    /// Unlike [`from_directory`](Self::from_directory) this needs no filesystem access, so library,
    /// napi, and browser consumers get `FalkorDB`-specific context by default — not only the Docker
    /// image (which loads skills from `SKILLS_DIR`).
    #[must_use]
    pub fn builtin() -> Self {
        // Parse the embedded skills once, then clone the cached catalog. `builtin()` is on the
        // per-request hot path (e.g. `process_text_to_cypher`), so avoid re-parsing YAML each call.
        static BUILTIN: std::sync::OnceLock<SkillCatalog> = std::sync::OnceLock::new();
        BUILTIN
            .get_or_init(|| Self {
                skills: builtin::builtin_skills(),
            })
            .clone()
    }

    /// Merge `other` into this catalog, returning the combined catalog. Skills in `other` override
    /// skills already present under the same ID.
    #[must_use]
    pub fn merged_with(
        mut self,
        other: Self,
    ) -> Self {
        self.skills.extend(other.skills);
        self
    }

    /// Drop any skills not permitted under `profile`.
    ///
    /// Under [`SkillProfile::ReadOnly`] known write/DDL skills are removed, so an externally supplied
    /// catalog (e.g. the server's `SKILLS_DIR`) cannot reintroduce write instructions into the prompt.
    #[must_use]
    pub fn with_profile(
        mut self,
        profile: SkillProfile,
    ) -> Self {
        match profile {
            SkillProfile::ReadOnly => self
                .skills
                .retain(|id, skill| !builtin::is_write_skill(id) && !builtin::teaches_write_cypher(&skill.content)),
        }
        self
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
    /// Skill IDs are listed in the prompt catalog and validated host-side to avoid
    /// duplicating large catalogs inside every tool schema.
    #[must_use]
    pub fn tool_definition(&self) -> Tool {
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
                sections.push(format!("\n{}", render_skill_content(skill, "###")));
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

/// Resolve `read_skill` tool calls with per-round caps and duplicate suppression.
#[must_use]
pub fn resolve_skill_tool_calls(
    tool_calls: &[ToolCall],
    catalog: Option<&SkillCatalog>,
) -> Vec<ToolResponse> {
    let mut served_skill_ids = HashSet::new();
    let mut served_skill_count = 0;

    tool_calls
        .iter()
        .map(|tool_call| {
            let content = resolve_skill_tool_call(tool_call, catalog, &mut served_skill_ids, &mut served_skill_count);
            ToolResponse::new(&tool_call.call_id, content)
        })
        .collect()
}

fn resolve_skill_tool_call(
    tool_call: &ToolCall,
    catalog: Option<&SkillCatalog>,
    served_skill_ids: &mut HashSet<String>,
    served_skill_count: &mut usize,
) -> String {
    if tool_call.fn_name != "read_skill" {
        return format!("Unknown tool: {}", tool_call.fn_name);
    }

    let Some(skill_id) = tool_call
        .fn_arguments
        .get("id")
        .and_then(|value| value.as_str())
        .map(str::trim)
        .filter(|id| !id.is_empty())
    else {
        return "Missing required argument: id".to_string();
    };

    let Some(skill) = catalog.and_then(|c| c.get_skill(skill_id)) else {
        return format!("Skill '{skill_id}' not found in catalog");
    };

    if served_skill_ids.contains(skill_id) {
        return format!("Skill '{skill_id}' was already provided in this round; reuse the previous tool response.");
    }

    if *served_skill_count >= MAX_SKILL_TOOL_CALLS_PER_ROUND {
        return format!(
            "Too many read_skill calls in one round. Request at most {MAX_SKILL_TOOL_CALLS_PER_ROUND} skills at a time."
        );
    }

    served_skill_ids.insert(skill_id.to_string());
    *served_skill_count += 1;
    render_skill_content(skill, "#")
}

fn render_skill_content(
    skill: &Skill,
    heading_prefix: &str,
) -> String {
    let content = skill.content.trim();
    if content.starts_with('#') {
        content.to_string()
    } else {
        format!("{heading_prefix} {}\n\n{content}", skill.name)
    }
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
    fn test_builtin_catalog_loads_read_only_skills() {
        let catalog = SkillCatalog::builtin();
        assert!(!catalog.is_empty());
        assert_eq!(catalog.len(), 5);
        for id in [
            "falkordb-index-aware-predicates",
            "falkordb-fulltext-search",
            "falkordb-vector-search",
            "falkordb-parameterized-queries",
            "falkordb-path-finding",
        ] {
            assert!(catalog.get_skill(id).is_some(), "missing built-in skill {id}");
        }
        assert!(catalog.render_catalog().contains("falkordb-fulltext-search"));
    }

    #[test]
    fn test_with_profile_read_only_filters_write_skills() {
        let mut skills = HashMap::new();
        skills.insert(
            "create-range-indexes".to_string(),
            Skill {
                id: "create-range-indexes".to_string(),
                name: "Create range indexes".to_string(),
                description: "DDL".to_string(),
                content: "creates an index".to_string(),
            },
        );
        skills.insert(
            "match-patterns-and-return-projections".to_string(),
            Skill {
                id: "match-patterns-and-return-projections".to_string(),
                name: "Match patterns".to_string(),
                description: "read".to_string(),
                content: "MATCH (n) RETURN n".to_string(),
            },
        );
        let catalog = SkillCatalog { skills }.with_profile(SkillProfile::ReadOnly);

        assert!(catalog.get_skill("create-range-indexes").is_none());
        assert!(catalog.get_skill("match-patterns-and-return-projections").is_some());
    }

    #[test]
    fn test_with_profile_filters_content_based_write_skills() {
        // A skill not in the ID denylist but whose code teaches a write clause is still filtered.
        let mut skills = HashMap::new();
        skills.insert(
            "custom-writer".to_string(),
            Skill {
                id: "custom-writer".to_string(),
                name: "Custom writer".to_string(),
                description: "not in the ID denylist".to_string(),
                content: "```cypher\nMATCH (n) SET n.flag = true\n```".to_string(),
            },
        );
        skills.insert(
            "custom-reader".to_string(),
            Skill {
                id: "custom-reader".to_string(),
                name: "Custom reader".to_string(),
                description: "read only".to_string(),
                content: "```cypher\nMATCH (n) RETURN n\n```".to_string(),
            },
        );
        let catalog = SkillCatalog { skills }.with_profile(SkillProfile::ReadOnly);

        assert!(catalog.get_skill("custom-writer").is_none());
        assert!(catalog.get_skill("custom-reader").is_some());
    }

    #[test]
    fn test_merged_with_overrides_by_id() {
        let mut base = HashMap::new();
        base.insert(
            "a".to_string(),
            Skill {
                id: "a".to_string(),
                name: "Base A".to_string(),
                description: "base".to_string(),
                content: "base".to_string(),
            },
        );
        let mut other = HashMap::new();
        other.insert(
            "a".to_string(),
            Skill {
                id: "a".to_string(),
                name: "Override A".to_string(),
                description: "override".to_string(),
                content: "override".to_string(),
            },
        );
        other.insert(
            "b".to_string(),
            Skill {
                id: "b".to_string(),
                name: "B".to_string(),
                description: "b".to_string(),
                content: "b".to_string(),
            },
        );

        let merged = SkillCatalog { skills: base }.merged_with(SkillCatalog { skills: other });

        assert_eq!(merged.len(), 2);
        assert_eq!(merged.get_skill("a").unwrap().name, "Override A");
        assert!(merged.get_skill("b").is_some());
    }

    #[test]
    fn test_builtin_render_all_content_within_budget() {
        // Non-tool providers inline the full built-in bodies; guard against unbounded prompt growth.
        let rendered = SkillCatalog::builtin().render_all_content();
        assert!(!rendered.is_empty());
        assert!(
            rendered.len() < 8_000,
            "built-in inline content too large: {} bytes",
            rendered.len()
        );
    }

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

        assert_eq!(tool.name.as_ref(), "read_skill");
        assert!(tool.description.is_some());
        let schema = tool.schema.unwrap();
        assert_eq!(schema["properties"]["id"]["type"], json!("string"));
        assert!(schema["properties"]["id"].get("enum").is_none());
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
    fn test_resolve_skill_tool_calls_caps_round_size() {
        let mut skills = HashMap::new();
        for index in 0..=MAX_SKILL_TOOL_CALLS_PER_ROUND {
            let id = format!("skill-{index}");
            skills.insert(
                id.clone(),
                Skill {
                    id,
                    name: format!("Skill {index}"),
                    description: format!("Skill {index} description"),
                    content: format!("Content {index}"),
                },
            );
        }
        let catalog = SkillCatalog { skills };
        let calls = (0..=MAX_SKILL_TOOL_CALLS_PER_ROUND)
            .map(|index| ToolCall {
                call_id: format!("call-{index}"),
                fn_name: "read_skill".to_string(),
                fn_arguments: json!({ "id": format!("skill-{index}") }),
                thought_signatures: None,
            })
            .collect::<Vec<_>>();

        let responses = resolve_skill_tool_calls(&calls, Some(&catalog));

        assert_eq!(responses.len(), calls.len());
        assert!(responses.last().unwrap().content.contains("Too many read_skill calls"));
    }

    #[test]
    fn test_resolve_skill_tool_calls_suppresses_duplicates() {
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
        let catalog = SkillCatalog { skills };
        let calls = vec![
            ToolCall {
                call_id: "call-1".to_string(),
                fn_name: "read_skill".to_string(),
                fn_arguments: json!({ "id": "skill-a" }),
                thought_signatures: None,
            },
            ToolCall {
                call_id: "call-2".to_string(),
                fn_name: "read_skill".to_string(),
                fn_arguments: json!({ "id": "skill-a" }),
                thought_signatures: None,
            },
        ];

        let responses = resolve_skill_tool_calls(&calls, Some(&catalog));

        assert!(responses[0].content.contains("Content A"));
        assert!(responses[1].content.contains("already provided"));
        assert!(!responses[1].content.contains("Content A"));
    }

    #[test]
    fn test_resolve_skill_tool_calls_duplicates_do_not_consume_cap() {
        let mut skills = HashMap::new();
        for index in 0..MAX_SKILL_TOOL_CALLS_PER_ROUND {
            let id = format!("skill-{index}");
            skills.insert(
                id.clone(),
                Skill {
                    id,
                    name: format!("Skill {index}"),
                    description: format!("Skill {index} description"),
                    content: format!("Content {index}"),
                },
            );
        }
        let catalog = SkillCatalog { skills };
        let mut calls = vec![ToolCall {
            call_id: "duplicate".to_string(),
            fn_name: "read_skill".to_string(),
            fn_arguments: json!({ "id": "skill-0" }),
            thought_signatures: None,
        }];
        calls.extend((0..MAX_SKILL_TOOL_CALLS_PER_ROUND).map(|index| ToolCall {
            call_id: format!("call-{index}"),
            fn_name: "read_skill".to_string(),
            fn_arguments: json!({ "id": format!("skill-{index}") }),
            thought_signatures: None,
        }));

        let responses = resolve_skill_tool_calls(&calls, Some(&catalog));

        assert!(responses[1].content.contains("already provided"));
        assert!(
            responses
                .last()
                .unwrap()
                .content
                .contains(&format!("Content {}", MAX_SKILL_TOOL_CALLS_PER_ROUND - 1))
        );
    }

    #[test]
    fn test_resolve_skill_tool_calls_reports_missing_id() {
        let calls = vec![ToolCall {
            call_id: "missing-id".to_string(),
            fn_name: "read_skill".to_string(),
            fn_arguments: json!({}),
            thought_signatures: None,
        }];

        let responses = resolve_skill_tool_calls(&calls, None);

        assert_eq!(responses[0].content, "Missing required argument: id");
    }

    #[test]
    fn test_resolve_skill_tool_calls_preserves_existing_heading() {
        let mut skills = HashMap::new();
        skills.insert(
            "skill-a".to_string(),
            Skill {
                id: "skill-a".to_string(),
                name: "Skill A".to_string(),
                description: "First skill".to_string(),
                content: "# Skill A\n\nContent A".to_string(),
            },
        );
        let catalog = SkillCatalog { skills };
        let calls = vec![ToolCall {
            call_id: "call-1".to_string(),
            fn_name: "read_skill".to_string(),
            fn_arguments: json!({ "id": "skill-a" }),
            thought_signatures: None,
        }];

        let responses = resolve_skill_tool_calls(&calls, Some(&catalog));

        assert!(responses[0].content.starts_with("# Skill A"));
        assert!(!responses[0].content.contains("# Skill A\n\n# Skill A"));
        assert!(responses[0].content.contains("Content A"));
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
                content: "# My Skill\n\nDetailed instructions here".to_string(),
            },
        );
        let catalog = SkillCatalog { skills };
        let rendered = catalog.render_all_content();

        assert!(rendered.contains("FalkorDB Cypher Skills:"));
        assert!(rendered.contains("# My Skill"));
        assert!(!rendered.contains("### My Skill\n# My Skill"));
        assert!(rendered.contains("Detailed instructions here"));
    }

    #[test]
    fn test_render_all_content_adds_heading_for_unheaded_skill() {
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

        assert!(rendered.contains("### My Skill"));
        assert!(rendered.contains("Detailed instructions here"));
    }
}
