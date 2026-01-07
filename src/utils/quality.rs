//! Context quality scoring - analyze generated context quality

use std::collections::HashSet;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

use crate::utils::graph::DependencyGraph;

/// Quality grade for context
#[derive(Debug, Clone, Copy)]
pub enum QualityGrade {
    A,
    B,
    C,
    D,
    F,
}

impl std::fmt::Display for QualityGrade {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            QualityGrade::A => write!(f, "A (excellent)"),
            QualityGrade::B => write!(f, "B (good)"),
            QualityGrade::C => write!(f, "C (acceptable)"),
            QualityGrade::D => write!(f, "D (poor)"),
            QualityGrade::F => write!(f, "F (insufficient)"),
        }
    }
}

/// Context quality analysis results
#[derive(Debug)]
pub struct QualityScore {
    pub dependency_coverage: f64,
    pub token_distribution: TokenDistribution,
    pub recency_score: f64,
    pub overall_grade: QualityGrade,
    pub suggestions: Vec<String>,
}

#[derive(Debug)]
pub enum TokenDistribution {
    Balanced,
    Skewed,
    VerySkewed,
}

impl std::fmt::Display for TokenDistribution {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TokenDistribution::Balanced => write!(f, "balanced"),
            TokenDistribution::Skewed => write!(f, "skewed"),
            TokenDistribution::VerySkewed => write!(f, "very skewed (80% in 20% of files)"),
        }
    }
}

impl std::fmt::Display for QualityScore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f, "Context Quality Analysis")?;
        writeln!(f, "========================")?;
        writeln!(
            f,
            "Dependency coverage: {:.0}%",
            self.dependency_coverage * 100.0
        )?;
        writeln!(f, "Token distribution:  {}", self.token_distribution)?;
        writeln!(f, "Recency score:       {:.0}%", self.recency_score * 100.0)?;
        writeln!(f, "Overall grade:       {}", self.overall_grade)?;

        if !self.suggestions.is_empty() {
            writeln!(f)?;
            writeln!(f, "Suggestions:")?;
            for s in &self.suggestions {
                writeln!(f, "  -> {}", s)?;
            }
        }

        Ok(())
    }
}

/// Analyze context quality based on selected files
pub fn analyze_quality(
    selected_files: &[PathBuf],
    all_files: &[PathBuf],
    graph: &DependencyGraph,
    file_tokens: &[(PathBuf, usize)],
) -> QualityScore {
    // Dependency coverage: what % of graph nodes are in selected files
    let selected_set: HashSet<_> = selected_files.iter().collect();
    let graph_nodes = graph.node_count();
    let covered_nodes = selected_files
        .iter()
        .filter(|f| graph.has_node(f))
        .count();
    let dependency_coverage = if graph_nodes > 0 {
        covered_nodes as f64 / graph_nodes as f64
    } else {
        1.0 // No dependencies = full coverage
    };

    // Token distribution: check if 80% of tokens are in 20% of files
    let mut token_counts: Vec<usize> = file_tokens.iter().map(|(_, t)| *t).collect();
    token_counts.sort_by(|a, b| b.cmp(a)); // Descending

    let total_tokens: usize = token_counts.iter().sum();
    let top_20_percent_count = (token_counts.len() / 5).max(1);
    let top_20_tokens: usize = token_counts.iter().take(top_20_percent_count).sum();

    let token_distribution = if total_tokens > 0 {
        let ratio = top_20_tokens as f64 / total_tokens as f64;
        if ratio > 0.9 {
            TokenDistribution::VerySkewed
        } else if ratio > 0.7 {
            TokenDistribution::Skewed
        } else {
            TokenDistribution::Balanced
        }
    } else {
        TokenDistribution::Balanced
    };

    // Recency: % of files modified in last 30 days
    let thirty_days_ago = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs().saturating_sub(30 * 24 * 60 * 60))
        .unwrap_or(0);

    let recent_files = selected_files
        .iter()
        .filter(|f| {
            std::fs::metadata(f)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(UNIX_EPOCH).ok())
                .map(|d| d.as_secs() > thirty_days_ago)
                .unwrap_or(false)
        })
        .count();

    let recency_score = if !selected_files.is_empty() {
        recent_files as f64 / selected_files.len() as f64
    } else {
        0.0
    };

    // Calculate grade
    let mut score = 0.0;
    score += dependency_coverage * 40.0;
    score += match token_distribution {
        TokenDistribution::Balanced => 30.0,
        TokenDistribution::Skewed => 20.0,
        TokenDistribution::VerySkewed => 10.0,
    };
    score += recency_score * 30.0;

    let overall_grade = if score >= 85.0 {
        QualityGrade::A
    } else if score >= 70.0 {
        QualityGrade::B
    } else if score >= 55.0 {
        QualityGrade::C
    } else if score >= 40.0 {
        QualityGrade::D
    } else {
        QualityGrade::F
    };

    // Generate suggestions
    let mut suggestions = Vec::new();

    if dependency_coverage < 0.8 {
        suggestions.push("Low dependency coverage - some modules may have missing context".into());
    }

    if matches!(
        token_distribution,
        TokenDistribution::Skewed | TokenDistribution::VerySkewed
    ) {
        suggestions.push("Consider --smart compression to balance token distribution".into());
    }

    if recency_score < 0.3 {
        suggestions.push("Most files are stale - context may not reflect current state".into());
    }

    let coverage = selected_files.len() as f64 / all_files.len().max(1) as f64;
    if coverage < 0.5 {
        suggestions.push(format!(
            "Only {:.0}% of files included - increase --max-tokens for more coverage",
            coverage * 100.0
        ));
    }

    QualityScore {
        dependency_coverage,
        token_distribution,
        recency_score,
        overall_grade,
        suggestions,
    }
}
