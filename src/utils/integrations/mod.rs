//! Integration utilities for external services
//!
//! Contains Git integration, file watching, workspace support, caching, and bundling.

pub mod bundle;
pub mod cache;
pub mod diff_explainer;
pub mod git_stats;
pub mod watch;
pub mod workspace;

// Re-export commonly used items
pub use cache::Cache;
pub use diff_explainer::DiffExplainer;
pub use git_stats::{get_diff_files, get_git_stats};
pub use watch::{Debouncer, FileWatcher, WatchEvent};
pub use workspace::{is_workspace_file, load_workspace_config};
