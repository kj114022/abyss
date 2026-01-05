//! Plain text output format for abyss

use anyhow::Result;
use std::io::Write;
use std::path::{Path, PathBuf};

use super::{Formatter, HeaderContext};

pub struct PlainFormatter;

impl Formatter for PlainFormatter {
    fn write_header(&mut self, output: &mut dyn Write, context: HeaderContext) -> Result<()> {
        writeln!(output, "=== REPOSITORY CONTEXT ===")?;
        if let Some(p) = context.prompt {
            writeln!(output, "Instruction:")?;
            writeln!(output, "{}", p)?;
            writeln!(output)?;
        }
        if let Some(count) = context.token_count {
            writeln!(output, "Total tokens: {}", count)?;
        }
        writeln!(output)?;
        Ok(())
    }

    fn write_directory_structure(
        &mut self,
        output: &mut dyn Write,
        files: &[PathBuf],
        repo_root: &Path,
    ) -> Result<()> {
        writeln!(output, "=== DIRECTORY STRUCTURE ===")?;
        for path in files {
            let relative = path.strip_prefix(repo_root).unwrap_or(path);
            writeln!(output, "{}", relative.display())?;
        }
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

        writeln!(output, "--- {} ---", relative.display())?;
        if let Some(s) = summary {
            writeln!(output, "Summary: {}", s)?;
        }
        writeln!(output, "{}", content)?;
        writeln!(output)?;
        Ok(())
    }

    fn write_footer(&mut self, output: &mut dyn Write, dropped: &[PathBuf]) -> Result<()> {
        if !dropped.is_empty() {
            writeln!(output, "=== DROPPED FILES ===")?;
            for path in dropped {
                writeln!(output, "- {}", path.display())?;
            }
            writeln!(output)?;
        }
        writeln!(output, "=== END OF CONTEXT ===")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plain_file() {
        let mut output = Vec::new();
        let root = PathBuf::from("/repo");
        let path = PathBuf::from("/repo/src/main.rs");
        let mut formatter = PlainFormatter;

        formatter
            .write_file(&mut output, &path, "fn main() {}", None, &root)
            .unwrap();

        let result = String::from_utf8(output).unwrap();
        assert!(result.contains("--- src/main.rs ---"));
        assert!(result.contains("fn main() {}"));
    }
}
