use crate::utils::graph::DependencyGraph;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

/// Generates a hierarchical Mermaid diagram from the dependency graph.
pub fn generate_diagram(graph: &DependencyGraph, root: &Path) -> String {
    let nodes = graph.get_nodes();

    // Safeguard: Limit node count
    if nodes.len() > 200 {
        return format!(
            "<!-- Graph too large to display ({} nodes). Limit is 200. -->",
            nodes.len()
        );
    }

    if nodes.is_empty() {
        return String::new();
    }

    let mut lines = Vec::new();
    lines.push("graph TD;".to_string());

    // 1. Assign IDs
    let mut path_to_id = HashMap::new();
    let mut sorted_nodes: Vec<_> = nodes.iter().collect();
    sorted_nodes.sort();

    for (i, path) in sorted_nodes.iter().enumerate() {
        path_to_id.insert((*path).clone(), format!("N{}", i));
    }

    // 2. Build Hierarchy (Directory Tree)
    // Map: Dir -> List of Files (that are nodes)
    // And Dir -> List of Subdirs
    // Actually, we can just iterate sorted nodes and open/close subgraphs?
    // Or simpler: Recursively build the structure.

    // Let's organize paths by their directory structure relative to root.
    let mut hierarchy: HashMap<PathBuf, Vec<PathBuf>> = HashMap::new(); // Dir -> Files
    let mut dirs: HashSet<PathBuf> = HashSet::new();

    for path in &sorted_nodes {
        if let Ok(relative) = path.strip_prefix(root) {
            if let Some(parent) = relative.parent() {
                let parent_path = if parent == Path::new("") {
                    PathBuf::from(".")
                } else {
                    parent.to_path_buf()
                };
                hierarchy
                    .entry(parent_path.clone())
                    .or_default()
                    .push((*path).clone());
                dirs.insert(parent_path);
            } else {
                // Root level file
                hierarchy
                    .entry(PathBuf::from("."))
                    .or_default()
                    .push((*path).clone());
                dirs.insert(PathBuf::from("."));
            }
        }
    }

    // We need to nest subgraphs.
    // e.g. subgraph src { subgraph utils { ... } }
    // We need to walk the directory tree.
    // Let's get all unique dirs and sort them.
    let mut sorted_dirs: Vec<_> = dirs.into_iter().collect();
    sorted_dirs.sort();

    // Mermaid subgraphs are flat-ish if we define them by ID.
    // But nesting requires `subgraph A [Block] \n subgraph B [Inner] ... end \n end`
    // This is hard to do with just a flat list of dirs.
    // Alternative: Just use full path as subgraph id? `subgraph src_utils ["src/utils"]`
    // Mermaid doesn't strictly mandate physical nesting in text if IDs are used?
    // Actually, physically nesting blocks in text is how you define hierarchy in Mermaid.

    // Recursive approach:
    // Function `write_dir(current_dir)`
    //   `subgraph current_dir_id [current_dir_name]`
    //   List files in this dir
    //   List sub-directories (recurse)
    //   `end`

    // To do this, we need a tree structure of dirs.
    // Let's build a proper tree.
    #[derive(Default)]
    struct DirNode {
        files: Vec<PathBuf>,
        subdirs: HashMap<String, DirNode>,
    }

    let mut tree = DirNode::default();

    for path in &sorted_nodes {
        if let Ok(relative) = path.strip_prefix(root) {
            let components: Vec<_> = relative
                .components()
                .map(|c| c.as_os_str().to_string_lossy().to_string())
                .collect();
            // Last component is filename
            if let Some((_filename, parents)) = components.split_last() {
                let mut current = &mut tree;
                for dir in parents {
                    current = current.subdirs.entry(dir.clone()).or_default();
                }
                current.files.push((*path).clone());
            }
        }
    }

    // Recursive writer
    fn write_tree(
        node: &DirNode,
        current_path: &Path,
        lines: &mut Vec<String>,
        path_to_id: &HashMap<PathBuf, String>,
        indent: usize,
    ) {
        let spaces = " ".repeat(indent);

        // Write files in this dir
        for file_path in &node.files {
            if let Some(id) = path_to_id.get(file_path) {
                let name = file_path.file_name().unwrap_or_default().to_string_lossy();
                // Style by extension
                let ext = file_path.extension().and_then(|s| s.to_str()).unwrap_or("");
                let style_class = match ext {
                    "rs" => "rust",
                    "py" => "python",
                    "js" | "ts" | "jsx" | "tsx" => "js",
                    "html" | "css" | "scss" => "web",
                    _ => "other",
                };

                // We will define classes at the end.
                // For now just node definition.
                // Escape name
                let clean_name = name.replace("\"", "'");
                lines.push(format!(
                    "{}{}[\"{}\"]:::{}",
                    spaces, id, clean_name, style_class
                ));
            }
        }

        // Write subdirs
        let mut sorted_subdirs: Vec<_> = node.subdirs.keys().collect();
        sorted_subdirs.sort();

        for dirname in sorted_subdirs {
            if let Some(subdir_node) = node.subdirs.get(dirname) {
                // Subgraph ID must be alphanumeric
                // We can use a hash or just path with underscores.
                // Let's generate a unique ID for the subgraph.
                let full_sub_path = current_path.join(dirname);
                // safe id
                let sub_id = format!(
                    "cluster_{}",
                    full_sub_path
                        .to_string_lossy()
                        .replace(['/', '.', '-', ' '], "_")
                );

                lines.push(format!("{}subgraph {} [\"{}\"]", spaces, sub_id, dirname));
                write_tree(subdir_node, &full_sub_path, lines, path_to_id, indent + 2);
                lines.push(format!("{}end", spaces));
            }
        }
    }

    write_tree(&tree, Path::new(""), &mut lines, &path_to_id, 4);

    // 3. Edges
    // We need to fetch edges from graph.
    // Graph doesn't expose edges publicly directly as a map?
    // We might need to add a getter to DependencyGraph or expose edges.
    // Assuming `get_edges()` exists or `edges` field is public (it's not).
    // The previous implementation utilized internal access or iterator?
    // `to_mermaid` was INSIDE `DependencyGraph`.
    // Now we are OUTSIDE.
    // We need to update `DependencyGraph` to expose edges or methods.

    // Let's assume we add `pub fn get_edges(&self) -> &HashMap<PathBuf, HashSet<PathBuf>>`

    let edges = graph.get_edges();
    let mut sorted_sources: Vec<_> = edges.keys().collect();
    sorted_sources.sort();

    for from in sorted_sources {
        if let Some(targets) = edges.get(from) {
            let mut sorted_targets: Vec<_> = targets.iter().collect();
            sorted_targets.sort();

            if let Some(from_id) = path_to_id.get(from) {
                for to in sorted_targets {
                    if let Some(to_id) = path_to_id.get(to) {
                        lines.push(format!("    {} --> {};", from_id, to_id));
                    }
                }
            }
        }
    }

    // 4. Styles
    lines.push("    classDef rust fill:#dea,stroke:#555,stroke-width:1px;".to_string());
    lines.push("    classDef python fill:#ade,stroke:#555,stroke-width:1px;".to_string());
    lines.push("    classDef js fill:#ee9,stroke:#555,stroke-width:1px;".to_string());
    lines.push("    classDef web fill:#fcd,stroke:#555,stroke-width:1px;".to_string());
    lines.push("    classDef other fill:#eee,stroke:#555,stroke-width:1px;".to_string());

    lines.join("\n")
}
