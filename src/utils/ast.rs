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

            // Heuristic: Only compress if body > 50 chars? Or always?
            // User wants to see interfaces. Empty body `{}` is fine.
            // But if we replace `{ ... }` with `{ /* ... */ }` it's uniform.
            // Let's replace the CONTENT of the block, keeping braces?
            // Queries match the whole `block`.
            // `block` in Rust includes `{` and `}`.
            // So we replace the whole node with `{ /* ... */ }`.

            ranges_to_replace.push((start, end));
        }
    }

    // Sort reverse to replace without offset issues
    ranges_to_replace.sort_by(|a, b| b.0.cmp(&a.0));
    // Deduplicate (nested blocks? Query matches top level? Tree-sitter query might match nested.
    // If we replace outer, inner is gone. That's fine.
    // But if we replace inner first, then outer, we waste work.
    // If we replace outer, we are good.
    // We should ensure we don't do overlapping replacements.
    // If we process in reverse start order?
    // And check if end is within previous (which is "later" in code) processed range.
    // No, reverse start order means we see (End of file) items first.
    // (Start 100, End 200) -> processed.
    // (Start 10, End 300) -> If this overlaps 100-200, it contains it.
    // We should merge or skip contained.

    // Simpler:
    // tree-sitter matches might be nested.
    // We want to collapse the *largest* functions?
    // Actually, we usually want to collapse *implementations*.
    // `function_item` body is the implementation.
    // If there is a helper function *inside*, it gets collapsed with the outer one.
    // So we should prioritize outermost.
    // Top-down?
    // But we need to replace strings.
    // Let's filter ranges: if a range is contained in another, ignore it?
    // Actually if we replace outer, inner doesn't matter.
    // So we just take top-level matches?
    // Tree-sitter query returns all.
    // We can filter.

    // Logic:
    // 1. Sort by start byte ascending.
    // 2. Iterate. If current range is inside previous range's end, skip it.
    // But we need to replace from back.
    // So:
    // 1. Identify non-overlapping ranges (outermost prefered).
    // 2. Sort those by start byte descending for replacement.

    // Identifying outermost:
    // Sort by start ASC, then end DESC (largest first).
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
