use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct FileNode {
    pub path: PathBuf,
    pub name: String,
    pub is_dir: bool,
    pub is_expanded: bool,
    pub is_selected: bool,
    pub children: Vec<FileNode>,
    pub depth: usize,
}

impl FileNode {
    pub fn new(path: PathBuf, is_dir: bool, depth: usize) -> Self {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string();

        Self {
            path,
            name,
            is_dir,
            is_expanded: true, // Auto-expand by default for now
            is_selected: true,
            children: Vec::new(),
            depth,
        }
    }

    pub fn toggle_expanded(&mut self) {
        if self.is_dir {
            self.is_expanded = !self.is_expanded;
        }
    }

    pub fn toggle_selected(&mut self) {
        self.is_selected = !self.is_selected;
        // Propagate down
        for child in &mut self.children {
            child.set_selected(self.is_selected);
        }
    }

    fn set_selected(&mut self, selected: bool) {
        self.is_selected = selected;
        for child in &mut self.children {
            child.set_selected(selected);
        }
    }

    // Flatten visible nodes for rendering list
    pub fn flatten(&self) -> Vec<&FileNode> {
        let mut result = Vec::new();
        result.push(self);

        if self.is_expanded {
            for child in &self.children {
                result.extend(child.flatten());
            }
        }

        result
    }

    // Mutable access needed for toggling via index in flat list?
    // This is tricky. A flat list index doesn't easily map back to the tree
    // without rebuilding the flat list or keeping references.
    // For TUI, we usually keep the tree key-stable.

    pub fn toggle_expand_at_index(&mut self, target_index: usize) {
        let mut current = 0;
        self.toggle_expand_recursive(&mut current, target_index);
    }

    fn toggle_expand_recursive(&mut self, current: &mut usize, target: usize) -> bool {
        if *current == target {
            self.toggle_expanded();
            return true;
        }
        *current += 1;

        if self.is_expanded {
            for child in &mut self.children {
                if child.toggle_expand_recursive(current, target) {
                    return true;
                }
            }
        }
        false
    }

    pub fn toggle_select_at_index(&mut self, target_index: usize) {
        let mut current = 0;
        self.toggle_select_recursive(&mut current, target_index);
    }

    fn toggle_select_recursive(&mut self, current: &mut usize, target: usize) -> bool {
        if *current == target {
            self.toggle_selected();
            return true;
        }
        *current += 1;

        if self.is_expanded {
            for child in &mut self.children {
                if child.toggle_select_recursive(current, target) {
                    return true;
                }
            }
        }
        false
    }

    pub fn collect_selected_paths(&self) -> Vec<PathBuf> {
        let mut paths = Vec::new();
        if self.is_selected && !self.is_dir {
            paths.push(self.path.clone());
        }

        for child in &self.children {
            paths.extend(child.collect_selected_paths());
        }
        paths
    }

    pub fn visible_count(&self) -> usize {
        let mut count = 1; // self
        if self.is_expanded {
            for child in &self.children {
                count += child.visible_count();
            }
        }
        count
    }
}

pub fn build_tree(root_path: &Path, paths: Vec<PathBuf>) -> FileNode {
    let mut root = FileNode::new(root_path.to_path_buf(), true, 0);

    // We group paths by their components relative to root
    // But simplified: naive insertion

    // Sort paths to ensure parents sort before children?
    // Actually we just insert one by one.

    // We can't easily build internal node directly with struct.
    // Let's use an intermediate "DirEntry" map or similar.

    // Better approach:
    // 1. Create all unique parent directories
    // 2. Nest them.

    // Let's stick to a recursive builder.
    // Group paths by first component.

    // ... Actually, for a CLI tool, the number of files usually isn't massive (10k max).
    // We can just iterate and insert.

    for path in paths {
        insert_into_tree(&mut root, root_path, &path);
    }

    // Sort children: Dirs first, then files
    sort_tree(&mut root);

    root
}

fn insert_into_tree(node: &mut FileNode, base: &Path, full_path: &Path) {
    // Current node represents 'base'
    // functionality: find the immediate child component of 'full_path' relative to 'base'

    let relative = match full_path.strip_prefix(base) {
        Ok(p) => p,
        Err(_) => return, // Should not happen if filtered correctly
    };

    if relative.as_os_str().is_empty() {
        return; // It IS the node
    }

    let component = relative.components().next();
    if let Some(std::path::Component::Normal(c)) = component {
        let child_name = c.to_string_lossy();
        let child_path = base.join(c);

        // Check if child exists
        let child_idx = node
            .children
            .iter()
            .position(|child| child.name == child_name);

        match child_idx {
            Some(idx) => {
                // Continue recursion
                insert_into_tree(&mut node.children[idx], &child_path, full_path);
            }
            None => {
                // Determine if this child is a leaf (file) or intermediate dir
                // If 'relative' has more than 1 component, it MUST be a dir (intermediate).
                // If 'relative' has 1 component, it could be a file OR a dir (if input list has dirs?)
                // Our discover_files returns FILES. So intermediate are implicit dirs.
                // Wait, if it's the target file itself, it's a file.

                let is_leaf = relative.components().count() == 1;
                // If it is a leaf, it is the file itself -> is_dir = false
                // If it is not a leaf, it is an intermediate directory -> is_dir = true

                let mut new_child = FileNode::new(child_path.clone(), !is_leaf, node.depth + 1);

                // If it's not a leaf, we need to recurse to add the *rest* of the path
                if !is_leaf {
                    insert_into_tree(&mut new_child, &child_path, full_path);
                }

                node.children.push(new_child);
            }
        }
    }
}

fn sort_tree(node: &mut FileNode) {
    node.children.sort_by(|a, b| {
        // Dirs first
        if a.is_dir != b.is_dir {
            return b.is_dir.cmp(&a.is_dir); // true > false
        }
        a.name.cmp(&b.name)
    });

    for child in &mut node.children {
        sort_tree(child);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tree_construction() {
        let root = PathBuf::from("root");
        let paths = vec![
            PathBuf::from("root/file1.rs"),
            PathBuf::from("root/src/main.rs"),
            PathBuf::from("root/src/lib.rs"),
        ];

        let mut tree = build_tree(&root, paths);

        assert_eq!(tree.children.len(), 2); // file1.rs, src/

        // src should be a dir
        let src_idx = tree.children.iter().position(|c| c.name == "src").unwrap();
        assert!(tree.children[src_idx].is_dir);
        assert_eq!(tree.children[src_idx].children.len(), 2); // main.rs, lib.rs

        // flatten check
        assert!(tree.is_expanded);
        let flat = tree.flatten();
        // root, file1, src, main, lib (sorting order: src first?)
        // Sort logic: Dirs first.
        // So children: src/, file1.rs
        // Flatten: root -> src -> main -> lib -> file1
        // Total 5 nodes.
        assert_eq!(flat.len(), 5);
    }

    #[test]
    fn test_toggle_expand() {
        let root = PathBuf::from("root");
        let paths = vec![PathBuf::from("root/dir/file.rs")];
        let mut tree = build_tree(&root, paths);

        // root (0) -> dir (1) -> file (2)
        // All expanded by default.

        // Collapse 'dir'
        // Index 1 should be 'dir' (root is 0, children sorted.. only 1 child)
        tree.toggle_expand_at_index(1);

        let flat = tree.flatten();
        // root -> dir (collapsed) -> file is hidden
        assert_eq!(flat.len(), 2); // root, dir
    }
}
