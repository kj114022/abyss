use anyhow::Result;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

pub fn walk_directory(path: &Path, ignore_patterns: &[String]) -> Result<Vec<PathBuf>> {
    let mut builder = WalkBuilder::new(path);

    // Add custom ignore patterns
    let mut override_builder = ignore::overrides::OverrideBuilder::new(path);
    for pattern in ignore_patterns {
        // In override builder, passing the pattern string ignores it.
        // We do NOT want to prefix with '!' unless we are whitelisting.
        override_builder.add(pattern)?;
    }
    let overrides = override_builder.build()?;

    builder.overrides(overrides);

    // Standard gitignore is on by default.
    builder.standard_filters(true);
    // Allow hidden files (like .gitignore, .github)
    builder.hidden(false);

    let walker = builder.build();
    let mut files = Vec::new();

    for result in walker {
        match result {
            Ok(entry) => {
                if entry.file_type().is_some_and(|ft| ft.is_file()) {
                    files.push(entry.into_path());
                }
            }
            Err(err) => eprintln!("Error walking directory: {}", err),
        }
    }

    files.sort();
    Ok(files)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs::File;
    use tempfile::TempDir;

    #[test]
    fn test_walk_directory_ignore_logic() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let root = temp_dir.path();

        // Create files
        File::create(root.join("include.rs"))?;
        File::create(root.join("exclude.env"))?;

        // 1. Walk with ignore "*.env"
        // In ignore crate:
        // "glob" -> ignore
        // "!glob" -> whitelist
        // So passing "*.env" should ignore .env files.
        let paths = walk_directory(root, &["*.env".to_string()])?;

        let path_strs: Vec<String> = paths
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();
        println!("Paths found: {:?}", path_strs);

        assert!(path_strs.contains(&"include.rs".to_string()));
        assert!(!path_strs.contains(&"exclude.env".to_string()));

        Ok(())
    }

    #[test]
    fn test_finds_gitignore() -> Result<()> {
        let temp_dir = TempDir::new()?;
        let root = temp_dir.path();

        File::create(root.join(".gitignore"))?;
        File::create(root.join("normal.rs"))?;

        // Should find .gitignore because we set hidden(false)
        let paths = walk_directory(root, &[])?;
        let path_strs: Vec<String> = paths
            .iter()
            .map(|p| p.file_name().unwrap().to_string_lossy().to_string())
            .collect();

        assert!(path_strs.contains(&"normal.rs".to_string()));
        assert!(path_strs.contains(&".gitignore".to_string()));

        Ok(())
    }
}
