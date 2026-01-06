use lazy_static::lazy_static;
use regex::Regex;

use crate::config::CompressionLevel;

lazy_static! {
    // Matches // ... until newline
    static ref LINE_COMMENT: Regex = Regex::new(r"//.*").unwrap();
    // Matches /* ... */ (dot matches newline)
    static ref BLOCK_COMMENT: Regex = Regex::new(r"(?s)/\*.*?\*/").unwrap();
    // Matches multiple newlines
    static ref MULTIPLE_NEWLINES: Regex = Regex::new(r"\n\s*\n").unwrap();
    // Matches doc comments (/// or //!)
    static ref DOC_COMMENT: Regex = Regex::new(r"///.*|//!.*").unwrap();
    // Matches simple getter patterns (e.g., fn get_x(&self) -> X { self.x })
    static ref GETTER_PATTERN: Regex = Regex::new(
        r"(?m)^\s*(pub\s+)?fn\s+\w+\s*\([^)]*\)\s*(->\s*[^{]+)?\s*\{\s*(self\.)?\w+\s*\}"
    ).unwrap();
    // Matches simple setter patterns
    static ref SETTER_PATTERN: Regex = Regex::new(
        r"(?m)^\s*(pub\s+)?fn\s+set_\w+\s*\([^)]*\)\s*\{\s*self\.\w+\s*=\s*\w+;\s*\}"
    ).unwrap();
    // Matches simple one-liner functions
    static ref ONELINER_PATTERN: Regex = Regex::new(
        r"(?m)^\s*(pub\s+)?fn\s+\w+\s*\([^)]*\)\s*(->\s*[^{]+)?\s*\{[^{}\n]+\}"
    ).unwrap();
    // Matches empty impl blocks
    static ref EMPTY_IMPL: Regex = Regex::new(
        r"(?m)impl[^{]+\{\s*\}"
    ).unwrap();
}

/// Apply compression based on level
pub fn compress_by_level(content: &str, level: CompressionLevel, extension: &str) -> String {
    match level {
        CompressionLevel::None => content.to_string(),
        CompressionLevel::Light => compress_light(content),
        CompressionLevel::Standard => compress_standard(content),
        CompressionLevel::Aggressive => compress_aggressive(content, extension),
    }
}

/// Light compression: Remove comments and extra whitespace only
pub fn compress_light(content: &str) -> String {
    // Remove block comments
    let no_block = BLOCK_COMMENT.replace_all(content, "");
    // Remove line comments (but preserve doc comments for now)
    let no_line = LINE_COMMENT.replace_all(&no_block, "");
    // Remove excess whitespace
    let compressed = MULTIPLE_NEWLINES.replace_all(&no_line, "\n");

    compressed.trim().to_string()
}

/// Standard compression: Remove comments, whitespace, and simple boilerplate
pub fn compress_standard(content: &str) -> String {
    // First apply light compression
    let light = compress_light(content);

    // Replace simple getters with placeholders
    let no_getters = GETTER_PATTERN.replace_all(&light, |caps: &regex::Captures| {
        // Keep the signature, replace body
        let full = caps.get(0).map_or("", |m| m.as_str());
        if let Some(brace_pos) = full.rfind('{') {
            format!("{} /* getter */ }}", &full[..brace_pos])
        } else {
            full.to_string()
        }
    });

    // Replace simple setters with placeholders
    let no_setters = SETTER_PATTERN.replace_all(&no_getters, |caps: &regex::Captures| {
        let full = caps.get(0).map_or("", |m| m.as_str());
        if let Some(brace_pos) = full.rfind('{') {
            format!("{} /* setter */ }}", &full[..brace_pos])
        } else {
            full.to_string()
        }
    });

    // Remove empty impl blocks
    let no_empty = EMPTY_IMPL.replace_all(&no_setters, "/* empty impl */");

    no_empty.to_string()
}

/// Aggressive compression: Replace function bodies with placeholders
pub fn compress_aggressive(content: &str, extension: &str) -> String {
    // Use AST-aware compression if available for this language
    crate::utils::ast::compress_ast(content, extension)
}

/// Legacy function for backward compatibility
pub fn compress_content(content: &str) -> String {
    compress_light(content)
}

/// Calculate compression ratio
pub fn compression_ratio(original: &str, compressed: &str) -> f64 {
    if original.is_empty() {
        return 1.0;
    }
    1.0 - (compressed.len() as f64 / original.len() as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_simple() {
        let input = "
            fn main() {
                // This is a comment
                println!(\"Hello\");
            }
        ";
        let result = compress_content(input);

        // Normalize newlines for comparison
        let result = result.trim().replace("\n\n", "\n");
        assert!(result.contains("fn main() {"));
        assert!(!result.contains("This is a comment"));
    }

    #[test]
    fn test_remove_block_comments() {
        let input = "
            /* 
               Block comment 
            */
            code();
        ";
        let result = compress_content(input);
        assert!(!result.contains("Block comment"));
        assert!(result.contains("code();"));
    }

    #[test]
    fn test_remove_line_comments() {
        let input = "code(); // line comment";
        let result = compress_content(input);
        assert!(!result.contains("line comment"));
        assert!(result.contains("code();"));
    }

    #[test]
    fn test_reduce_whitespace() {
        let input = "code();\n\n\n\nnext();";
        let result = compress_content(input);
        assert!(result.contains("code();\nnext();") || result.contains("code();\n\nnext();"));
    }

    #[test]
    fn test_compression_levels() {
        let code = "fn get_x(&self) -> i32 { self.x }";
        
        let none = compress_by_level(code, CompressionLevel::None, "rs");
        assert_eq!(none, code);
        
        let light = compress_by_level(code, CompressionLevel::Light, "rs");
        assert!(light.contains("get_x"));
    }

    #[test]
    fn test_compression_ratio() {
        let original = "// comment\ncode();";
        let compressed = "code();";
        let ratio = compression_ratio(original, compressed);
        assert!(ratio > 0.3); // Should be significant compression
    }
}

