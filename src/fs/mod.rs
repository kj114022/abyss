use anyhow::Result;
use ignore::WalkBuilder;
use std::path::{Path, PathBuf};

/// Configuration for directory walking
pub struct WalkConfig<'a> {
    pub ignore_patterns: &'a [String],
    pub include_patterns: &'a [String],
    pub max_depth: Option<usize>,
    pub max_file_size: Option<usize>,
}

impl<'a> Default for WalkConfig<'a> {
    fn default() -> Self {
        Self {
            ignore_patterns: &[],
            include_patterns: &[],
            max_depth: None,
            max_file_size: None,
        }
    }
}

pub fn walk_directory(path: &Path, ignore_patterns: &[String]) -> Result<Vec<PathBuf>> {
    walk_directory_with_config(
        path,
        WalkConfig {
            ignore_patterns,
            ..Default::default()
        },
    )
}

pub fn walk_directory_with_config(path: &Path, config: WalkConfig) -> Result<Vec<PathBuf>> {
    let mut builder = WalkBuilder::new(path);

    // Add custom ignore patterns
    // In ignore crate's OverrideBuilder:
    // - Pattern "foo" means WHITELIST "foo" (only match foo)
    // - Pattern "!foo" means IGNORE "foo" (exclude foo)
    // So to ignore files matching a pattern, we prefix with "!"
    let mut override_builder = ignore::overrides::OverrideBuilder::new(path);
    for pattern in config.ignore_patterns {
        override_builder.add(&format!("!{}", pattern))?;
    }
    let overrides = override_builder.build()?;
    builder.overrides(overrides);

    // Standard gitignore is on by default.
    builder.standard_filters(true);
    // Allow hidden files (like .gitignore, .github)
    builder.hidden(false);

    // Max depth
    if let Some(depth) = config.max_depth {
        builder.max_depth(Some(depth));
    }

    let walker = builder.build();
    let mut files = Vec::new();

    // Precompile include patterns for matching
    let include_matchers: Vec<glob::Pattern> = config
        .include_patterns
        .iter()
        .filter_map(|p| glob::Pattern::new(p).ok())
        .collect();

    for result in walker {
        match result {
            Ok(entry) => {
                if entry.file_type().is_some_and(|ft| ft.is_file()) {
                    let file_path = entry.path();

                    // Max file size check
                    if let Some(max_size) = config.max_file_size {
                        if let Ok(metadata) = file_path.metadata() {
                            if metadata.len() as usize > max_size {
                                continue; // Skip oversized files
                            }
                        }
                    }

                    // Include pattern check
                    if !include_matchers.is_empty() {
                        let matches = include_matchers.iter().any(|m| {
                            m.matches_path(file_path)
                                || file_path
                                    .file_name()
                                    .and_then(|n| n.to_str())
                                    .is_some_and(|name| m.matches(name))
                        });
                        if !matches {
                            continue; // Skip files not matching include patterns
                        }
                    }

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
