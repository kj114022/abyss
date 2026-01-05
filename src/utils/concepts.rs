use streaming_iterator::StreamingIterator;
use tree_sitter::{Parser, Query, QueryCursor};

pub fn extract_concepts(content: &str, extension: &str) -> Vec<String> {
    let mut concepts = Vec::new();
    let mut parser = Parser::new();

    let (language, query_str) = match extension {
        "rs" => (
            tree_sitter_rust::LANGUAGE.into(),
            r#"
            (struct_item name: (_) @name)
            (enum_item name: (_) @name)
            (trait_item name: (_) @name)
            (impl_item type: (_) @name)
            (function_item name: (_) @name)
            (mod_item name: (_) @name)
            "#,
        ),
        "py" => (
            tree_sitter_python::LANGUAGE.into(),
            r#"
            (class_definition name: (_) @name)
            (function_definition name: (_) @name)
            "#,
        ),
        "js" | "jsx" => (
            tree_sitter_javascript::LANGUAGE.into(),
            r#"
            (class_declaration name: (_) @name)
            (function_declaration name: (_) @name)
            (variable_declarator name: (_) @name value: (arrow_function))
             "#,
        ),
        "ts" | "tsx" => (
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            r#"
            (class_declaration name: (_) @name)
            (interface_declaration name: (_) @name)
            (function_declaration name: (_) @name)
            (variable_declarator name: (_) @name value: (arrow_function))
             "#,
        ),
        "go" => (
            tree_sitter_go::LANGUAGE.into(),
            r#"
            (type_spec name: (_) @name)
            (function_declaration name: (_) @name)
            (method_declaration name: (_) @name)
             "#,
        ),
        _ => return concepts,
    };

    if parser.set_language(&language).is_err() {
        return concepts;
    }

    let tree = match parser.parse(content, None) {
        Some(t) => t,
        None => return concepts,
    };

    let query = match Query::new(&language, query_str) {
        Ok(q) => q,
        Err(_) => return concepts,
    };

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

    while let Some(m) = matches.next() {
        for capture in m.captures {
            if let Ok(text) = capture.node.utf8_text(content.as_bytes()) {
                // Heuristic: Filter out very short names or common noise
                if text.len() > 1 && text != "_" {
                    // Try to infer type from the query pattern index if we wanted to be fancy,
                    // but for now just the name is a good "concept".
                    // We could also get the parent kind to prefix it like "struct Foo"
                    // but raw names are often enough for LLM anchoring.

                    // Let's try to be a bit smarter and get the kind.
                    let kind = capture
                        .node
                        .parent()
                        .map(|p: tree_sitter::Node| p.kind())
                        .unwrap_or("");
                    let label = match kind {
                        "struct_item" => format!("struct {}", text),
                        "enum_item" => format!("enum {}", text),
                        "trait_item" => format!("trait {}", text),
                        "impl_item" => format!("impl {}", text),
                        "function_item" => format!("fn {}", text),
                        "class_definition" | "class_declaration" => format!("class {}", text),
                        "function_definition" | "function_declaration" => format!("fn {}", text),
                        "interface_declaration" => format!("interface {}", text),
                        _ => text.to_string(),
                    };

                    concepts.push(label);
                }
            }
        }
    }

    // Dedup
    concepts.sort();
    concepts.dedup();

    concepts
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_rust_concepts() {
        let code = r#"
            struct User { id: usize }
            trait Auth { fn login(&self); }
            impl Auth for User { fn login(&self) {} }
            fn helper() {}
        "#;
        let concepts = extract_concepts(code, "rs");
        assert!(concepts.contains(&"struct User".to_string()));
        assert!(concepts.contains(&"trait Auth".to_string()));
        assert!(concepts.contains(&"impl User".to_string()));
        assert!(concepts.contains(&"fn helper".to_string()));
    }

    #[test]
    fn test_extract_python_concepts() {
        let code = r#"
class MyClass:
    def method(self):
        pass

def global_func():
    pass
        "#;
        let concepts = extract_concepts(code, "py");
        assert!(concepts.contains(&"class MyClass".to_string()));
        assert!(concepts.contains(&"fn global_func".to_string()));
        // fn method might be there too.
    }
}
