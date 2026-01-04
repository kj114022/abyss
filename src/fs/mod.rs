use anyhow::Result;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

pub fn walk_directory(path: &Path, ignore_patterns: &[String]) -> Result<Vec<PathBuf>> {
    let mut builder = WalkBuilder::new(path);

    // Add custom ignore patterns
    let mut override_builder = ignore::overrides::OverrideBuilder::new(path);
    for pattern in ignore_patterns {
        // In override builder:
        // - "pattern" means IGNORE "pattern"
        // - "!pattern" means WHITELIST "pattern"
        // Repomix/User input usually expects "glob to ignore".
        // So we pass it directly.
        // We might need to ensure valid glob.
        override_builder.add(&format!("!{}", pattern))?;
    }
    let overrides = override_builder.build()?;

    builder.overrides(overrides);

    // Standard gitignore is on by default.
    builder.standard_filters(true);

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
}
