//! Hierarchical context strategy for tiered LLM outputs
//!
//! Generates multi-level context: summary -> detailed -> full
//! This helps LLMs maintain focus by providing appropriate context depth.

use std::collections::HashMap;
use std::path::PathBuf;

use crate::config::CompressionLevel;
use crate::utils::compression::compress_by_level;
use crate::utils::graph::DependencyGraph;

/// Context tier with different levels of detail
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ContextTier {
    /// Summary: just signatures and structure (~10% of full)
    Summary,
    /// Detailed: full interfaces, compressed implementations (~30% of full)
    Detailed,
    /// Full: complete source code
    Full,
}

impl ContextTier {
    /// Get the compression level for this tier
    pub fn compression_level(&self) -> CompressionLevel {
        match self {
            ContextTier::Summary => CompressionLevel::Aggressive,
            ContextTier::Detailed => CompressionLevel::Standard,
            ContextTier::Full => CompressionLevel::None,
        }
    }

    /// Get the token budget multiplier for this tier
    pub fn budget_multiplier(&self) -> f64 {
        match self {
            ContextTier::Summary => 0.1,
            ContextTier::Detailed => 0.3,
            ContextTier::Full => 1.0,
        }
    }

    /// Parse from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "summary" | "s" | "1" => Some(ContextTier::Summary),
            "detailed" | "d" | "2" => Some(ContextTier::Detailed),
            "full" | "f" | "3" => Some(ContextTier::Full),
            _ => None,
        }
    }
}

impl std::fmt::Display for ContextTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ContextTier::Summary => write!(f, "summary"),
            ContextTier::Detailed => write!(f, "detailed"),
            ContextTier::Full => write!(f, "full"),
        }
    }
}

/// Hierarchical context result
#[derive(Debug)]
pub struct HierarchicalContext {
    /// Summary tier files (compressed signatures)
    pub summary: Vec<(PathBuf, String)>,
    /// Detailed tier files (interfaces + key implementations)
    pub detailed: Vec<(PathBuf, String)>,
    /// Full tier files (complete source)
    pub full: Vec<(PathBuf, String)>,
    /// Token estimates for each tier
    pub token_estimates: HashMap<ContextTier, usize>,
}

/// Generate hierarchical context from file contents
pub fn generate_hierarchical(
    files: &[(PathBuf, String)],
    graph: &DependencyGraph,
    base_budget: usize,
) -> HierarchicalContext {
    let pagerank = graph.calculate_pagerank();

    // Sort files by importance (PageRank)
    let mut scored_files: Vec<_> = files
        .iter()
        .map(|(path, content)| {
            let score = pagerank.get(path).copied().unwrap_or(0.0);
            (path.clone(), content.clone(), score)
        })
        .collect();

    scored_files.sort_by(|a, b| b.2.partial_cmp(&a.2).unwrap_or(std::cmp::Ordering::Equal));

    // Calculate budgets for each tier
    let summary_budget = (base_budget as f64 * ContextTier::Summary.budget_multiplier()) as usize;
    let detailed_budget = (base_budget as f64 * ContextTier::Detailed.budget_multiplier()) as usize;
    let full_budget = base_budget;

    let mut summary = Vec::new();
    let mut detailed = Vec::new();
    let mut full = Vec::new();

    let mut summary_tokens = 0;
    let mut detailed_tokens = 0;
    let mut full_tokens = 0;

    for (path, content, _score) in scored_files {
        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

        // Estimate tokens for each compression level
        let summary_content = compress_by_level(&content, CompressionLevel::Aggressive, extension);
        let detailed_content = compress_by_level(&content, CompressionLevel::Standard, extension);
        let full_content = content.clone();

        let summary_est = summary_content.len() / 4;
        let detailed_est = detailed_content.len() / 4;
        let full_est = full_content.len() / 4;

        // Add to appropriate tier based on budget
        if summary_tokens + summary_est <= summary_budget {
            summary.push((path.clone(), summary_content));
            summary_tokens += summary_est;
        }

        if detailed_tokens + detailed_est <= detailed_budget {
            detailed.push((path.clone(), detailed_content));
            detailed_tokens += detailed_est;
        }

        if full_tokens + full_est <= full_budget {
            full.push((path.clone(), full_content));
            full_tokens += full_est;
        }
    }

    let mut token_estimates = HashMap::new();
    token_estimates.insert(ContextTier::Summary, summary_tokens);
    token_estimates.insert(ContextTier::Detailed, detailed_tokens);
    token_estimates.insert(ContextTier::Full, full_tokens);

    HierarchicalContext {
        summary,
        detailed,
        full,
        token_estimates,
    }
}

/// Format hierarchical context for LLM consumption
pub fn format_hierarchical(context: &HierarchicalContext, tier: ContextTier) -> String {
    let files = match tier {
        ContextTier::Summary => &context.summary,
        ContextTier::Detailed => &context.detailed,
        ContextTier::Full => &context.full,
    };

    let tokens = context.token_estimates.get(&tier).copied().unwrap_or(0);

    let mut output = format!(
        "# Codebase Context ({})\n\nFiles: {} | Tokens: ~{}\n\n---\n\n",
        tier,
        files.len(),
        tokens
    );

    for (path, content) in files {
        output.push_str(&format!(
            "## {}\n\n```\n{}\n```\n\n",
            path.display(),
            content
        ));
    }

    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tier_compression() {
        assert_eq!(
            ContextTier::Summary.compression_level(),
            CompressionLevel::Aggressive
        );
        assert_eq!(
            ContextTier::Detailed.compression_level(),
            CompressionLevel::Standard
        );
        assert_eq!(
            ContextTier::Full.compression_level(),
            CompressionLevel::None
        );
    }

    #[test]
    fn test_tier_budgets() {
        assert!((ContextTier::Summary.budget_multiplier() - 0.1).abs() < 0.001);
        assert!((ContextTier::Detailed.budget_multiplier() - 0.3).abs() < 0.001);
        assert!((ContextTier::Full.budget_multiplier() - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_tier_from_str() {
        assert_eq!(ContextTier::from_str("summary"), Some(ContextTier::Summary));
        assert_eq!(
            ContextTier::from_str("detailed"),
            Some(ContextTier::Detailed)
        );
        assert_eq!(ContextTier::from_str("full"), Some(ContextTier::Full));
        assert_eq!(ContextTier::from_str("invalid"), None);
    }
}
