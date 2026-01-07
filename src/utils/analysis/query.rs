//! Query-driven context generation
//!
//! Implements semantic search to find the most relevant files for a natural language query.
//! This is the killer feature that differentiates abyss from simple file concatenation tools.

use rayon::prelude::*;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use crate::utils::graph::DependencyGraph;

/// Stopwords to filter from queries (common English words)
const STOPWORDS: &[&str] = &[
    "a", "an", "the", "is", "are", "was", "were", "be", "been", "being", "have", "has", "had",
    "do", "does", "did", "will", "would", "could", "should", "may", "might", "must", "shall",
    "can", "need", "dare", "ought", "used", "to", "of", "in", "for", "on", "with", "at", "by",
    "from", "as", "into", "through", "during", "before", "after", "above", "below", "between",
    "and", "but", "or", "nor", "so", "yet", "both", "either", "neither", "not", "only", "own",
    "same", "than", "too", "very", "just", "how", "what", "when", "where", "why", "which", "who",
    "whom", "this", "that", "these", "those", "i", "you", "he", "she", "it", "we", "they", "me",
    "him", "her", "us", "them", "my", "your", "his", "its", "our", "their", "work", "works",
    "does", "file", "files", "code", "function", "class",
];

/// Technical synonyms for common programming concepts
fn get_synonyms(word: &str) -> Vec<&'static str> {
    match word.to_lowercase().as_str() {
        "auth" | "authentication" | "authenticate" => vec![
            "auth",
            "login",
            "session",
            "credential",
            "token",
            "jwt",
            "oauth",
        ],
        "login" | "signin" | "sign-in" => {
            vec!["login", "auth", "signin", "session", "authenticate"]
        }
        "database" | "db" => vec![
            "database",
            "db",
            "sql",
            "query",
            "model",
            "schema",
            "migration",
        ],
        "api" | "endpoint" => vec!["api", "endpoint", "route", "handler", "controller", "rest"],
        "test" | "testing" => vec!["test", "spec", "mock", "assert", "expect"],
        "config" | "configuration" => vec!["config", "settings", "env", "environment", "options"],
        "error" | "exception" => vec!["error", "exception", "panic", "fail", "catch", "throw"],
        "user" | "account" => vec!["user", "account", "profile", "member"],
        "payment" | "billing" => vec![
            "payment",
            "billing",
            "stripe",
            "charge",
            "invoice",
            "subscription",
        ],
        "cache" | "caching" => vec!["cache", "redis", "memcache", "memoize"],
        _ => vec![],
    }
}

/// Query analysis result
#[derive(Debug, Clone)]
pub struct QueryAnalysis {
    /// Original query string
    pub query: String,
    /// Extracted keywords (lowercased, stopwords removed)
    pub keywords: Vec<String>,
    /// Expanded keywords including synonyms
    pub expanded_keywords: HashSet<String>,
}

impl QueryAnalysis {
    /// Analyze a natural language query and extract meaningful keywords
    pub fn from_query(query: &str) -> Self {
        // Tokenize: split on whitespace and punctuation
        let tokens: Vec<&str> = query
            .split(|c: char| c.is_whitespace() || c.is_ascii_punctuation())
            .filter(|s| !s.is_empty())
            .collect();

        // Filter stopwords and lowercase
        let keywords: Vec<String> = tokens
            .iter()
            .map(|s| s.to_lowercase())
            .filter(|s| s.len() > 2) // Skip very short words
            .filter(|s| !STOPWORDS.contains(&s.as_str()))
            .collect();

        // Expand with synonyms
        let mut expanded_keywords: HashSet<String> = keywords.iter().cloned().collect();
        for keyword in &keywords {
            for synonym in get_synonyms(keyword) {
                expanded_keywords.insert(synonym.to_string());
            }
        }

        Self {
            query: query.to_string(),
            keywords,
            expanded_keywords,
        }
    }
}

/// File relevance score for a query
#[derive(Debug, Clone)]
pub struct FileRelevance {
    pub path: PathBuf,
    pub score: f64,
    pub keyword_matches: usize,
    pub filename_match: bool,
    pub dependency_boost: f64,
}

impl std::fmt::Display for FileRelevance {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}: score={:.2} (matches={}, filename={}, dep_boost={:.2})",
            self.path.display(),
            self.score,
            self.keyword_matches,
            self.filename_match,
            self.dependency_boost
        )
    }
}

/// Query engine for finding relevant files
pub struct QueryEngine<'a> {
    analysis: QueryAnalysis,
    #[allow(dead_code)]
    graph: &'a DependencyGraph,
    pagerank: HashMap<PathBuf, f64>,
}

