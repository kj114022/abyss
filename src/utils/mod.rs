//! Utility modules for Abyss LLM Context Compiler
//!
//! Organized into logical groups:
//! - `analysis/` - Code intelligence (graph, ranking, impact, query)
//! - `compress/` - Content optimization (AST, levels, hierarchy)
//! - `integrations/` - External services (Git, watch, workspace, cache)

// Grouped submodules
pub mod analysis;
pub mod compress;
pub mod integrations;

// Remaining flat modules
pub mod abyssignore;
pub mod binary;
pub mod clipboard;
pub mod image;
pub mod pdf;
pub mod privacy;
pub mod summary;
pub mod tokens;

// ============================================================================
// BACKWARD COMPATIBILITY RE-EXPORTS
// These allow existing code to use the old paths like `utils::graph`
// ============================================================================

// Analysis re-exports
pub use analysis::concepts;
pub use analysis::dependencies;
pub use analysis::graph;
pub use analysis::impact;
pub use analysis::preflight;
pub use analysis::quality;
pub use analysis::query;
pub use analysis::rank;

// Compression re-exports (from compress/ to avoid naming conflict)
pub use compress::ast;
pub use compress::compression;
pub use compress::hierarchy;

// Integration re-exports
pub use integrations::bundle;
pub use integrations::cache;
pub use integrations::diff_explainer;
pub use integrations::git_stats;
pub use integrations::watch;
pub use integrations::workspace;
