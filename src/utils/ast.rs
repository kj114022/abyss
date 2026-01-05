use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};

/// AST-aware compression that replaces function bodies with placeholders.
/// Preserves function signatures, type definitions, and interfaces for LLM context.
pub fn compress_ast(content: &str, extension: &str) -> String {
    let language: tree_sitter::Language = match extension {
        "rs" => tree_sitter_rust::LANGUAGE.into(),
        "js" | "jsx" => tree_sitter_javascript::LANGUAGE.into(),
        "ts" | "tsx" => tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
        "py" => tree_sitter_python::LANGUAGE.into(),
        "go" => tree_sitter_go::LANGUAGE.into(),
        "c" | "h" => tree_sitter_c::LANGUAGE.into(),
        "cpp" | "hpp" | "cc" | "cxx" => tree_sitter_cpp::LANGUAGE.into(),
        _ => return content.to_string(),
    };

    let mut parser = Parser::new();
    if parser.set_language(&language).is_err() {
        return content.to_string();
    }

    let tree = match parser.parse(content, None) {
        Some(t) => t,
        None => return content.to_string(),
    };

    // Language-specific queries for function bodies
    let query_str = match extension {
        "rs" => {
            r#"
            (function_item body: (block) @body)
        "#
        }
        "js" | "jsx" | "ts" | "tsx" => {
            r#"
            (function_declaration body: (statement_block) @body)
            (method_definition body: (statement_block) @body)
            (arrow_function body: (statement_block) @body)
        "#
        }
        "py" => {
            r#"
            (function_definition body: (block) @body)
        "#
        }
        "go" => {
            r#"
            (function_declaration body: (block) @body)
            (method_declaration body: (block) @body)
        "#
        }
        "c" | "h" | "cpp" | "hpp" | "cc" | "cxx" => {
            r#"
            (function_definition body: (compound_statement) @body)
        "#
        }
        _ => return content.to_string(),
    };

    let query = match Query::new(&language, query_str) {
        Ok(q) => q,
        Err(e) => {
            eprintln!("Query error: {:?}", e);
            return content.to_string();
        }
    };

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

    let mut ranges_to_replace = Vec::new();

    while let Some(m) = matches.next() {
        for capture in m.captures {
            let node = capture.node;
            let start = node.start_byte();
            let end = node.end_byte();

            // Replace entire block node with placeholder

            ranges_to_replace.push((start, end));
        }
    }

    // Sort reverse to replace without offset issues
    ranges_to_replace.sort_by(|a, b| b.0.cmp(&a.0));
    // Sort by start ASC, end DESC to prefer outermost ranges
    // Filter nested ranges: only keep non-overlapping outermost matches
    // If we pick one, we skip all subsequent that start before its end.

    // Implementation:
    ranges_to_replace.sort_by(|a, b| a.0.cmp(&b.0).then(b.1.cmp(&a.1)));

    let mut final_ranges = Vec::new();
    let mut last_end = 0;

    for (start, end) in ranges_to_replace {
        if start >= last_end {
            final_ranges.push((start, end));
            last_end = end;
        }
    }

    // Now reverse for replacement
    final_ranges.reverse();

    let mut result = content.to_string();
    for (start, end) in final_ranges {
        // Safety check bounds
        if start >= result.len() || end > result.len() {
            continue;
        }

        // We replace with `{ /* ... */ }`.
        // But the node includes braces?
        // Rust `(block)` includes `{}`.
        // JS `(statement_block)` includes `{}`.
        // So we replace the whole range.

        result.replace_range(start..end, "{ /* ... */ }");
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rust_compression() {
        let code = r#"
fn complex_logic(x: i32) -> i32 {
    let y = x + 1;
    println!("Calculating...");
    y * 2
}

struct Data {
    id: u32,
}
"#;
        let compressed = compress_ast(code, "rs");
        assert!(compressed.contains("fn complex_logic(x: i32) -> i32"));
        assert!(compressed.contains("{ /* ... */ }"));
        assert!(compressed.contains("struct Data"));
        assert!(!compressed.contains("let y = x + 1"));
    }

    #[test]
    fn test_js_compression() {
        let code = r#"
function doWork() {
    console.log("Work");
    return true;
}
"#;
        let compressed = compress_ast(code, "js");
        assert!(compressed.contains("function doWork() { /* ... */ }"));
        assert!(!compressed.contains("console.log"));
    }

    #[test]
    fn test_rust_impl_compression() {
        let code = r#"
struct Foo;
impl Foo {
    fn bar(&self) {
        println!("baz");
    }
}
"#;
        let compressed = compress_ast(code, "rs");
        assert!(compressed.contains("fn bar(&self) { /* ... */ }"));
        assert!(!compressed.contains("println"));
    }
}
