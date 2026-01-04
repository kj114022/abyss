/// Checks if the buffer contains binary data.
/// Uses a simple heuristic: looks for null bytes in the first 8KB.
pub fn is_binary(content: &[u8]) -> bool {
    let check_len = content.len().min(8192);
    content[..check_len].contains(&0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_binary() {
        assert!(is_binary(b"\x00\x01\x02"));
        assert!(!is_binary(b"Hello World"));
        assert!(!is_binary("Hello\nWorld".as_bytes()));
    }
}
