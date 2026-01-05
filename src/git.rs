use anyhow::{Context, Result};
use std::process::Command;
use tempfile::TempDir;

pub fn is_remote_url(path: &str) -> bool {
    path.starts_with("http") || path.starts_with("git@") || path.starts_with("ssh://")
}

pub fn clone_repo(url: &str) -> Result<TempDir> {
    let temp_dir = TempDir::new().context("Failed to create temporary directory")?;

    // Perform git clone
    let status = Command::new("git")
        .args(["clone", "--depth", "1", url, "."])
        .current_dir(temp_dir.path())
        .status()
        .context("Failed to execute git clone")?;

    if !status.success() {
        return Err(anyhow::anyhow!("Git clone failed for URL: {}", url));
    }

    Ok(temp_dir)
}
