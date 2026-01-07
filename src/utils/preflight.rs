//! Pre-flight analysis - estimate scan cost before running

use rayon::prelude::*;
use std::fs;
use std::path::PathBuf;

use crate::config::{AbyssConfig, CompressionMode};

/// Pre-flight analysis results
#[derive(Debug)]
pub struct PreflightAnalysis {
    pub total_files: usize,
    pub total_size_bytes: u64,
    pub estimated_tokens: usize,
    pub estimated_time_secs: f64,
    pub recommendations: Vec<String>,
}

impl std::fmt::Display for PreflightAnalysis {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Pre-flight Analysis")?;
        writeln!(f, "===================")?;
        writeln!(f, "Total files:      {}", self.total_files)?;
        writeln!(
            f,
            "Total size:       {:.2} MB",
            self.total_size_bytes as f64 / 1_048_576.0
        )?;
        writeln!(
            f,
            "Estimated tokens: ~{}",
            format_number(self.estimated_tokens)
        )?;
        writeln!(f, "Estimated time:   {:.1}s", self.estimated_time_secs)?;

        if !self.recommendations.is_empty() {
            writeln!(f)?;
            writeln!(f, "Recommendations:")?;
            for rec in &self.recommendations {
                writeln!(f, "  -> {}", rec)?;
            }
        }

        Ok(())
    }
}

fn format_number(n: usize) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

/// Perform pre-flight analysis on discovered files
pub fn analyze(config: &AbyssConfig, files: &[PathBuf]) -> PreflightAnalysis {
    let total_files = files.len();

    // Calculate sizes and token estimates in parallel
    let (total_size, estimated_tokens): (u64, usize) = files
        .par_iter()
        .map(|f| {
            let size = fs::metadata(f).map(|m| m.len()).unwrap_or(0);
            // Token heuristic: ~4 chars per token
            let tokens = (size as usize) / 4;
            (size, tokens)
        })
        .reduce(
            || (0, 0),
            |(s1, t1), (s2, t2)| (s1 + s2, t1 + t2),
        );

    // Time estimate: ~100 files/sec for scanning, slower for large files
    let estimated_time_secs = (total_files as f64 / 100.0).max(0.5);

    // Generate recommendations
    let mut recommendations = Vec::new();

    // Check for test files
    let test_files = files
        .iter()
        .filter(|f| {
            let s = f.to_string_lossy().to_lowercase();
            s.contains("test") || s.contains("spec") || s.contains("__tests__")
        })
        .count();
    if test_files > total_files / 4 {
        recommendations.push(format!(
            "{}% of files are tests - consider adding 'test' to ignore patterns",
            test_files * 100 / total_files
        ));
    }

    // Check for generated files
    let generated_files = files
        .iter()
        .filter(|f| {
            let s = f.to_string_lossy().to_lowercase();
            s.contains(".generated.") || s.contains(".min.") || s.contains("bundle")
        })
        .count();
    if generated_files > 5 {
        recommendations.push(format!(
            "{} generated files detected - consider adding '*.generated.*' to ignore",
            generated_files
        ));
    }

    // Check for compression opportunity
    if config.compression == CompressionMode::None && estimated_tokens > 50_000 {
        recommendations.push("Use --smart compression to reduce tokens by ~40%".into());
    }

    // Check for git diff opportunity
    if config.diff.is_none() && total_files > 100 {
        if let Ok(git_dir) = config.path.join(".git").canonicalize() {
            if git_dir.exists() {
                recommendations.push("Use --diff main to scan only changed files".into());
            }
        }
    }

    // Token budget warning
    if let Some(max) = config.max_tokens {
        if estimated_tokens > max * 2 {
            recommendations.push(format!(
                "Estimated tokens ({}) exceed budget ({}) by {}x - many files will be dropped",
                format_number(estimated_tokens),
                format_number(max),
                estimated_tokens / max
            ));
        }
    }

    PreflightAnalysis {
        total_files,
        total_size_bytes: total_size,
        estimated_tokens,
        estimated_time_secs,
        recommendations,
    }
}