impl<'a> QueryEngine<'a> {
    /// Create a new query engine
    pub fn new(query: &str, graph: &'a DependencyGraph) -> Self {
        let analysis = QueryAnalysis::from_query(query);
        let pagerank = graph.calculate_pagerank();

        Self {
            analysis,
            graph,
            pagerank,
        }
    }

    /// Score all files by relevance to query
    pub fn score_files(&self, files: &[PathBuf]) -> Vec<FileRelevance> {
        files
            .par_iter()
            .filter_map(|path| self.score_file(path))
            .collect()
    }

    /// Score a single file
    fn score_file(&self, path: &PathBuf) -> Option<FileRelevance> {
        // Read file content
        let content = fs::read_to_string(path).ok()?;
        let content_lower = content.to_lowercase();

        // Count keyword matches in content
        let keyword_matches: usize = self
            .analysis
            .expanded_keywords
            .iter()
            .filter(|kw| content_lower.contains(kw.as_str()))
            .count();

        // Check filename match
        let filename = path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_lowercase();
        let filename_match = self
            .analysis
            .expanded_keywords
            .iter()
            .any(|kw| filename.contains(kw.as_str()));

        // Skip files with no matches
        if keyword_matches == 0 && !filename_match {
            return None;
        }

        // Get PageRank boost
        let pagerank_boost = self.pagerank.get(path).copied().unwrap_or(0.0);

        // Calculate final score
        let mut score = 0.0;

        // Content matches (main signal)
        score += keyword_matches as f64 * 10.0;

        // Filename match bonus (strong signal)
        if filename_match {
            score += 50.0;
        }

        // PageRank boost (centrality matters)
        score += pagerank_boost * 100.0;

        // Normalize by file size (prefer smaller, focused files)
        let size_factor = 1.0 / (1.0 + (content.len() as f64 / 10000.0).ln());
        score *= size_factor;

        Some(FileRelevance {
            path: path.clone(),
            score,
            keyword_matches,
            filename_match,
            dependency_boost: pagerank_boost,
        })
    }

    /// Get top N most relevant files
    pub fn get_top_files(&self, files: &[PathBuf], limit: usize) -> Vec<PathBuf> {
        let mut scored = self.score_files(files);
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        scored.into_iter().take(limit).map(|r| r.path).collect()
    }

    /// Get files that match query within token budget
    pub fn get_files_within_budget(
        &self,
        files: &[PathBuf],
        max_tokens: usize,
        file_tokens: &HashMap<PathBuf, usize>,
    ) -> Vec<PathBuf> {
        let mut scored = self.score_files(files);
        scored.sort_by(|a, b| {
            b.score
                .partial_cmp(&a.score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut selected = Vec::new();
        let mut used_tokens = 0;

        for relevance in scored {
            let tokens = file_tokens.get(&relevance.path).copied().unwrap_or(0);
            if used_tokens + tokens <= max_tokens {
                selected.push(relevance.path);
                used_tokens += tokens;
            }
        }

        selected
    }

    /// Get keywords used in analysis
    pub fn keywords(&self) -> &[String] {
        &self.analysis.keywords
    }

    /// Get expanded keywords (including synonyms)
    pub fn expanded_keywords(&self) -> &HashSet<String> {
        &self.analysis.expanded_keywords
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_query_analysis() {
        let analysis = QueryAnalysis::from_query("how does authentication work?");
        assert!(analysis.keywords.contains(&"authentication".to_string()));
        assert!(analysis.expanded_keywords.contains("auth"));
        assert!(analysis.expanded_keywords.contains("login"));
        assert!(!analysis.keywords.contains(&"how".to_string())); // Stopword
    }

    #[test]
    fn test_stopwords_removed() {
        let analysis = QueryAnalysis::from_query("what is the database configuration?");
        assert!(!analysis.keywords.contains(&"what".to_string()));
        assert!(!analysis.keywords.contains(&"is".to_string()));
        assert!(!analysis.keywords.contains(&"the".to_string()));
        assert!(analysis.keywords.contains(&"database".to_string()));
        assert!(analysis.keywords.contains(&"configuration".to_string()));
    }

    #[test]
    fn test_synonyms_expanded() {
        let analysis = QueryAnalysis::from_query("payment processing");
        assert!(analysis.expanded_keywords.contains("payment"));
        assert!(analysis.expanded_keywords.contains("billing"));
        assert!(analysis.expanded_keywords.contains("stripe"));
    }

    #[test]
    fn test_short_words_filtered() {
        let analysis = QueryAnalysis::from_query("a is to be or");
        assert!(analysis.keywords.is_empty());
    }
}
