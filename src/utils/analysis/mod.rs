//! Analysis utilities for code intelligence
//!
//! Contains dependency graph, ranking, impact analysis, and query processing.

pub mod concepts;
pub mod dependencies;
pub mod graph;
pub mod impact;
pub mod preflight;
pub mod quality;
pub mod query;
pub mod rank;

// Re-export commonly used items
pub use graph::DependencyGraph;
pub use impact::ImpactAnalyzer;
pub use query::QueryEngine;
pub use rank::{FileScore, calculate_entropy, heuristic_score, sort_files, sort_paths};
