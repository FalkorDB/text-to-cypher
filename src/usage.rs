//! Token usage accounting for text-to-cypher requests.
//!
//! A single request may issue several LLM calls (cypher generation, final answer
//! generation, self-healing retries, and tool-call rounds for skills). [`TokenUsage`]
//! aggregates the prompt, completion, and total token counts across all of those calls
//! so the consumed tokens can be surfaced in the request result.

use genai::chat::Usage;
use serde::{Deserialize, Serialize};
#[cfg(feature = "server")]
use utoipa::ToSchema;

/// Aggregated token usage for a text-to-cypher request.
///
/// Counts are summed across every LLM call made while serving a single request.
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "server", derive(ToSchema))]
#[allow(clippy::struct_field_names)]
pub struct TokenUsage {
    /// Total input (prompt) tokens consumed across all calls.
    pub prompt_tokens: u64,
    /// Total output (completion) tokens produced across all calls.
    pub completion_tokens: u64,
    /// Total tokens consumed across all calls.
    pub total_tokens: u64,
}

impl TokenUsage {
    /// Creates a new, empty [`TokenUsage`] with all counts at zero.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            prompt_tokens: 0,
            completion_tokens: 0,
            total_tokens: 0,
        }
    }

    /// Adds the counts from another [`TokenUsage`] into this one, saturating on overflow.
    pub const fn accumulate(
        &mut self,
        other: &Self,
    ) {
        self.prompt_tokens = self.prompt_tokens.saturating_add(other.prompt_tokens);
        self.completion_tokens = self.completion_tokens.saturating_add(other.completion_tokens);
        self.total_tokens = self.total_tokens.saturating_add(other.total_tokens);
    }

    /// Adds the counts from a genai [`Usage`] into this one, saturating on overflow.
    pub fn add_genai_usage(
        &mut self,
        usage: &Usage,
    ) {
        self.accumulate(&Self::from(usage));
    }
}

/// Clamps a possibly-negative, possibly-missing token count to a non-negative `u64`.
fn clamp(value: Option<i32>) -> u64 {
    value.map_or(0, |v| u64::try_from(v).unwrap_or(0))
}

impl From<&Usage> for TokenUsage {
    fn from(usage: &Usage) -> Self {
        let prompt_tokens = clamp(usage.prompt_tokens);
        let completion_tokens = clamp(usage.completion_tokens);
        // Prefer the provider-reported total; fall back to prompt + completion when absent.
        // A negative provider-reported total is clamped to 0 by `clamp` (not summed).
        let total_tokens = usage
            .total_tokens
            .map_or_else(|| prompt_tokens.saturating_add(completion_tokens), |t| clamp(Some(t)));

        Self {
            prompt_tokens,
            completion_tokens,
            total_tokens,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn usage(
        prompt: Option<i32>,
        completion: Option<i32>,
        total: Option<i32>,
    ) -> Usage {
        Usage {
            prompt_tokens: prompt,
            completion_tokens: completion,
            total_tokens: total,
            prompt_tokens_details: None,
            completion_tokens_details: None,
        }
    }

    #[test]
    fn from_genai_usage_uses_reported_total() {
        let token_usage = TokenUsage::from(&usage(Some(10), Some(5), Some(15)));
        assert_eq!(token_usage.prompt_tokens, 10);
        assert_eq!(token_usage.completion_tokens, 5);
        assert_eq!(token_usage.total_tokens, 15);
    }

    #[test]
    fn from_genai_usage_falls_back_to_sum_when_total_missing() {
        let token_usage = TokenUsage::from(&usage(Some(10), Some(5), None));
        assert_eq!(token_usage.total_tokens, 15);
    }

    #[test]
    fn from_genai_usage_treats_none_and_negative_as_zero() {
        let token_usage = TokenUsage::from(&usage(None, Some(-7), None));
        assert_eq!(token_usage.prompt_tokens, 0);
        assert_eq!(token_usage.completion_tokens, 0);
        assert_eq!(token_usage.total_tokens, 0);
    }

    #[test]
    fn accumulate_sums_all_fields() {
        let mut acc = TokenUsage::new();
        acc.add_genai_usage(&usage(Some(10), Some(5), Some(15)));
        acc.add_genai_usage(&usage(Some(3), Some(2), Some(5)));
        assert_eq!(acc.prompt_tokens, 13);
        assert_eq!(acc.completion_tokens, 7);
        assert_eq!(acc.total_tokens, 20);
    }

    #[test]
    fn accumulate_saturates_on_overflow() {
        let mut acc = TokenUsage {
            prompt_tokens: u64::MAX,
            completion_tokens: 0,
            total_tokens: 0,
        };
        acc.accumulate(&TokenUsage {
            prompt_tokens: 1,
            completion_tokens: 0,
            total_tokens: 0,
        });
        assert_eq!(acc.prompt_tokens, u64::MAX);
    }
}
