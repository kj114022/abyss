use tree_sitter::{Parser, Query, QueryCursor};

/// Generates a brief summary of the file content based on its extension.
/// extracted symbols (structs, functions, classes).
pub fn summarize_content(content: &str, extension: &str) -> Option<String> {
    let mut parser = Parser::new();

    let (language, query_str) = match extension {
        "rs" => (
            tree_sitter_rust::LANGUAGE.into(),
            r#"
            (struct_item name: (_) @struct)
            (enum_item name: (_) @enum)
            (trait_item name: (_) @trait)
            (function_item name: (_) @fn)
            (impl_item type: (_) @impl)
            "#,
        ),
        "py" => (
            tree_sitter_python::LANGUAGE.into(),
            r#"
            (class_definition name: (_) @class)
            (function_definition name: (_) @fn)
            "#,
        ),
        "js" | "jsx" | "ts" | "tsx" => (
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(), // Use TS for both for now or split?
            // TS language usually handles JS fine.
            r#"
            (class_declaration name: (_) @class)
            (function_declaration name: (_) @fn)
            (interface_declaration name: (_) @interface)
            (type_alias_declaration name: (_) @type)
            "#,
        ),
        "go" => (
            tree_sitter_go::LANGUAGE.into(),
            r#"
            (type_spec name: (_) @type)
            (function_declaration name: (_) @fn)
            "#,
        ),
        "c" | "h" => (
            tree_sitter_c::LANGUAGE.into(),
            r#"
            (struct_specifier name: (_) @struct)
            (function_definition declarator: (function_declarator declarator: (identifier) @fn))
            "#,
        ),
        "cpp" | "hpp" | "cc" | "cxx" => (
            tree_sitter_cpp::LANGUAGE.into(),
            r#"
            (class_specifier name: (_) @class)
            (function_definition declarator: (function_declarator declarator: (identifier) @fn))
            "#,
        ),
        _ => return summarize_content_regex(content),
    };

    if parser.set_language(&language).is_err() {
        return None;
    }

    let tree = parser.parse(content, None)?;
    let query = Query::new(&language, query_str).ok()?;
    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

    let mut structs = Vec::new();
    let mut enums = Vec::new();
    let mut traits = Vec::new();
    let mut fns = Vec::new();
    let mut classes = Vec::new();

    use streaming_iterator::StreamingIterator;
    while let Some(m) = matches.next() {
        for capture in m.captures {
            let capture_name = query.capture_names()[capture.index as usize];
            if let Ok(text) = capture.node.utf8_text(content.as_bytes()) {
                // For some items we might want just the name
                // The query captures the *name* node usually, based on the query above.
                // e.g. (struct_item name: (_) @struct) captures the identifier.

                // However, for `impl`, we capture the type.
                match capture_name {
                    "struct" | "type" => structs.push(text),
                    "enum" => enums.push(text),
                    "trait" | "interface" => traits.push(text),
                    "fn" => fns.push(text),
                    "class" => classes.push(text),
                    "impl" => {
                        // "impl Foo" -> Foo
                        structs.push(text); // Treat impls as related to structs
                    }
                    _ => {}
                }
            }
        }
    }

    // Build summary string
    let mut parts = Vec::new();

    // Helper to format list
    let format_list = |label: &str, mut items: Vec<&str>| -> String {
        items.sort();
        items.dedup();
        if items.is_empty() {
            return String::new();
        }
        // Limit to 5 items to keep it brief
        let count = items.len();
        let display_items: Vec<_> = items.into_iter().take(5).collect();
        let mut s = format!("{}: {}", label, display_items.join(", "));
        if count > 5 {
            s.push_str(&format!(" (+{})", count - 5));
        }
        s
    };

    if !structs.is_empty() {
        parts.push(format_list("Structs/Types", structs));
    }
    if !classes.is_empty() {
        parts.push(format_list("Classes", classes));
    }
    if !enums.is_empty() {
        parts.push(format_list("Enums", enums));
    }
    if !traits.is_empty() {
        parts.push(format_list("Traits/Interfaces", traits));
    }

    // Functions are usually too many. Only show if no types?
    // Or just count?
    // "Functions: run, build, test (+12)"
    if !fns.is_empty() {
        parts.push(format_list("Functions", fns));
    }

    if parts.is_empty() {
        return None;
    }

    Some(
        parts
            .into_iter()
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join("; "),
    )
}

/// Fallback summarizer using Regex for unknown languages
fn summarize_content_regex(content: &str) -> Option<String> {
    use regex::Regex;
    // Compile regexes once or lazily? For now compiled inside.
    let re_class = Regex::new(r"^\s*(class|struct|module|interface|trait)\s+([a-zA-Z0-9_]+)").ok()?;
    let re_fn = Regex::new(r"^\s*(function|def|fn|func|public\s+sub|sub)\s+([a-zA-Z0-9_]+)").ok()?;

    let mut classes = Vec::new();
    let mut functions = Vec::new();

    for line in content.lines() {
        if let Some(caps) = re_class.captures(line) {
            if let Some(name) = caps.get(2) {
                classes.push(name.as_str());
            }
        } else if let Some(caps) = re_fn.captures(line) {
             #[allow(clippy::collapsible_if)]
             if let Some(name) = caps.get(2) {
                functions.push(name.as_str());
            }
        }
    }

     let format_list = |label: &str, mut items: Vec<&str>| -> String {
        items.sort();
        items.dedup();
        if items.is_empty() { return String::new(); }
        let count = items.len();
        let display: Vec<_> = items.into_iter().take(3).collect();
        let mut s = format!("{}: {}", label, display.join(", "));
        if count > 3 { s.push_str(&format!(" (+{})", count - 3)); }
        s
    };

    let mut parts = Vec::new();
    if !classes.is_empty() { parts.push(format_list("Classes/Modules", classes)); }
    if !functions.is_empty() { parts.push(format_list("Functions", functions)); }

    if parts.is_empty() { return None; }
    Some(parts.join("; ") + " (Heuristic)")
}

/// Extracts a purpose statement from a README content.
/// Returns the first non-header, non-link, non-empty line.
pub fn extract_readme_purpose(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty()
            && !trimmed.starts_with('#')
            && !trimmed.starts_with("<!--")
            && !trimmed.starts_with('!')
            && !trimmed.starts_with('[')
            && !trimmed.starts_with('`')
        {
            return Some(trimmed.to_string());
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_summary_ruby() {
        let content = r#"
        class User
          def login
          end
        end
        "#;
        let summary = summarize_content(content, "rb").unwrap();
        assert!(summary.contains("Classes/Modules: User"));
        assert!(summary.contains("Functions: login"));
        assert!(summary.contains("(Heuristic)"));
    }

    #[test]
    fn test_regex_summary_c() {
        let content = r#"
        struct Point { int x; };
        int main() { return 0; }
        "#;
        // Should use tree-sitter-c, NOT heuristic
        let summary = summarize_content(content, "c").unwrap();
        assert!(summary.contains("Functions: main"));
        assert!(!summary.contains("(Heuristic)"));
    }
}
