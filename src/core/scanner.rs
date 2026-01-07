//! File scanner for discovering and filtering files
//!
//! Handles file discovery, workspace support, and diff filtering.

use crate::config::AbyssConfig;
use crate::core::ScanEvent;
use crate::utils::git_stats::get_diff_files;
use anyhow::{Context, Result};
use crossbeam_channel::Sender;
use std::collections::HashSet;
use std::path::{Path, PathBuf};

/// Notify helper for optional sender
fn notify(tx: &Option<Sender<ScanEvent>>, event: ScanEvent) {
    if let Some(tx) = tx {
        let _ = tx.send(event);
    }
}

/// Discover files according to configuration
///
/// Returns a tuple of (files_with_roots, dropped_files) where each file is paired
/// with its repository root for multi-repo support.
pub fn discover_files(
    config: &AbyssConfig,
    tx: Option<Sender<ScanEvent>>,
) -> Result<(Vec<(PathBuf, PathBuf)>, Vec<PathBuf>)> {
    notify(&tx, ScanEvent::StartScanning);

    let root_path = config
        .path
        .canonicalize()
        .with_context(|| format!("Failed to find directory: {:?}", config.path))?;

    // Use if-expression to avoid late initialization
    let collected_files = if crate::utils::workspace::is_workspace_file(&root_path) {
        scan_workspace(config, &root_path)?
    } else {
        scan_single_directory(config, &root_path)?
    };

    notify(&tx, ScanEvent::FilesFound(collected_files.len()));

    let dropped_files: Vec<PathBuf> = Vec::new();
    Ok((collected_files, dropped_files))
}

/// Scan a workspace configuration file
fn scan_workspace(config: &AbyssConfig, root_path: &Path) -> Result<Vec<(PathBuf, PathBuf)>> {
    let ws_config = crate::utils::workspace::load_workspace_config(root_path)?;
    let mut collected_files = Vec::new();

    for repo in ws_config.repositories {
        if !repo.path.exists() {
            eprintln!("Warning: Repository path not found: {:?}", repo.path);
            continue;
        }

        let paths = scan_repository(config, &repo.path)?;
        let filtered = filter_by_diff(config, &repo.path, paths);

        for path in filtered {
            collected_files.push((path, repo.path.clone()));
        }
    }

    Ok(collected_files)
}

/// Scan a single directory
fn scan_single_directory(
    config: &AbyssConfig,
    root_path: &Path,
) -> Result<Vec<(PathBuf, PathBuf)>> {
    let paths = scan_repository(config, root_path)?;
    let filtered = filter_by_diff(config, root_path, paths);
    let root_owned = root_path.to_path_buf();

    Ok(filtered
        .into_iter()
        .map(|p| (p, root_owned.clone()))
        .collect())
}

/// Scan a repository with ignore patterns
fn scan_repository(config: &AbyssConfig, repo_path: &Path) -> Result<Vec<PathBuf>> {
    // Load .abyssignore for this repo
    let abyssignore_patterns = crate::utils::abyssignore::load_abyssignore(repo_path);
    let mut all_ignore_patterns = config.ignore_patterns.clone();
    all_ignore_patterns.extend(abyssignore_patterns);

    let walk_config = crate::fs::WalkConfig {
        ignore_patterns: &all_ignore_patterns,
        include_patterns: &config.include_patterns,
        max_depth: config.max_depth,
        max_file_size: config.max_file_size,
    };

    crate::fs::walk_directory_with_config(repo_path, walk_config)
}

/// Filter paths by diff target if specified
fn filter_by_diff(config: &AbyssConfig, repo_path: &Path, mut paths: Vec<PathBuf>) -> Vec<PathBuf> {
    if let Some(ref target) = config.diff
        && let Some(diff_files) = get_diff_files(repo_path, target)
    {
        let diff_set: HashSet<PathBuf> = diff_files.into_iter().map(PathBuf::from).collect();
        paths.retain(|p| {
            if let Ok(relative) = p.strip_prefix(repo_path) {
                diff_set.contains(relative)
            } else {
                false
            }
        });
    }
    paths
}

#[cfg(test)]
mod tests {
    // Scanner tests would go here
}
