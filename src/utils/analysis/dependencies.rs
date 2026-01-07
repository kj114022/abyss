use std::collections::HashSet;
use std::path::{Path, PathBuf};
use streaming_iterator::StreamingIterator;
use topological_sort::TopologicalSort;
use tree_sitter::{Parser, Query, QueryCursor};

/// Extracts import statements from file content.
/// Returns a list of imported module names/paths.
pub fn extract_imports(content: &str, extension: &str) -> Vec<String> {
    let mut imports = Vec::new();
    let mut parser = Parser::new();

    let (language, query_str) = match extension {
        "rs" => (
            tree_sitter_rust::LANGUAGE.into(),
            r#"
            (use_declaration argument: (_) @import)
            (mod_item name: (_) @import)
            "#,
        ),
        "py" => (
            tree_sitter_python::LANGUAGE.into(),
            r#"
            (import_statement name: (_) @import)
            (import_from_statement module_name: (_) @import)
            "#,
        ),
        "js" | "jsx" => (
            tree_sitter_javascript::LANGUAGE.into(),
            r#"
            (import_statement source: (string) @import)
            (call_expression function: (identifier) @func arguments: (arguments (string) @import) (#eq? @func "require"))
             "#,
        ),
        "ts" | "tsx" => (
            tree_sitter_typescript::LANGUAGE_TYPESCRIPT.into(),
            r#"
            (import_statement source: (string) @import)
            (call_expression function: (identifier) @func arguments: (arguments (string) @import) (#eq? @func "require"))
             "#,
        ),
        "go" => (
            tree_sitter_go::LANGUAGE.into(),
            r#"
            (import_spec path: (string_literal) @import)
             "#,
        ),
        _ => return imports,
    };

    if parser.set_language(&language).is_err() {
        return imports;
    }

    let tree = match parser.parse(content, None) {
        Some(t) => t,
        None => return imports,
    };

    let query = match Query::new(&language, query_str) {
        Ok(q) => q,
        Err(_) => return imports,
    };

    let mut cursor = QueryCursor::new();
    let mut matches = cursor.matches(&query, tree.root_node(), content.as_bytes());

    while let Some(m) = matches.next() {
        for capture in m.captures {
            // Filter for @import capture.
            // Handles JS require(@func, @import) specifically.
            let capture_name = query.capture_names()[capture.index as usize];
            if capture_name != "import" {
                continue;
            }

            if let Ok(text) = capture.node.utf8_text(content.as_bytes()) {
                let mut clean_text = text.trim_matches(|c| c == '"' || c == '\'').to_string();
                #[allow(clippy::collapsible_if)]
                if extension == "py" {
                    if let Some(idx) = clean_text.find(" as ") {
                        clean_text = clean_text[..idx].to_string();
                    }
                }
                imports.push(clean_text);
            }
        }
    }

    // Normalize and dedup
    imports.sort();
    imports.dedup();
    imports
}

/// Heuristic resolution of import strings to repository paths.
pub fn resolve_import(import: &str, current_file: &Path, repo_root: &Path) -> Option<PathBuf> {
    let extension = current_file
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("");

    match extension {
        "rs" => {
            // crate::utils::foo -> src/utils/foo.rs
            // super::foo -> parent/foo.rs
            // mod foo -> current_dir/foo.rs or current_dir/foo/mod.rs

            let current_dir = current_file.parent().unwrap_or(repo_root);

            if import.starts_with("crate::") {
                let suffix = import.trim_start_matches("crate::").replace("::", "/");
                let candidate = repo_root.join("src").join(format!("{}.rs", suffix));
                if candidate.exists() {
                    return Some(candidate);
                }
                let candidate_mod = repo_root.join("src").join(&suffix).join("mod.rs");
                if candidate_mod.exists() {
                    return Some(candidate_mod);
                }
            } else if !import.contains("::") {
                // simple mod declaration or separate use
                let candidate = current_dir.join(format!("{}.rs", import));
                if candidate.exists() {
                    return Some(candidate);
                }
                let candidate_mod = current_dir.join(import).join("mod.rs");
                if candidate_mod.exists() {
                    return Some(candidate_mod);
                }
            }
            // Relative `super::` resolution is not implemented.
        }
        "js" | "ts" | "jsx" | "tsx" => {
            // Relatives: ./foo, ../bar
            if import.starts_with('.') {
                let current_dir = current_file.parent().unwrap_or(repo_root);
                let base = current_dir.join(import);

                // Try extensions
                for ext in &["ts", "tsx", "js", "jsx", "d.ts"] {
                    let candidate = base.with_extension(ext);
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
                // Index files
                for ext in &["ts", "tsx", "js", "jsx"] {
                    let candidate = base.join(format!("index.{}", ext));
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
            }
        }
        "py" => {
            // from . import foo
            // import foo.bar
            let parts: Vec<&str> = import.split('.').collect();
            if parts.is_empty() {
                return None;
            }

            // Heuristic: maps dots to slashes relative to root.
            let rel_path = import.replace('.', "/");
            let candidate = repo_root.join(format!("{}.py", rel_path));
            if candidate.exists() {
                return Some(candidate);
            }

            let candidate_init = repo_root.join(rel_path).join("__init__.py");
            if candidate_init.exists() {
                return Some(candidate_init);
            }
        }
        _ => {}
    }

    None
}

/// Sorts paths topologically based on dependencies.
/// Assemblies context in dependency order (definitions before usages) to optimize LLM comprehension.
/// If A -> B, B appears before A.
///
/// `topological-sort` crate:
/// `add_dependency(dependency, dependent)`
/// `dependency` must be processed before `dependent`.
/// So if A imports B, B is the dependency, A is the dependent.
/// We call `ts.add_dependency(B, A)`.
pub fn sort_paths_topologically<F>(
    paths: &[PathBuf],
    repo_root: &Path,
    mut comparator: F,
) -> Vec<PathBuf>
where
    F: FnMut(&PathBuf, &PathBuf) -> std::cmp::Ordering,
{
    let mut ts = TopologicalSort::<PathBuf>::new();
    let path_set: HashSet<PathBuf> = paths.iter().cloned().collect();

    // Seed the graph with all files to ensure isolated files are included
    for path in paths {
        ts.insert(path.clone());
    }

    for path in paths {
        if let Ok(content) = std::fs::read_to_string(path) {
            let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            let imports = extract_imports(&content, extension);

            for import in imports {
                if let Some(dep_path) = resolve_import(&import, path, repo_root) {
                    // Resolve if the dependency is within the scanned set.
                    // Performs a strict matching check on canonical paths.
                    if let Ok(canon_dep) = dep_path.canonicalize() {
                        if path_set.contains(&canon_dep) && canon_dep != *path {
                            // dependency -> dependent
                            // canon_dep -> path
                            ts.add_dependency(canon_dep, path.clone());
                        }
                    } else if path_set.contains(&dep_path) && dep_path != *path {
                        ts.add_dependency(dep_path, path.clone());
                    }
                }
            }
        }
    }

    let mut result = Vec::new();
    // pop_all() returns items with 0 dependencies.
    // We loop until empty.
    loop {
        let mut chunk = ts.pop_all();
        if chunk.is_empty() {
            if !ts.is_empty() {
                break;
            }
            break;
        }
        chunk.sort_by(|a, b| comparator(a, b));
        result.extend(chunk);
    }

    // Cycles are added last to ensure inclusion.
    let result_set: HashSet<_> = result.iter().cloned().collect();
    for path in paths {
        if !result_set.contains(path) {
            result.push(path.clone());
        }
    }

    result
}

/// Builds a full dependency graph for the given set of files.
pub fn build_dependency_graph(
    paths: &[PathBuf],
    repo_root: &Path,
) -> crate::utils::graph::DependencyGraph {
    let mut graph = crate::utils::graph::DependencyGraph::new();

    for path in paths {
        graph.add_node(path.clone());
        if let Ok(content) = std::fs::read_to_string(path) {
            let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            let imports = extract_imports(&content, extension);

            for import in imports {
                if let Some(dep_path) = resolve_import(&import, path, repo_root) {
                    // Add edges for internal dependencies to provide architectural context.
                    // resolve_import verifies existence of target on disk.
                    graph.add_edge(path.clone(), dep_path);
                }
            }
        }
    }
    graph
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_rust_imports() {
        let code = r#"
            use crate::utils::foo;
            use std::collections::HashMap;
            mod bar;
        "#;
        let imports = extract_imports(code, "rs");
        assert!(imports.contains(&"crate::utils::foo".to_string()));
        assert!(imports.contains(&"bar".to_string()));
    }

    #[test]
    fn test_extract_python_imports() {
        let code = r#"
            import os
            from utils import helper
            import numpy as np
        "#;
        let imports = extract_imports(code, "py");
        assert!(imports.contains(&"os".to_string()));
        // "utils" is captured from `from utils ...`
        // We don't capture "helper" because without FS it's ambiguous if it's a file or symbol.
        // For file dependency graph, depending on "utils" (utils.py or utils/__init__.py) is sufficient
        // to establish the edge.
        assert!(imports.contains(&"utils".to_string()));
        assert!(imports.contains(&"numpy".to_string()));
    }

    #[test]
    fn test_topo_sort_with_files() -> anyhow::Result<()> {
        let temp = std::env::temp_dir().join("abyss_dep_test");
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp)?;

        // c.rs (Independent)
        std::fs::write(temp.join("c.rs"), "fn c() {}")?;

        // b.rs (Imports c)
        std::fs::write(temp.join("b.rs"), "mod c;")?;

        // a.rs (Imports b)
        std::fs::write(temp.join("a.rs"), "mod b;")?;

        // d.rs (Independent)
        std::fs::write(temp.join("d.rs"), "fn d() {}")?;

        let paths = vec![
            temp.join("a.rs"),
            temp.join("b.rs"),
            temp.join("c.rs"),
            temp.join("d.rs"),
        ];

        // Sort descending by score.
        // fake scores: d=100, c=50, b=10, a=1
        // topo order for connected: c -> b -> a
        // d is independent.
        // Start: Leaves = {c, d}.
        // Sort leaves by score: d (100) > c (50).
        // Chunk 1: [d, c]
        // Remove c -> b becomes leaf.
        // Chunk 2: [b]
        // Remove b -> a becomes leaf.
        // Chunk 3: [a]
        // Result: d, c, b, a.

        let sorted = sort_paths_topologically(&paths, &temp, |x, y| {
            let score = |p: &PathBuf| {
                if p.ends_with("d.rs") {
                    100
                } else if p.ends_with("c.rs") {
                    50
                } else if p.ends_with("b.rs") {
                    10
                } else {
                    1
                }
            };
            score(y).cmp(&score(x)) // Descending
        });

        let names: Vec<_> = sorted
            .iter()
            .map(|p| p.file_name().unwrap().to_str().unwrap())
            .collect();

        assert_eq!(names, vec!["d.rs", "c.rs", "b.rs", "a.rs"]);

        let _ = std::fs::remove_dir_all(&temp);
        Ok(())
    }
}
