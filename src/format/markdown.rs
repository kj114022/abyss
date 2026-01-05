//! Markdown output format for abyss

use anyhow::Result;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::{Formatter, HeaderContext};

pub struct MarkdownFormatter;

impl Formatter for MarkdownFormatter {
    fn write_header(&mut self, output: &mut dyn Write, context: HeaderContext) -> Result<()> {
        writeln!(output, "# Repository Context")?;
        writeln!(output)?;

        // Executive Summary
        if let Some(overview) = context.overview {
            writeln!(output, "## Executive Summary")?;
            writeln!(output)?;
            if let Some(purpose) = &overview.purpose {
                writeln!(output, "> **Purpose**")?;
                writeln!(output, "> {}", purpose.replace("\n", "\n> "))?;
                writeln!(output)?;
            }
            if !overview.key_files.is_empty() {
                writeln!(output, "### Key Modules")?;
                writeln!(output, "| File | Summary |")?;
                writeln!(output, "|------|---------|")?;
                for (path, summary) in &overview.key_files {
                    writeln!(output, "| {} | {} |", path.display(), summary)?;
                }
                writeln!(output)?;
            }
            if let Some(changes) = &overview.changes {
                writeln!(output, "### Recent Evolution")?;
                writeln!(output, "*Latest changes from git history:*")?;
                for msg in changes.iter().take(5) {
                    // Show top 5 commits
                    writeln!(output, "- {}", msg)?;
                }
                writeln!(output)?;
            }
        }

        if let Some(g) = context.graph {
            writeln!(output, "## Dependency Graph")?;
            writeln!(output)?;
            writeln!(output, "```mermaid")?;
            writeln!(output, "{}", g)?;
            writeln!(output, "```")?;
            writeln!(output)?;
        }

        if let Some(p) = context.prompt {
            writeln!(output, "> **Instruction**")?;
            writeln!(output, "> {}", p.replace("\n", "\n> "))?; // Quote the prompt
            writeln!(output)?;
        }
        if let Some(count) = context.token_count {
            writeln!(output, "> Total tokens: {}", count)?;
            writeln!(output)?;
        }
        Ok(())
    }

    fn write_directory_structure(
        &mut self,
        output: &mut dyn Write,
        files: &[PathBuf],
        repo_root: &Path,
    ) -> Result<()> {
        writeln!(output, "## Directory Structure")?;
        writeln!(output)?;
        writeln!(output, "```")?;
        for path in files {
            let relative = path.strip_prefix(repo_root).unwrap_or(path);
            writeln!(output, "{}", relative.display())?;
        }
        writeln!(output, "```")?;
        writeln!(output)?;
        Ok(())
    }

    fn write_file(
        &mut self,
        output: &mut dyn Write,
        path: &Path,
        content: &str,
        summary: Option<&str>,
        repo_root: &Path,
    ) -> Result<()> {
        let relative = path.strip_prefix(repo_root).unwrap_or(path);
        let extension = path.extension().and_then(|e| e.to_str()).unwrap_or("");

        // Map extensions to markdown language hints
        let lang = match extension {
            "rs" => "rust",
            "py" => "python",
            "js" => "javascript",
            "ts" => "typescript",
            "go" => "go",
            "c" | "h" => "c",
            "cpp" | "hpp" | "cc" => "cpp",
            "java" => "java",
            "rb" => "ruby",
            "sh" => "bash",
            "yml" | "yaml" => "yaml",
            "json" => "json",
            "toml" => "toml",
            "md" => "markdown",
            "html" => "html",
            "css" => "css",
            "sql" => "sql",
            _ => "",
        };

        writeln!(output, "## {}", relative.display())?;
        if let Some(s) = summary {
            writeln!(output, "> *summary: {}*", s)?;
        }
        writeln!(output)?;
        writeln!(output, "```{}", lang)?;
        writeln!(output, "{}", content)?;
        writeln!(output, "```")?;
        writeln!(output)?;
        Ok(())
    }

    fn write_footer(&mut self, output: &mut dyn Write, dropped: &[PathBuf]) -> Result<()> {
        if !dropped.is_empty() {
            writeln!(output, "## Dropped Files")?;
            writeln!(
                output,
                "The following files were excluded to fit within the token limit:"
            )?;
            writeln!(output)?;
            for path in dropped {
                writeln!(output, "- {}", path.display())?;
            }
            writeln!(output)?;
        }
        writeln!(output, "---")?;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_markdown_file() {
        let mut output = Vec::new();
        let root = PathBuf::from("/repo");
        let path = PathBuf::from("/repo/src/main.rs");
        let mut formatter = MarkdownFormatter;

        formatter
            .write_file(&mut output, &path, "fn main() {}", None, &root)
            .unwrap();

        let result = String::from_utf8(output).unwrap();
        assert!(result.contains("## src/main.rs"));
        assert!(result.contains("```rust"));
        assert!(result.contains("fn main() {}"));
    }
}
