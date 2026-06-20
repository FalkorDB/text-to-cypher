//! Compile-time-embedded, curated read-only `FalkorDB` Cypher skills.
//!
//! These ship inside the crate so every consumer — library, napi bindings, browser, and server — gets
//! `FalkorDB`-specific Cypher context by default, not only the Docker image (which loads skills from
//! `SKILLS_DIR`). See `assets/cypher-skills/SOURCE.md` for provenance and curation rules.

use super::parser::{Skill, parse_skill};
use std::collections::HashMap;

/// Curated, read-only `FalkorDB` Cypher skills embedded at compile time.
///
/// Each entry is `(stable_id, raw skill.md)`. The set is deliberately read-only (issue #82 option A):
/// no DDL/write or operational/admin skills are bundled.
const BUILTIN_SKILLS: &[(&str, &str)] = &[
    (
        "falkordb-index-aware-predicates",
        include_str!("../../assets/cypher-skills/falkordb-index-aware-predicates/skill.md"),
    ),
    (
        "falkordb-fulltext-search",
        include_str!("../../assets/cypher-skills/falkordb-fulltext-search/skill.md"),
    ),
    (
        "falkordb-vector-search",
        include_str!("../../assets/cypher-skills/falkordb-vector-search/skill.md"),
    ),
    (
        "falkordb-parameterized-queries",
        include_str!("../../assets/cypher-skills/falkordb-parameterized-queries/skill.md"),
    ),
    (
        "falkordb-path-finding",
        include_str!("../../assets/cypher-skills/falkordb-path-finding/skill.md"),
    ),
];

/// Stable IDs of upstream `FalkorDB/skills` cypher-skills that perform writes/DDL. Under
/// [`SkillProfile::ReadOnly`] these are filtered out of any externally supplied catalog so the
/// read-only contract holds for the server's `SKILLS_DIR` override too, not just the built-in set.
const WRITE_SKILL_IDS: &[&str] = &[
    "create-range-indexes",
    "create-and-query-fulltext-indexes",
    "create-and-query-vector-indexes",
    "manage-constraints",
    "create-nodes-and-relationships",
    "update-and-remove-properties",
    "use-merge-to-avoid-duplicates",
];

/// Which skills are eligible for inclusion in the prompt.
///
/// Today text-to-cypher only generates read-only queries, so [`SkillProfile::ReadOnly`] is the only
/// profile. A future write/DDL generation mode would add its own variant.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SkillProfile {
    /// Only read-only skills. Known write/DDL skills are filtered out.
    #[default]
    ReadOnly,
}

/// Returns true if `id` names a known write/DDL skill that must be excluded under a read-only profile.
#[must_use]
pub(super) fn is_write_skill(id: &str) -> bool {
    WRITE_SKILL_IDS.contains(&id)
}

/// Returns true if any fenced code block in `content` contains a Cypher write/DDL clause.
///
/// Only fenced code blocks are scanned (not prose), so a read-only skill whose notes merely *mention*
/// write clauses (e.g. "writes such as CREATE are rejected") is not misclassified. This complements the
/// [`is_write_skill`] ID denylist with a best-effort content check for arbitrary external skills; the
/// hard read-only boundary remains `GRAPH.RO_QUERY` at execution time.
#[must_use]
pub(super) fn teaches_write_cypher(content: &str) -> bool {
    const WRITE_CLAUSES: &[&str] = &["CREATE", "MERGE", "DELETE", "SET", "REMOVE", "DROP"];
    let mut in_code_block = false;
    for segment in content.split("```") {
        if in_code_block {
            let normalized = segment.to_uppercase().replace(|c: char| !c.is_ascii_alphanumeric(), " ");
            if normalized.split_whitespace().any(|word| WRITE_CLAUSES.contains(&word)) {
                return true;
            }
        }
        in_code_block = !in_code_block;
    }
    false
}

/// Parse the embedded skills into a map keyed by stable ID.
///
/// Embedded assets are validated by tests (`builtin_skills_all_parse`), so a parse failure here means a
/// malformed vendored file slipped through; it is logged and skipped rather than panicking in a
/// library constructor.
pub(super) fn builtin_skills() -> HashMap<String, Skill> {
    let mut skills = HashMap::with_capacity(BUILTIN_SKILLS.len());
    for (id, raw) in BUILTIN_SKILLS {
        match parse_skill(id, raw) {
            Ok(skill) => {
                skills.insert((*id).to_string(), skill);
            }
            Err(e) => {
                tracing::error!("Built-in skill '{id}' failed to parse: {e}");
            }
        }
    }
    skills
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builtin_skills_all_parse() {
        // Guards against vendoring a malformed skill.md: every embedded asset must parse.
        for (id, raw) in BUILTIN_SKILLS {
            let skill = parse_skill(id, raw).unwrap_or_else(|e| panic!("built-in skill '{id}' must parse: {e}"));
            assert_eq!(&skill.id, id);
            assert!(!skill.name.trim().is_empty(), "skill '{id}' has empty name");
            assert!(
                !skill.description.trim().is_empty(),
                "skill '{id}' has empty description"
            );
            assert!(!skill.content.trim().is_empty(), "skill '{id}' has empty body");
        }
    }

    #[test]
    fn builtin_skills_are_read_only() {
        // None of the bundled skills may be a known write/DDL skill, and none should teach write clauses.
        for (id, raw) in BUILTIN_SKILLS {
            assert!(!is_write_skill(id), "built-in skill '{id}' is a write skill");
            let upper = raw.to_uppercase();
            for clause in ["CREATE ", "MERGE ", "DELETE ", " SET ", "REMOVE ", "DROP "] {
                assert!(
                    !upper.contains(clause),
                    "built-in skill '{id}' must not contain write clause {clause:?}"
                );
            }
        }
    }

    #[test]
    fn builtin_skills_loads_expected_count() {
        let skills = builtin_skills();
        assert_eq!(skills.len(), BUILTIN_SKILLS.len());
        assert!(skills.contains_key("falkordb-fulltext-search"));
    }

    #[test]
    fn write_skill_denylist_matches_upstream_ids() {
        assert!(is_write_skill("create-range-indexes"));
        assert!(is_write_skill("use-merge-to-avoid-duplicates"));
        assert!(!is_write_skill("falkordb-index-aware-predicates"));
        assert!(!is_write_skill("match-patterns-and-return-projections"));
    }

    #[test]
    fn teaches_write_cypher_flags_write_code() {
        assert!(teaches_write_cypher("```cypher\nCREATE (n:X)\n```"));
        assert!(teaches_write_cypher("```\nMATCH (n) SET n.x = 1 RETURN n\n```"));
        assert!(teaches_write_cypher("```cypher\nMATCH (n) DETACH DELETE n\n```"));
    }

    #[test]
    fn teaches_write_cypher_ignores_prose_and_reads() {
        // Prose that merely mentions write clauses must not be flagged.
        assert!(!teaches_write_cypher(
            "Writes such as CREATE, SET, DELETE, MERGE are rejected by RO_QUERY."
        ));
        // Read-only code is not flagged.
        assert!(!teaches_write_cypher(
            "```cypher\nMATCH (n:Person) WHERE n.age > 30 RETURN n.name\n```"
        ));
        // Identifiers that contain a clause as a substring are not flagged.
        assert!(!teaches_write_cypher(
            "```cypher\nMATCH (n) RETURN n.subset, n.created_at\n```"
        ));
    }
}
