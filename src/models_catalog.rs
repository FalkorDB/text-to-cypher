//! Curated fallback catalog of model names per AI provider.
//!
//! `genai`'s `Client::all_model_names` is *dynamic*: for hosted providers such as
//! `OpenAI`, Anthropic, Gemini and Groq it performs a live, authenticated HTTP request
//! to the provider's `/models` endpoint. Without a valid API key those calls fail, so
//! no models can be listed. A few adapters (e.g. Cohere) return a hardcoded list and
//! work without a key, and Ollama lists models from a local daemon.
//!
//! To give callers a useful list even when no key is configured, this module maintains
//! a small, curated set of well-known model names per provider. These lists are merged
//! with the dynamic results (see [`crate::core::list_adapter_models`]).
//!
//! This is a *curated convenience catalog*, not an authoritative list of everything the
//! provider or `genai` supports. `genai` routes a model to an adapter by name prefix
//! (e.g. `gpt*` → `OpenAI`, `claude*` → Anthropic), so additional models not listed here
//! still work as long as they use the expected prefix.
//!
//! Last reviewed against `genai` 0.6.3. Keep these lists short and high-confidence;
//! update them when adding support for newer models.

use genai::adapter::AdapterKind;

/// Curated `OpenAI` model names.
const OPENAI_MODELS: &[&str] = &[
    "gpt-4o",
    "gpt-4o-mini",
    "gpt-4.1",
    "gpt-4.1-mini",
    "gpt-4-turbo",
    "gpt-5",
    "gpt-5-mini",
    "gpt-5-codex",
    "o1",
    "o1-mini",
    "o3",
    "o3-mini",
    "o4-mini",
];

/// Curated Anthropic model names.
const ANTHROPIC_MODELS: &[&str] = &[
    "claude-opus-4-6",
    "claude-sonnet-4-6",
    "claude-opus-4-5",
    "claude-3-7-sonnet-latest",
    "claude-3-5-sonnet-latest",
    "claude-3-5-haiku-latest",
    "claude-3-haiku-20240307",
];

/// Curated Google Gemini model names.
const GEMINI_MODELS: &[&str] = &[
    "gemini-2.5-pro",
    "gemini-2.5-flash",
    "gemini-2.0-flash",
    "gemini-1.5-pro",
    "gemini-1.5-flash",
];

/// Curated Groq model names.
const GROQ_MODELS: &[&str] = &[
    "llama-3.3-70b-versatile",
    "llama-3.1-8b-instant",
    "mixtral-8x7b-32768",
    "gemma2-9b-it",
];

/// Curated `DeepSeek` model names.
const DEEPSEEK_MODELS: &[&str] = &["deepseek-chat", "deepseek-reasoner"];

/// Curated xAI (Grok) model names.
const XAI_MODELS: &[&str] = &["grok-4", "grok-3", "grok-3-mini", "grok-2-vision"];

/// Curated Cohere model names (mirrors `genai`'s static Cohere list).
const COHERE_MODELS: &[&str] = &[
    "command-r-plus",
    "command-r",
    "command",
    "command-nightly",
    "command-light",
    "command-light-nightly",
];

/// Returns the curated, statically-known model names for a given provider.
///
/// Returns an empty slice for providers without a curated list (e.g. Ollama, which is
/// listed from a local daemon, or any adapter not covered here).
#[must_use]
pub const fn static_models(adapter_kind: AdapterKind) -> &'static [&'static str] {
    match adapter_kind {
        AdapterKind::OpenAI => OPENAI_MODELS,
        AdapterKind::Anthropic => ANTHROPIC_MODELS,
        AdapterKind::Gemini => GEMINI_MODELS,
        AdapterKind::Groq => GROQ_MODELS,
        AdapterKind::DeepSeek => DEEPSEEK_MODELS,
        AdapterKind::Xai => XAI_MODELS,
        AdapterKind::Cohere => COHERE_MODELS,
        _ => &[],
    }
}

/// Returns a curated static model list to use when a dynamic listing fails.
///
/// Returns `Some` (the curated list as owned strings) when the provider has a curated
/// catalog, or `None` when it does not — in which case the caller should propagate the
/// original dynamic error.
#[must_use]
pub fn static_fallback(statics: &[&str]) -> Option<Vec<String>> {
    if statics.is_empty() {
        None
    } else {
        Some(merge_models(Vec::new(), statics))
    }
}

/// Merges dynamically-fetched model names with the curated static list.
///
/// Dynamic models keep their original (provider-native) ordering and appear first.
/// Any curated model not already present is appended afterwards. De-duplication is an
/// exact, case-sensitive match, so distinct aliases (e.g. `*-latest` vs a dated
/// snapshot) are preserved.
#[must_use]
pub fn merge_models(
    dynamic: Vec<String>,
    statics: &[&str],
) -> Vec<String> {
    let mut merged = dynamic;
    for &model in statics {
        if !merged.iter().any(|existing| existing == model) {
            merged.push(model.to_string());
        }
    }
    merged
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn static_models_known_providers_non_empty() {
        for kind in [
            AdapterKind::OpenAI,
            AdapterKind::Anthropic,
            AdapterKind::Gemini,
            AdapterKind::Groq,
            AdapterKind::Cohere,
        ] {
            assert!(!static_models(kind).is_empty(), "{kind} should have curated models");
        }
    }

    #[test]
    fn static_models_unknown_provider_empty() {
        assert!(static_models(AdapterKind::Ollama).is_empty());
    }

    #[test]
    fn merge_preserves_dynamic_order_and_appends_missing_statics() {
        let dynamic = vec!["gpt-4o".to_string(), "gpt-custom".to_string()];
        let statics = &["gpt-4o", "gpt-5"];
        let merged = merge_models(dynamic, statics);

        assert_eq!(merged, vec!["gpt-4o", "gpt-custom", "gpt-5"]);
    }

    #[test]
    fn merge_dedups_exact_matches_only() {
        let dynamic = vec!["claude-3-5-sonnet-latest".to_string()];
        let statics = &["claude-3-5-sonnet-latest", "claude-3-5-sonnet-20241022"];
        let merged = merge_models(dynamic, statics);

        // Exact duplicate dropped, the distinct dated alias kept.
        assert_eq!(merged, vec!["claude-3-5-sonnet-latest", "claude-3-5-sonnet-20241022"]);
    }

    #[test]
    fn merge_with_empty_dynamic_returns_statics() {
        let merged = merge_models(Vec::new(), &["command-r", "command"]);
        assert_eq!(merged, vec!["command-r", "command"]);
    }

    #[test]
    fn static_fallback_some_when_catalog_present() {
        assert_eq!(
            static_fallback(&["a", "b"]),
            Some(vec!["a".to_string(), "b".to_string()])
        );
    }

    #[test]
    fn static_fallback_none_when_catalog_empty() {
        assert_eq!(static_fallback(&[]), None);
    }
}
