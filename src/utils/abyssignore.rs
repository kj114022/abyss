//! Abyssignore file support - project-specific ignore patterns

use std::fs;
use std::path::Path;

/// Load ignore patterns from .abyssignore file in project root
/// Returns empty vec if file doesn't exist
pub fn load_abyssignore(root: &Path) -> Vec<String> {
    let ignore_file = root.join(".abyssignore");
    if !ignore_file.exists() {
        return Vec::new();
    }

    fs::read_to_string(ignore_file)
        .unwrap_or_default()
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            !trimmed.is_empty() && !trimmed.starts_with('#')
        })
        .map(|line| line.trim().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_load_abyssignore() {
        let temp = TempDir::new().unwrap();
        let ignore_path = temp.path().join(".abyssignore");

        {
            let mut file = fs::File::create(&ignore_path).unwrap();
            writeln!(file, "# Comment line").unwrap();
            writeln!(file, "*.test.rs").unwrap();
            writeln!(file, "").unwrap();
            writeln!(file, "mock_*").unwrap();
            writeln!(file, "  # Indented comment  ").unwrap();
            writeln!(file, "  spaced_pattern  ").unwrap();
        }

        let patterns = load_abyssignore(temp.path());
        assert_eq!(patterns.len(), 3);
        assert!(patterns.contains(&"*.test.rs".to_string()));
        assert!(patterns.contains(&"mock_*".to_string()));
        assert!(patterns.contains(&"spaced_pattern".to_string()));
    }

    #[test]
    fn test_missing_abyssignore() {
        let temp = TempDir::new().unwrap();
        let patterns = load_abyssignore(temp.path());
        assert!(patterns.is_empty());
    }
}
