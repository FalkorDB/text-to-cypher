use super::parser::{Skill, parse_skill};
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;

/// A catalog of loaded skills, keyed by stable ID.
#[derive(Debug, Clone)]
pub struct SkillCatalog {
    pub(crate) skills: HashMap<String, Skill>,
}

/// Load all skills from a directory.
///
/// Expected structure:
/// ```text
/// skills_dir/
///   skill-name-a/
///     skill.md
///   skill-name-b/
///     skill.md
/// ```
///
/// The subdirectory name becomes the skill's stable ID.
/// Subdirectories without a `skill.md` are silently skipped.
///
/// # Errors
///
/// Returns an error if the directory cannot be read.
/// Individual skill parse failures are logged and skipped.
pub fn load_skills_from_directory(path: &Path) -> Result<HashMap<String, Skill>, Box<dyn Error + Send + Sync>> {
    let mut skills = HashMap::new();

    if !path.is_dir() {
        return Err(format!("Skills path is not a directory: {}", path.display()).into());
    }

    let entries = std::fs::read_dir(path).map_err(|e| format!("Failed to read skills directory: {e}"))?;

    for entry in entries {
        let entry = match entry {
            Ok(e) => e,
            Err(e) => {
                tracing::warn!("Failed to read directory entry: {e}");
                continue;
            }
        };

        let entry_path = entry.path();
        if !entry_path.is_dir() {
            continue;
        }

        let skill_id = match entry_path.file_name().and_then(|n| n.to_str()) {
            Some(name) => name.to_string(),
            None => continue,
        };

        let skill_file = entry_path.join("skill.md");
        if !skill_file.exists() {
            tracing::debug!("Skipping directory without skill.md: {}", entry_path.display());
            continue;
        }

        let raw = match std::fs::read_to_string(&skill_file) {
            Ok(content) => content,
            Err(e) => {
                tracing::warn!("Failed to read {}: {e}", skill_file.display());
                continue;
            }
        };

        match parse_skill(&skill_id, &raw) {
            Ok(skill) => {
                tracing::info!("Loaded skill: {} ({})", skill.name, skill.id);
                skills.insert(skill_id, skill);
            }
            Err(e) => {
                tracing::warn!("Failed to parse skill '{}': {e}", skill_id);
            }
        }
    }

    tracing::info!("Loaded {} skills from {}", skills.len(), path.display());
    Ok(skills)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    fn create_test_skill_dir(
        base: &Path,
        name: &str,
        content: &str,
    ) {
        let dir = base.join(name);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("skill.md"), content).unwrap();
    }

    #[test]
    fn test_load_from_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();

        create_test_skill_dir(
            base,
            "skill-a",
            "---\nname: Skill A\ndescription: First skill\n---\n# Skill A\nContent A",
        );
        create_test_skill_dir(
            base,
            "skill-b",
            "---\nname: Skill B\ndescription: Second skill\n---\n# Skill B\nContent B",
        );

        let skills = load_skills_from_directory(base).unwrap();
        assert_eq!(skills.len(), 2);
        assert!(skills.contains_key("skill-a"));
        assert!(skills.contains_key("skill-b"));
        assert_eq!(skills["skill-a"].name, "Skill A");
    }

    #[test]
    fn test_load_skips_invalid_skills() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();

        create_test_skill_dir(base, "good-skill", "---\nname: Good\ndescription: Works\n---\n# Good");
        create_test_skill_dir(base, "bad-skill", "No frontmatter here");

        let skills = load_skills_from_directory(base).unwrap();
        assert_eq!(skills.len(), 1);
        assert!(skills.contains_key("good-skill"));
    }

    #[test]
    fn test_load_skips_dirs_without_skill_md() {
        let tmp = tempfile::tempdir().unwrap();
        let base = tmp.path();

        // Directory without skill.md
        fs::create_dir_all(base.join("empty-dir")).unwrap();
        create_test_skill_dir(
            base,
            "real-skill",
            "---\nname: Real\ndescription: Has content\n---\n# Real",
        );

        let skills = load_skills_from_directory(base).unwrap();
        assert_eq!(skills.len(), 1);
    }

    #[test]
    fn test_load_nonexistent_directory() {
        let result = load_skills_from_directory(Path::new("/nonexistent/path"));
        assert!(result.is_err());
    }

    #[test]
    fn test_load_empty_directory() {
        let tmp = tempfile::tempdir().unwrap();
        let skills = load_skills_from_directory(tmp.path()).unwrap();
        assert!(skills.is_empty());
    }
}
