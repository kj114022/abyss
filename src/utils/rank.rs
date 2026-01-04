use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Semantic Ranking Logic for File Ordering
///
/// Heuristics:
/// 1. Documentation (1000): README.md
/// 2. Configs (800): Cargo.toml, package.json
/// 3. Entry Points (700): main.rs, lib.rs
/// 4. Core Logic (600): core/, app/, model/
/// 5. Utilities (400): util/, common/
/// 6. Tests (100): tests/, *.spec.ts
/// 7. Default (500)
///
/// Higher score = Earlier in output.
pub fn sort_paths(
    paths: &mut [PathBuf],
    git_stats: &HashMap<PathBuf, crate::utils::git_stats::GitStats>,
) {
    paths.sort_by(|a, b| {
        let score_a = score_path(a, git_stats.get(a));
        let score_b = score_path(b, git_stats.get(b));
        // Descending order of score
        score_b.cmp(&score_a).then_with(|| a.cmp(b)) // Tie-break alphabetically
    });
}

pub fn score_path(path: &Path, stats: Option<&crate::utils::git_stats::GitStats>) -> i32 {
    let filename = path
        .file_name()
        .and_then(|f: &std::ffi::OsStr| f.to_str())
        .unwrap_or_default()
        .to_lowercase();

    let path_str = path.to_string_lossy().to_lowercase();
    let components: Vec<_> = path
        .components()
        .map(|c: std::path::Component| c.as_os_str().to_string_lossy().to_lowercase())
        .collect();

    let mut score = 500; // Default baseline

    // 1. Documentation (Highest Priority) +1000
    if filename == "readme.md" || filename == "readme.txt" {
        score = 1000;
    } else if filename == "architecture.md" || filename == "contributing.md" {
        score = 900;
    }
    // 2. Project Configuration +800
    else if filename == "cargo.toml"
        || filename == "package.json"
        || filename == "go.mod"
        || filename == "makefile"
        || filename == "dockerfile"
    {
        score = 800;
    }
    // 3. Entry Points +700
    else if filename == "main.rs"
        || filename == "lib.rs"
        || filename == "index.js"
        || filename == "main.go"
    {
        score = 700;
    }
    // 4. Core Logic +600
    else if path_str.contains("core")
        || path_str.contains("app")
        || path_str.contains("model")
        || path_str.contains("schema")
    {
        score = 600;
    }
    // 6. Utilities +400
    else if path_str.contains("util")
        || path_str.contains("common")
        || path_str.contains("helper")
    {
        score = 400;
    }
    // 7. Tests/Benchmarks (Lowest Priority) +100
    else if path_str.contains("test")
        || path_str.contains("spec")
        || path_str.contains("bench")
        || filename.ends_with("_test.go")
        || filename.ends_with(".test.ts")
    {
        score = 100;
    }

    // 8. Depth Penalty
    // Subtract 10 points per depth level to prefer high-level files
    let depth = components.len() as i32;
    score -= depth * 10;

    // 9. Git Churn Boost
    // Files changed frequently (high churn) are likely important.
    // Boost score by churn_score (capped at 200).
    if let Some(s) = stats {
        let boost = std::cmp::min(s.churn_score * 5, 200) as i32;
        score += boost;
    }

    score
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sorting_smart() {
        let mut paths = vec![
            PathBuf::from("src/utils.rs"),
            PathBuf::from("tests/integration.rs"),
            PathBuf::from("Cargo.toml"),
            PathBuf::from("src/main.rs"),
            PathBuf::from("README.md"),
            PathBuf::from("unknown.txt"),
        ];

        let git_stats = std::collections::HashMap::new();

        sort_paths(&mut paths, &git_stats);

        let filenames: Vec<_> = paths
            .iter()
            .map(|p: &PathBuf| p.to_string_lossy().to_string())
            .collect();

        // 1. README.md (1000)
        // 2. Cargo.toml (800)
        // 3. src/main.rs (700)
        // 4. unknown.txt (500)
        // 5. src/utils.rs (400)
        // 6. tests/integration.rs (100)

        // Note: Depths might affect minor scores, but gaps are large enough (100+) that depth (10 per level) won't flip categories unless extremely deep.

        assert_eq!(filenames[0], "README.md");
        assert_eq!(filenames[1], "Cargo.toml");
        assert_eq!(filenames[2], "src/main.rs");
        assert_eq!(filenames[3], "unknown.txt");
        assert_eq!(filenames[4], "src/utils.rs");
        assert_eq!(filenames[5], "tests/integration.rs");
        assert_eq!(filenames[5], "tests/integration.rs");
    }

    #[test]
    fn test_churn_boost() {
        use crate::utils::git_stats::GitStats;

        let mut paths = vec![
            PathBuf::from("regular_core.rs"), // Score 600
            PathBuf::from("churned_util.rs"), // Score 400 + Boost
        ];

        let mut stats = std::collections::HashMap::new();
        stats.insert(
            PathBuf::from("churned_util.rs"),
            GitStats {
                churn_score: 50, // 50 * 5 = 250 boost. Final 650.
                ..Default::default()
            },
        );

        sort_paths(&mut paths, &stats);

        // churned_util (650) > regular_core (600)
        assert_eq!(paths[0].to_str().unwrap(), "churned_util.rs");
    }
}
