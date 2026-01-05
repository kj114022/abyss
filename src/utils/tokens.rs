use anyhow::Result;
use tiktoken_rs::cl100k_base;

pub fn count_tokens(text: &str) -> Result<usize> {
    let bpe = cl100k_base()?;
    let tokens = bpe.encode_with_special_tokens(text);
    Ok(tokens.len())
}



#[cfg(test)]
mod tests {
    use super::*;



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
