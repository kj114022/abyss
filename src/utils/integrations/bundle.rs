//! Portable bundle generation for sharing context snapshots
//!
//! Creates self-contained bundles with context + metadata + patches

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

/// Bundle metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleMetadata {
    /// Bundle format version
    pub version: String,
    /// Creation timestamp (ISO 8601)
    pub created_at: String,
    /// Git commit hash if available
    pub git_commit: Option<String>,
    /// Git branch if available
    pub git_branch: Option<String>,
    /// Number of files in bundle
    pub file_count: usize,
    /// Total token estimate
    pub token_estimate: usize,
    /// Compression level used
    pub compression: String,
    /// Query used for generation (if any)
    pub query: Option<String>,
    /// Custom notes
    pub notes: Option<String>,
}

/// A portable bundle containing context and metadata
#[derive(Debug, Serialize, Deserialize)]
pub struct Bundle {
    /// Metadata about the bundle
    pub metadata: BundleMetadata,
    /// File contents (path -> content)
    pub files: HashMap<String, String>,
    /// Dependency graph in Mermaid format
    pub graph: Option<String>,
    /// Summary/overview text
    pub summary: Option<String>,
}

impl Bundle {
    /// Create a new bundle from files
    pub fn new(
        files: Vec<(PathBuf, String)>,
        graph: Option<String>,
        compression: &str,
        query: Option<String>,
    ) -> Self {
        let file_count = files.len();
        let token_estimate: usize = files.iter().map(|(_, c)| c.len() / 4).sum();

        let file_map: HashMap<String, String> = files
            .into_iter()
            .map(|(p, c)| (p.to_string_lossy().to_string(), c))
            .collect();

        // Get git info
        let git_commit = std::process::Command::new("git")
            .args(["rev-parse", "HEAD"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());

        let git_branch = std::process::Command::new("git")
            .args(["rev-parse", "--abbrev-ref", "HEAD"])
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string());

        let metadata = BundleMetadata {
            // Bundle format version (independent of CLI version)
            version: "1.0".to_string(),
            created_at: chrono::Utc::now().to_rfc3339(),
            git_commit,
            git_branch,
            file_count,
            token_estimate,
            compression: compression.to_string(),
            query,
            notes: None,
        };

        Self {
            metadata,
            files: file_map,
            graph,
            summary: None,
        }
    }

    /// Add a summary/overview
    pub fn with_summary(mut self, summary: String) -> Self {
        self.summary = Some(summary);
        self
    }

    /// Add notes
    pub fn with_notes(mut self, notes: String) -> Self {
        self.metadata.notes = Some(notes);
        self
    }

    /// Save bundle as JSON
    pub fn save_json(&self, path: &Path) -> Result<(), String> {
        let json = serde_json::to_string_pretty(self)
            .map_err(|e| format!("Failed to serialize bundle: {}", e))?;

        fs::write(path, json).map_err(|e| format!("Failed to write bundle: {}", e))?;

        Ok(())
    }

    /// Save bundle as compressed tarball
    pub fn save_tar_gz(&self, path: &Path) -> Result<(), String> {
        use flate2::Compression;
        use flate2::write::GzEncoder;

        let file =
            fs::File::create(path).map_err(|e| format!("Failed to create bundle file: {}", e))?;

        let encoder = GzEncoder::new(file, Compression::default());
        let mut tar = tar::Builder::new(encoder);

        // Add metadata.json
        let metadata_json = serde_json::to_string_pretty(&self.metadata)
            .map_err(|e| format!("Failed to serialize metadata: {}", e))?;
        add_file_to_tar(&mut tar, "metadata.json", metadata_json.as_bytes())?;

        // Add each file
        for (filepath, content) in &self.files {
            let tar_path = format!("files/{}", filepath);
            add_file_to_tar(&mut tar, &tar_path, content.as_bytes())?;
        }

        // Add graph if present
        if let Some(graph) = &self.graph {
            add_file_to_tar(&mut tar, "graph.mermaid", graph.as_bytes())?;
        }

        // Add summary if present
        if let Some(summary) = &self.summary {
            add_file_to_tar(&mut tar, "summary.md", summary.as_bytes())?;
        }

        tar.finish()
            .map_err(|e| format!("Failed to finalize tar: {}", e))?;

        Ok(())
    }

    /// Load bundle from JSON
    pub fn load_json(path: &Path) -> Result<Self, String> {
        let content =
            fs::read_to_string(path).map_err(|e| format!("Failed to read bundle: {}", e))?;

        serde_json::from_str(&content).map_err(|e| format!("Failed to parse bundle: {}", e))
    }
}

fn add_file_to_tar<W: Write>(
    tar: &mut tar::Builder<W>,
    path: &str,
    content: &[u8],
) -> Result<(), String> {
    let mut header = tar::Header::new_gnu();
    header.set_path(path).map_err(|e| e.to_string())?;
    header.set_size(content.len() as u64);
    header.set_mode(0o644);
    header.set_cksum();

    tar.append(&header, content)
        .map_err(|e| format!("Failed to add {} to tar: {}", path, e))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bundle_creation() {
        let files = vec![
            (PathBuf::from("src/main.rs"), "fn main() {}".to_string()),
            (PathBuf::from("src/lib.rs"), "pub mod foo;".to_string()),
        ];

        let bundle = Bundle::new(files, None, "none", None);

        assert_eq!(bundle.metadata.file_count, 2);
        assert!(bundle.metadata.token_estimate > 0);
        assert_eq!(bundle.files.len(), 2);
    }

    #[test]
    fn test_bundle_with_summary() {
        let files = vec![(PathBuf::from("test.rs"), "test".to_string())];
        let bundle =
            Bundle::new(files, None, "none", None).with_summary("Test summary".to_string());

        assert_eq!(bundle.summary, Some("Test summary".to_string()));
    }
}
