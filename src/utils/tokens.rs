use anyhow::Result;
use tiktoken_rs::cl100k_base;

/// Fast token estimation using heuristic (~4 chars per token for code)
/// Use this for filtering/selection where speed matters more than precision
pub fn estimate_tokens(text: &str) -> usize {
    // Heuristic: code averages ~4 characters per token
    let char_estimate = text.len() / 4;

    // Also consider whitespace-delimited words as a floor
    let word_estimate = text.split_whitespace().count();

    // Take the max as conservative estimate
    char_estimate.max(word_estimate)
}

/// Accurate token count using tiktoken (slower but precise)
/// Use this for final output where accuracy matters
pub fn count_tokens(text: &str) -> Result<usize> {
    let bpe = cl100k_base()?;
    let tokens = bpe.encode_with_special_tokens(text);
    Ok(tokens.len())
}

/// Smart token counting: estimate for speed, accurate when needed
pub fn count_tokens_smart(text: &str, need_accuracy: bool) -> usize {
    if need_accuracy {
        count_tokens(text).unwrap_or_else(|_| estimate_tokens(text))
    } else {
        estimate_tokens(text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_count_tokens_ascii() {
        let text = "hello world";
        let count = count_tokens(text).unwrap();
        assert!(count > 0);
    }

    #[test]
    fn test_count_tokens_special() {
        let text = "fn main() { println!(\"test\"); }";
        let count = count_tokens(text).unwrap();
        assert!(count > 5);
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(count_tokens("").unwrap(), 0);
    }

    #[test]
    fn test_estimate_tokens() {
        let text = "fn main() { println!(\"hello world\"); }";
        let estimate = estimate_tokens(text);
        let accurate = count_tokens(text).unwrap();
        // Estimate should be within 50% of accurate
        assert!(estimate > accurate / 2);
        assert!(estimate < accurate * 2);
    }

    #[test]
    fn test_smart_counting() {
        let text = "hello world";
        let fast = count_tokens_smart(text, false);
        let accurate = count_tokens_smart(text, true);
        assert!(fast > 0);
        assert!(accurate > 0);
    }
}
