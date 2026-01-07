//! Compression utilities for content optimization
//!
//! Contains level-based compression, AST-aware compression, and hierarchical context.

pub mod ast;
pub mod compression;
pub mod hierarchy;

// Re-export commonly used items
pub use ast::compress_ast;
pub use compression::{
    compress_by_level, compress_content, compress_light, compress_standard, compression_ratio,
};
pub use hierarchy::ContextTier;
