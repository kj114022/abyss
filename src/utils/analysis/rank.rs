use std::collections::HashMap;
use std::path::{Path, PathBuf};

/// Calculates the Shannon entropy of the content.
/// Higher entropy implies more complex/random information (dense code).
/// Low entropy implies repetition (boilerplate).
pub fn calculate_entropy(content: &str) -> f64 {
    if content.is_empty() {
        return 0.0;
    }

    // Char-based entropy: fast and sufficient for code density analysis
    let mut counts = [0usize; 256];
    let mut total = 0;

    for byte in content.bytes() {
        counts[byte as usize] += 1;
        total += 1;
    }

    let mut entropy = 0.0;
    let total_f = total as f64;

    for &count in &counts {
        if count > 0 {
            let p = count as f64 / total_f;
            entropy -= p * p.log2();
        }
    }

    entropy
}

/// Structure to hold scoring metadata for a file
#[derive(Debug, Default, Clone)]
pub struct FileScore {
    pub pagerank: f64,
    pub entropy: f64,
    pub churn: i32,
    pub heuristic: i32,
    pub tokens: usize,
}

impl FileScore {
    pub fn final_score(&self) -> f64 {
        // Normalize and weight
        // Heuristic: 0-1000
        // Churn: 0-200
        // PageRank: 0.0-1.0 (approx, depends on N)
        // Entropy: 4.0-6.0 range usually for code

        // Combine all factors into final score

        let rank_bonus = self.pagerank * 1000.0; // Assume 0.01 PR -> 10 points
        let entropy_bonus = self.entropy * 10.0;

        self.heuristic as f64 + (self.churn as f64) + rank_bonus + entropy_bonus
    }
}

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
/// Sorting using unified FileScore
pub fn sort_paths(paths: &mut [PathBuf], scores: &HashMap<PathBuf, FileScore>) {
    paths.sort_by(|a, b| {
        let score_a = scores.get(a).map(|s| s.final_score()).unwrap_or(0.0);
        let score_b = scores.get(b).map(|s| s.final_score()).unwrap_or(0.0);

        // Descending order sort
        score_b
            .partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.cmp(b))
    });
}

/// Calculates the base heuristic score based on filename rules.
pub fn heuristic_score(path: &Path) -> i32 {
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

    score
}

/// Sorts files using topological order from the dependency graph,
/// with score-based tie-breaking for files at the same dependency level.
pub fn sort_files(
    _paths: &[PathBuf],
    scores: &HashMap<PathBuf, FileScore>,
    graph: &crate::utils::graph::DependencyGraph,
) -> Vec<PathBuf> {
    graph.sort_topologically(|a, b| {
        let score_a = scores.get(a).map(|s| s.final_score()).unwrap_or(0.0);
        let score_b = scores.get(b).map(|s| s.final_score()).unwrap_or(0.0);
        // Descending score (higher score first)
        score_b
            .partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.cmp(b))
    })
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

        let mut scores = std::collections::HashMap::new();
        for path in &paths {
            scores.insert(
                path.clone(),
                FileScore {
                    heuristic: heuristic_score(path),
                    ..Default::default()
                },
            );
        }

        sort_paths(&mut paths, &scores);

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
        let mut paths = vec![
            PathBuf::from("regular_core.rs"), // Score 600
            PathBuf::from("churned_util.rs"), // Score 400 + Boost
        ];

        let mut scores = std::collections::HashMap::new();
        scores.insert(
            PathBuf::from("churned_util.rs"),
            FileScore {
                churn: 200, // 50 * 5 = 250, capped at 200.
                heuristic: 400,
                ..Default::default()
            },
        );
        scores.insert(
            PathBuf::from("regular_core.rs"),
            FileScore {
                heuristic: 600,
                ..Default::default()
            },
        );

        sort_paths(&mut paths, &scores);

        // churned_util (650) > regular_core (600)
        assert_eq!(paths[0].to_str().unwrap(), "churned_util.rs");
    }
}
