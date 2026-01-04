use git2::Repository;
use std::collections::HashMap;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default)]
pub struct GitStats {
    pub last_modified: u64,
    pub author: String,
    pub churn_score: u32, // Number of commits touching this file
}

/// Collects git statistics for files in the repository.
/// Returns an empty map if the directory is not a git repository.
pub fn get_git_stats(repo_root: &Path) -> HashMap<PathBuf, GitStats> {
    let mut stats_map = HashMap::new();

    let repo = match Repository::open(repo_root) {
        Ok(r) => r,
        Err(_) => return stats_map, // Not a git repo, return empty
    };

    // Walk commits to compute churn and last modified
    // This can be expensive for huge repos, so we might want to limit depth or use a cached approach in the future.
    // For now, we'll try a reasonably efficient walk.

    let mut revwalk = match repo.revwalk() {
        Ok(rw) => rw,
        Err(_) => return stats_map,
    };

    if revwalk.push_head().is_err() {
        return stats_map;
    }

    // Sort by time to get most recent first
    revwalk.set_sorting(git2::Sort::TIME).ok();

    // Limit to last 1000 commits to avoid hanging on massive repos?
    // Or just let it run? For "Deep Intelligence" we want good data.
    // Let's cap at 1000 for performance safety in this iteration.
    let commit_limit = 1000;

    for (i, oid) in revwalk.enumerate() {
        if i >= commit_limit {
            break;
        }

        // This unwrap is safe-ish because oid comes from revwalk.
        // But better use robust error handling.
        let oid = match oid {
            Ok(o) => o,
            Err(_) => continue,
        };

        let commit = match repo.find_commit(oid) {
            Ok(c) => c,
            Err(_) => continue,
        };

        // We need to compare with parent to see changed files.
        // If no parent (first commit), compare with empty tree?
        // Usually we just care about diffs.

        let tree = match commit.tree() {
            Ok(t) => t,
            Err(_) => continue,
        };

        let parent_tree = commit.parent(0).and_then(|p| p.tree()).ok();

        let diff = match repo.diff_tree_to_tree(parent_tree.as_ref(), Some(&tree), None) {
            Ok(d) => d,
            Err(_) => continue,
        };

        let commit_time = commit.time().seconds();
        let author = commit.author().name().unwrap_or("Unknown").to_string();

        let _ = diff.foreach(
            &mut |delta, _progress| {
                if let Some(path) = delta.new_file().path() {
                    let full_path = repo_root.join(path);
                    let entry = stats_map.entry(full_path).or_insert(GitStats {
                        last_modified: 0,
                        author: String::new(),
                        churn_score: 0,
                    });

                    // Since we walk backwards in time (implied by push_head default?),
                    // the first time we see a file is its latest modification.
                    if entry.last_modified == 0 {
                        entry.last_modified = commit_time as u64;
                        entry.author = author.clone();
                    }
                    entry.churn_score += 1;
                }
                true
            },
            None,
            None,
            None,
        );
    }

    stats_map
}

/// Returns a list of files changed between HEAD and the target reference (e.g. "main", "HEAD~1").
/// Returns paths relative to the repo root.
pub fn get_diff_files(repo_path: &Path, target_ref: &str) -> Option<Vec<String>> {
    let repo = Repository::open(repo_path).ok()?;

    // Resolve HEAD tree
    let head = repo.head().ok()?;
    let head_tree = head.peel_to_tree().ok()?;

    // Resolve Target tree
    let target_obj = repo.revparse_single(target_ref).ok()?;
    let target_tree = target_obj.peel_to_tree().ok()?;

    // Diff
    let diff = repo
        .diff_tree_to_tree(Some(&target_tree), Some(&head_tree), None)
        .ok()?;

    let mut files = Vec::new();

    // Iterate over deltas
    let _ = diff.foreach(
        &mut |delta, _hunks| {
            // Check for changed files (Added, Modified)
            // We ignore Deleted for now as we want content context? Or maybe we want to know it's gone.
            // Original logic just took the path.
            #[allow(clippy::collapsible_if)]
            if let Some(path_str) = delta.new_file().path().and_then(|p| p.to_str()) {
                files.push(path_str.to_string());
            }
            true
        },
        None,
        None,
        None,
    );

    Some(files)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_diff_files_integration() -> anyhow::Result<()> {
        let temp_dir = tempfile::TempDir::new()?;
        let repo_root = temp_dir.path();

        // Init repo
        let repo = Repository::init(repo_root)?;
        let signature = git2::Signature::now("Test User", "test@example.com")?;

        // Commit 1: Add file A
        let file_a = repo_root.join("file_a.txt");
        std::fs::write(&file_a, "content A")?;

        let mut index = repo.index()?;
        index.add_path(Path::new("file_a.txt"))?;
        let oid = index.write_tree()?;
        let tree = repo.find_tree(oid)?;
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Initial commit",
            &tree,
            &[],
        )?;

        // Commit 2: Modify A, Add B
        std::fs::write(&file_a, "content A modified")?;
        let file_b = repo_root.join("file_b.txt");
        std::fs::write(&file_b, "content B")?;

        index.add_path(Path::new("file_a.txt"))?;
        index.add_path(Path::new("file_b.txt"))?;
        let oid2 = index.write_tree()?;
        let tree2 = repo.find_tree(oid2)?;
        let parent = repo.head()?.peel_to_commit()?;
        repo.commit(
            Some("HEAD"),
            &signature,
            &signature,
            "Second commit",
            &tree2,
            &[&parent],
        )?;

        // Check diff between HEAD and HEAD~1
        // HEAD has modified A and new B compared to HEAD~1
        let diffs = get_diff_files(repo_root, "HEAD~1");
        assert!(diffs.is_some());
        let files = diffs.unwrap();

        assert!(files.contains(&"file_a.txt".to_string()));
        assert!(files.contains(&"file_b.txt".to_string()));
        assert_eq!(files.len(), 2);

        Ok(())
    }
}
