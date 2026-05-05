use serde::Deserialize;
use std::error::Error;

/// A single skill parsed from a `skill.md` file.
#[derive(Debug, Clone)]
pub struct Skill {
    /// Stable machine ID (directory name / slug).
    pub id: String,
    /// Human-readable title from YAML frontmatter.
    pub name: String,
    /// Description from YAML frontmatter.
    pub description: String,
    /// Full markdown body (everything after the frontmatter).
    pub content: String,
}

/// YAML frontmatter structure expected in skill.md files.
#[derive(Deserialize)]
struct SkillFrontmatter {
    name: String,
    description: String,
}

/// Parse a skill.md file content into a `Skill`.
///
/// The file must have YAML frontmatter delimited by `---` lines,
/// containing at least `name` and `description` fields.
///
/// # Arguments
///
/// * `id` - The stable skill identifier (typically the directory name)
/// * `raw` - The raw file content
///
/// # Errors
///
/// Returns an error if frontmatter is missing, malformed, or lacks required fields.
pub fn parse_skill(
    id: &str,
    raw: &str,
) -> Result<Skill, Box<dyn Error + Send + Sync>> {
    let raw = raw.trim();

    if !raw.starts_with("---") {
        return Err(format!("Skill '{id}': missing YAML frontmatter delimiter").into());
    }

    let after_open = raw
        .strip_prefix("---\n")
        .or_else(|| raw.strip_prefix("---\r\n"))
        .ok_or_else(|| format!("Skill '{id}': missing YAML frontmatter delimiter"))?;

    let (yaml_str, content) = split_frontmatter_body(after_open)
        .ok_or_else(|| format!("Skill '{id}': missing closing frontmatter delimiter"))?;

    let frontmatter: SkillFrontmatter =
        serde_yaml_ng::from_str(yaml_str).map_err(|e| format!("Skill '{id}': invalid YAML: {e}"))?;

    Ok(Skill {
        id: id.to_string(),
        name: frontmatter.name,
        description: frontmatter.description,
        content: content.to_string(),
    })
}

fn split_frontmatter_body(after_open: &str) -> Option<(&str, &str)> {
    let mut offset = 0;

    for line in after_open.split_inclusive('\n') {
        let line_without_lf = line.strip_suffix('\n').unwrap_or(line);
        let line_without_ending = line_without_lf.strip_suffix('\r').unwrap_or(line_without_lf);

        if line_without_ending.trim_end() == "---" {
            let body_start = offset + line.len();
            return Some((after_open[..offset].trim(), after_open[body_start..].trim()));
        }

        offset += line.len();
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_skill() {
        let raw = r"---
name: Apply FalkorDB Cypher limitations correctly
description: Account for FalkorDB Cypher limitations like non-indexed not-equal filters
---

# Apply FalkorDB Cypher limitations correctly

Some instructions here.

## Example

```cypher
MATCH (n:Person) WHERE n.age > 30 RETURN n
```";

        let skill = parse_skill("apply-cypher-limitations", raw).unwrap();
        assert_eq!(skill.id, "apply-cypher-limitations");
        assert_eq!(skill.name, "Apply FalkorDB Cypher limitations correctly");
        assert!(skill.description.contains("non-indexed not-equal"));
        assert!(skill.content.contains("# Apply FalkorDB Cypher limitations correctly"));
        assert!(skill.content.contains("MATCH (n:Person)"));
    }

    #[test]
    fn test_parse_missing_frontmatter() {
        let raw = "# Just a markdown file\nNo frontmatter here.";
        let result = parse_skill("bad-skill", raw);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing YAML frontmatter"));
    }

    #[test]
    fn test_parse_unclosed_frontmatter() {
        let raw = "---\nname: Test\ndescription: Missing close\n# Body";
        let result = parse_skill("unclosed", raw);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("missing closing"));
    }

    #[test]
    fn test_parse_missing_required_field() {
        let raw = "---\nname: Only name\n---\n# Body";
        let result = parse_skill("missing-desc", raw);
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_empty_body() {
        let raw = "---\nname: Empty Body Skill\ndescription: Has no content\n---";
        let skill = parse_skill("empty-body", raw).unwrap();
        assert_eq!(skill.id, "empty-body");
        assert_eq!(skill.name, "Empty Body Skill");
        assert!(skill.content.is_empty());
    }

    #[test]
    fn test_parse_extra_yaml_fields() {
        let raw = "---\nname: Extended Skill\ndescription: Has extra fields\ntags: [cypher, index]\n---\n# Body";
        let skill = parse_skill("extended", raw).unwrap();
        assert_eq!(skill.name, "Extended Skill");
        assert!(skill.content.contains("# Body"));
    }

    #[test]
    fn test_parse_closing_delimiter_requires_full_line() {
        let raw = "---\nname: Delimiter-like YAML\ndescription: Has a delimiter-like key\n---foo: bar\n---\n# Body";
        let skill = parse_skill("delimiter-like-yaml", raw).unwrap();
        assert_eq!(skill.name, "Delimiter-like YAML");
        assert!(skill.content.contains("# Body"));
    }

    #[test]
    fn test_parse_crlf_frontmatter() {
        let raw = "---\r\nname: CRLF Skill\r\ndescription: Uses CRLF delimiters\r\n---\r\n# Body";
        let skill = parse_skill("crlf", raw).unwrap();
        assert_eq!(skill.name, "CRLF Skill");
        assert!(skill.content.contains("# Body"));
    }
}
