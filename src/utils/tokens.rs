use anyhow::Result;
use tiktoken_rs::cl100k_base;

pub fn count_tokens(text: &str) -> Result<usize> {
    let bpe = cl100k_base()?;
    let tokens = bpe.encode_with_special_tokens(text);
    Ok(tokens.len())
}

/// Cost Estimate for a specific model
pub struct CostEstimate {
    pub model_name: String,
    pub cost_usd: f64,
}

/// Prices per 1 Million Input Tokens (as of Jan 2025)
const PRICE_GPT4O: f64 = 2.50;
const PRICE_CLAUDE_35_SONNET: f64 = 3.00;
const PRICE_GEMINI_15_PRO: f64 = 1.25;

pub fn estimate_cost(token_count: usize) -> Vec<CostEstimate> {
    let tokens_m = token_count as f64 / 1_000_000.0;

    vec![
        CostEstimate {
            model_name: "GPT-4o".to_string(),
            cost_usd: tokens_m * PRICE_GPT4O,
        },
        CostEstimate {
            model_name: "Claude 3.5 Sonnet".to_string(),
            cost_usd: tokens_m * PRICE_CLAUDE_35_SONNET,
        },
        CostEstimate {
            model_name: "Gemini 1.5 Pro".to_string(),
            cost_usd: tokens_m * PRICE_GEMINI_15_PRO,
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_estimate_cost() {
        let tokens = 1_000_000;
        let estimates = estimate_cost(tokens);

        // Check GPT-4o
        let gpt4o = estimates.iter().find(|e| e.model_name == "GPT-4o").unwrap();
        assert!((gpt4o.cost_usd - 2.50).abs() < 0.001);

        // Check Claude
        let claude = estimates
            .iter()
            .find(|e| e.model_name == "Claude 3.5 Sonnet")
            .unwrap();
        assert!((claude.cost_usd - 3.00).abs() < 0.001);
    }

    #[test]
    fn test_count_tokens_ascii() {
        let text = "hello world";
        // "hello" " world" -> 2 tokens usually in cl100k_base
        let count = count_tokens(text).unwrap();
        assert!(count > 0);
    }

    #[test]
    fn test_count_tokens_special() {
        let text = "fn main() { println!(\"test\"); }";
        let count = count_tokens(text).unwrap();
        assert!(count > 5); // Rough check
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(count_tokens("").unwrap(), 0);
    }
}
