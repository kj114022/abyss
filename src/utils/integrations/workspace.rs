use anyhow::{Context, Result};
use std::path::Path;

use crate::config::WorkspaceConfig;

/// Load workspace configuration from a YAML file
pub fn load_workspace_config(path: &Path) -> Result<WorkspaceConfig> {
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("Failed to read workspace config: {:?}", path))?;

    let config: WorkspaceConfig =
        serde_yaml::from_str(&content).with_context(|| "Failed to parse workspace config YAML")?;

    // Resolve relative paths
    let root = path.parent().unwrap_or_else(|| Path::new("."));
    let repositories = config
        .repositories
        .into_iter()
        .map(|mut repo| {
            if repo.path.is_relative() {
                repo.path = root.join(repo.path);
            }
            repo.path = repo.path.canonicalize().unwrap_or(repo.path);
            repo
        })
        .collect();

    Ok(WorkspaceConfig {
        repositories,
        output: config
            .output
            .map(|p| if p.is_relative() { root.join(p) } else { p }),
    })
}

/// Check if a path looks like a workspace config
pub fn is_workspace_file(path: &Path) -> bool {
    // Check extension
    if let Some(ext) = path.extension()
        && (ext == "yaml" || ext == "yml")
    {
        // Check content structure quickly
        if let Ok(content) = std::fs::read_to_string(path) {
            return content.contains("repositories:");
        }
    }
    false
}
