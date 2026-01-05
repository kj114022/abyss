use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Output format for generated context
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum OutputFormat {
    #[default]
    Xml,
    Json,
    Markdown,
    Plain,
}

/// Compression mode for file content
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CompressionMode {
    #[default]
    None,
    Simple, // Regex-based comment/whitespace removal
    Smart,  // AST-aware compression
}

/// Main configuration for abyss
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AbyssConfig {
    /// Path to the repository or file to scan
    pub path: PathBuf,
    /// Path to the output file
    pub output: PathBuf,
    /// List of glob patterns to ignore (e.g. "*.log")
    pub ignore_patterns: Vec<String>,
    /// List of glob patterns to include (overrides ignore if matched)
    pub include_patterns: Vec<String>,
    /// If true, skip token counting (faster)
    pub no_tokens: bool,
    /// If true, copy output to system clipboard
    pub clipboard_copy: bool,
    /// Redact secrets and PII
    pub redact: bool,
    /// Compression mode for file content (None, Simple, or Smart AST-based)
    pub compression: CompressionMode,
    /// Split output into chunks of this many tokens
    pub split_tokens: Option<usize>,
    /// Enabled verbose logging to stdout
    pub verbose: bool,
    /// Internal flag for remote repository processing
    #[serde(skip)]
    pub is_remote: bool,
    /// Smart compression limit in bytes (files larger than this use simple compression)
    pub smart_limit: Option<usize>,
    /// Output format (XML, JSON, Markdown, Plain)
    pub output_format: OutputFormat,
    /// Maximum file size to include (in bytes)
    pub max_file_size: Option<usize>,
    /// Maximum directory depth to traverse
    pub max_depth: Option<usize>,
    /// Custom prompt/instruction to prepend to the output
    pub prompt: Option<String>,
    /// Maximum number of tokens to include in output
    pub max_tokens: Option<usize>,
    /// Diff mode: Scan only changed files relative to this git ref (e.g., "main", "HEAD~1")
    /// Diff mode: Scan only changed files relative to this git ref (e.g., "main", "HEAD~1")
    pub diff: Option<String>,
    /// Include Mermaid dependency graph in output
    pub graph: bool,
}

impl AbyssConfig {
    /// Validates the configuration, ensuring the path exists (for local scans).
    pub fn validate(&self) -> anyhow::Result<()> {
        if !self.path.exists() && !self.is_remote {
            anyhow::bail!("Path does not exist: {:?}", self.path);
        }
        Ok(())
    }

    /// Attempts to load configuration from `abyss.toml` in the current directory.
    /// Attempts to load configuration from `abyss.toml` in the current directory.
    pub fn load_from_file() -> Option<Self> {
        std::fs::read_to_string("abyss.toml")
            .ok()
            .and_then(|content| toml::from_str(&content).ok())
    }
}

impl Default for AbyssConfig {
    fn default() -> Self {
        let defaults = vec![
            // Version Control
            ".git",
            ".hg",
            ".svn",
            ".bzr",
            // IDEs
            ".idea",
            ".vscode",
            ".vs",
            ".settings",
            ".project",
            "*.swp",
            "*.swo",
            // Build / Dependency
            "node_modules",
            "target",
            "dist",
            "build",
            "out",
            "vendor",
            "venv",
            ".venv",
            "env",
            ".env",
            ".tox",
            "__pycache__",
            "*.pyc",
            "*.class",
            "*.o",
            // Lockfiles
            "package-lock.json",
            "yarn.lock",
            "pnpm-lock.yaml",
            "Cargo.lock",
            "Gemfile.lock",
            "composer.lock",
            // System
            ".DS_Store",
            "Thumbs.db",
            // Logs
            "*.log",
            "npm-debug.log*",
            "yarn-debug.log*",
            "yarn-error.log*",
            // Binary / Media / Compressed (Exclusions for LLM context)
            "*.exe",
            "*.dll",
            "*.so",
            "*.dylib",
            "*.bin",
            "*.jpg",
            "*.jpeg",
            "*.png",
            "*.gif",
            "*.ico",
            "*.svg",
            "*.webp",
            "*.mp3",
            "*.mp4",
            "*.avi",
            "*.mov",
            "*.pdf",
            "*.doc",
            "*.docx",
            "*.xls",
            "*.xlsx",
            "*.ppt",
            "*.pptx",
            "*.zip",
            "*.tar",
            "*.tar.gz",
            "*.7z",
            "*.rar",
            "*.db",
            "*.sqlite",
            "*.sqlite3",
        ];

        Self {
            path: PathBuf::from("."),
            output: PathBuf::from("abyss-output.xml"),
            ignore_patterns: defaults.into_iter().map(String::from).collect(),
            include_patterns: Vec::new(),
            no_tokens: false,
            clipboard_copy: false,
            compression: CompressionMode::None,
            split_tokens: None,
            verbose: false,
            is_remote: false,
            smart_limit: None,
            output_format: OutputFormat::Xml,
            max_file_size: None,
            max_depth: None,
            prompt: None,
            max_tokens: None,
            redact: false,
            diff: None,
            graph: false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_config_validation() {
        let config = AbyssConfig {
            path: PathBuf::from("non_existent_path_xyz_123"),
            output: PathBuf::from("output.xml"),
            ignore_patterns: vec![],
            include_patterns: vec![],
            no_tokens: false,
            clipboard_copy: false,
            compression: CompressionMode::None,
            split_tokens: None,
            verbose: false,
            is_remote: false,
            smart_limit: None,
            output_format: OutputFormat::Xml,
            max_file_size: None,
            max_depth: None,
            prompt: None,
            redact: false,
            diff: None,
            graph: false,
            max_tokens: None,
        };
        assert!(config.validate().is_err());
    }
}
