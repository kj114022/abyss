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

/// Multi-tier compression levels for fine-grained control
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
pub enum CompressionLevel {
    /// No compression - full source code
    #[default]
    None,
    /// Light - remove comments and extra whitespace only
    Light,
    /// Standard - remove comments, whitespace, and simple boilerplate
    Standard,
    /// Aggressive - replace function bodies with placeholders, keep signatures
    Aggressive,
}

impl CompressionLevel {
    /// Convert to the legacy CompressionMode for backward compatibility
    pub fn to_compression_mode(self) -> CompressionMode {
        match self {
            CompressionLevel::None => CompressionMode::None,
            CompressionLevel::Light => CompressionMode::Simple,
            CompressionLevel::Standard => CompressionMode::Simple,
            CompressionLevel::Aggressive => CompressionMode::Smart,
        }
    }

    /// Parse from string (for CLI)
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "none" | "0" => Some(CompressionLevel::None),
            "light" | "1" => Some(CompressionLevel::Light),
            "standard" | "2" => Some(CompressionLevel::Standard),
            "aggressive" | "3" => Some(CompressionLevel::Aggressive),
            _ => None,
        }
    }
}

impl std::fmt::Display for CompressionLevel {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CompressionLevel::None => write!(f, "none"),
            CompressionLevel::Light => write!(f, "light"),
            CompressionLevel::Standard => write!(f, "standard"),
            CompressionLevel::Aggressive => write!(f, "aggressive"),
        }
    }
}

/// Main configuration for abyss
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
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
    /// Multi-tier compression level (None, Light, Standard, Aggressive)
    pub compression_level: CompressionLevel,
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
    pub diff: Option<String>,
    /// Include Mermaid dependency graph in output
    pub graph: bool,
    /// Path to export portable bundle
    pub bundle: Option<PathBuf>,
    /// Generate semantic explanation of diffs
    pub explain_diff: bool,
}

/// Workspace configuration for multi-repository merging
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceConfig {
    /// List of repositories to include
    pub repositories: Vec<WorkspaceRepo>,
    /// Global output configuration
    pub output: Option<PathBuf>,
}

/// A repository within a workspace
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WorkspaceRepo {
    /// Path to the repository
    pub path: PathBuf,
    /// Optional name (prefix) for files from this repo
    pub name: Option<String>,
    /// Priority weight for token budget allocation (default: 1.0)
    pub weight: Option<f64>,
}

impl AbyssConfig {
    /// Validates the configuration, ensuring the path exists and patterns are valid.
    pub fn validate(&self) -> anyhow::Result<()> {
        if !self.path.exists() && !self.is_remote {
            anyhow::bail!("Path does not exist: {:?}", self.path);
        }

        // Validate glob patterns
        for pattern in &self.ignore_patterns {
            glob::Pattern::new(pattern)
                .map_err(|e| anyhow::anyhow!("Invalid ignore pattern '{}': {}", pattern, e))?;
        }
        for pattern in &self.include_patterns {
            glob::Pattern::new(pattern)
                .map_err(|e| anyhow::anyhow!("Invalid include pattern '{}': {}", pattern, e))?;
        }

        Ok(())
    }

    /// Attempts to load configuration from `abyss.toml` in the current directory.
    /// Returns None if file doesn't exist, Err on parse failure.
    pub fn load_from_file() -> Option<Self> {
        let content = std::fs::read_to_string("abyss.toml").ok()?;
        match toml::from_str(&content) {
            Ok(config) => Some(config),
            Err(e) => {
                eprintln!("Warning: Failed to parse abyss.toml: {}", e);
                None
            }
        }
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
            compression_level: CompressionLevel::None,
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
            bundle: None,
            explain_diff: false,
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
            compression_level: CompressionLevel::None,
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
            bundle: None,
            explain_diff: false,
        };
        assert!(config.validate().is_err());
    }
}
