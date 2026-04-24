//! Static pricing table for known Claude models.
//!
//! Prices are in USD per 1 million tokens, based on Anthropic's published
//! pricing as of early 2026.
//!
//! # TODO
//! Consider moving these values to a config file so they can be updated
//! without a code change. The static approach is intentional for the initial
//! implementation to keep dependencies minimal.

use crate::backends::ModelId;

/// Per-model token prices in USD per 1 million tokens.
#[derive(Debug, Clone, Copy)]
pub struct ModelPricing {
    /// Price per million input tokens.
    pub input_per_m: f64,
    /// Price per million output tokens.
    pub output_per_m: f64,
    /// Price per million cache-read tokens.
    pub cache_read_per_m: f64,
    /// Price per million cache-creation tokens.
    pub cache_write_per_m: f64,
}

impl ModelPricing {
    /// Compute the total cost in USD for the given token counts.
    pub fn compute_cost(&self, usage: &crate::backends::TokenUsage) -> f64 {
        let m = 1_000_000.0_f64;
        (usage.input_tokens as f64 / m) * self.input_per_m
            + (usage.output_tokens as f64 / m) * self.output_per_m
            + (usage.cache_read_input_tokens as f64 / m) * self.cache_read_per_m
            + (usage.cache_creation_input_tokens as f64 / m) * self.cache_write_per_m
    }
}

/// Look up the pricing for a given `ModelId`. Returns `None` for unknown models
/// (absence is meaningful — do not default to zero).
pub fn pricing_for(model: &ModelId) -> Option<ModelPricing> {
    // Model IDs may include date suffixes (e.g., "claude-haiku-4-5-20251001").
    // We match on the base prefix so both versioned and bare IDs work.
    let id = model.0.as_str();

    // claude-opus-4-7
    if id.starts_with("claude-opus-4-7") {
        return Some(ModelPricing {
            input_per_m: 15.00,
            output_per_m: 75.00,
            cache_read_per_m: 1.50,
            cache_write_per_m: 18.75,
        });
    }

    // claude-sonnet-4-6
    if id.starts_with("claude-sonnet-4-6") {
        return Some(ModelPricing {
            input_per_m: 3.00,
            output_per_m: 15.00,
            cache_read_per_m: 0.30,
            cache_write_per_m: 3.75,
        });
    }

    // claude-haiku-4-5 (with optional date suffix)
    if id.starts_with("claude-haiku-4-5") {
        return Some(ModelPricing {
            input_per_m: 1.00,
            output_per_m: 5.00,
            cache_read_per_m: 0.10,
            cache_write_per_m: 1.25,
        });
    }

    // Legacy aliases still seen in some fixtures
    if id.starts_with("claude-opus-4") {
        return Some(ModelPricing {
            input_per_m: 15.00,
            output_per_m: 75.00,
            cache_read_per_m: 1.50,
            cache_write_per_m: 18.75,
        });
    }

    if id.starts_with("claude-sonnet-4") {
        return Some(ModelPricing {
            input_per_m: 3.00,
            output_per_m: 15.00,
            cache_read_per_m: 0.30,
            cache_write_per_m: 3.75,
        });
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::backends::{ModelId, TokenUsage};

    #[test]
    fn known_model_returns_some_pricing() {
        assert!(pricing_for(&ModelId("claude-opus-4-7".to_string())).is_some());
        assert!(pricing_for(&ModelId("claude-sonnet-4-6".to_string())).is_some());
        assert!(pricing_for(&ModelId("claude-haiku-4-5-20251001".to_string())).is_some());
    }

    #[test]
    fn unknown_model_returns_none() {
        assert!(pricing_for(&ModelId("gpt-4o".to_string())).is_none());
        assert!(pricing_for(&ModelId("unknown-model".to_string())).is_none());
    }

    #[test]
    fn cost_computation_is_correct() {
        let pricing = pricing_for(&ModelId("claude-sonnet-4-6".to_string())).unwrap();
        let usage = TokenUsage {
            input_tokens: 1_000_000,
            output_tokens: 1_000_000,
            cache_read_input_tokens: 0,
            cache_creation_input_tokens: 0,
        };
        // 3.00 + 15.00 = 18.00
        let cost = pricing.compute_cost(&usage);
        assert!((cost - 18.00).abs() < 1e-9, "expected 18.00, got {cost}");
    }
}
