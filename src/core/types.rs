//! Core types shared across Abyss modules

use std::path::PathBuf;

/// Events emitted during the scanning process
#[derive(Debug, Clone)]
pub enum ScanEvent {
    /// Scanning has started
    StartScanning,
    /// Number of files discovered
    FilesFound(usize),
    /// A file has been processed
    FileProcessed(PathBuf),
    /// Token count update
    TokenCountUpdate(usize),
    /// Scanning complete with message
    Complete(String),
    /// Error occurred
    Error(String),
}

/// Result of file discovery
#[derive(Debug, Clone)]
pub struct DiscoveryResult {
    /// Files discovered with their repository roots
    pub files: Vec<(PathBuf, PathBuf)>,
    /// Files that were dropped (e.g., too large)
    pub dropped: Vec<PathBuf>,
}

impl DiscoveryResult {
    pub fn new(files: Vec<(PathBuf, PathBuf)>, dropped: Vec<PathBuf>) -> Self {
        Self { files, dropped }
    }

    pub fn file_count(&self) -> usize {
        self.files.len()
    }
}

/// File metadata for scoring and analysis
#[derive(Debug, Clone, Default)]
pub struct FileMetadata {
    pub path: PathBuf,
    pub root: PathBuf,
    pub entropy: f64,
    pub tokens: usize,
    pub imports: Vec<String>,
}

impl FileMetadata {
    pub fn new(path: PathBuf, root: PathBuf) -> Self {
        Self {
            path,
            root,
            entropy: 0.0,
            tokens: 0,
            imports: Vec::new(),
        }
    }
}
