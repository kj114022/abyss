//! Core module for Abyss LLM Context Compiler
//!
//! This module contains the core types, scanner, and processing logic.

pub mod scanner;
mod types;

pub use scanner::discover_files;
pub use types::*;
