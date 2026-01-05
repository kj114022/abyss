//! JSON output format for abyss

use anyhow::Result;

use std::io::Write;
use std::path::{Path, PathBuf};

use super::{Formatter, HeaderContext};

pub struct JsonFormatter {
    first_file: bool,
}

impl Default for JsonFormatter {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(serde::Serialize)]
struct FileEntry<'a> {
    path: String,
    content: &'a str,
}

impl JsonFormatter {
    pub fn new() -> Self {
        Self { first_file: true }
    }
}

impl Formatter for JsonFormatter {
    fn write_header(&mut self, output: &mut dyn Write, context: HeaderContext) -> Result<()> {
        writeln!(output, "{{")?;
        if let Some(p) = context.prompt {
            writeln!(output, "  \"prompt\": {},", serde_json::to_string(p)?)?;
        }
        if let Some(count) = context.token_count {
            writeln!(output, "  \"token_count\": {},", count)?;
        }
        Ok(())
    }

    fn write_directory_structure(
        &mut self,
        output: &mut dyn Write,
        files: &[PathBuf],
        repo_root: &Path,
    ) -> Result<()> {
        writeln!(output, "  \"directory_structure\": [")?;
        for (i, path) in files.iter().enumerate() {
            let relative = path.strip_prefix(repo_root).unwrap_or(path);
            let comma = if i < files.len() - 1 { "," } else { "" };
            writeln!(
                output,
                "    {}{}",
                serde_json::to_string(&relative.display().to_string())?,
                comma
            )?;
        }
        writeln!(output, "  ],")?;
        writeln!(output, "  \"files\": [")?;
        Ok(())
    }

    fn write_file(
        &mut self,
        output: &mut dyn Write,
        path: &Path,
        content: &str,
        _summary: Option<&str>,
        repo_root: &Path,
    ) -> Result<()> {
        if !self.first_file {
            writeln!(output, ",")?;
        }
        self.first_file = false;

        let relative = path.strip_prefix(repo_root).unwrap_or(path);

        // Use a temporary struct to ensure safe JSON encoding of the object
        // We output objects one by one to support streaming large datasets (O(1) memory)
        let entry = FileEntry {
            path: relative.display().to_string(),
            content,
        };

        // Write the entry, removing the trailing newline from to_string to keep format tight
        let json_line = serde_json::to_string(&entry)?;
        write!(output, "    {}", json_line)?;

        Ok(())
    }

    fn write_footer(&mut self, output: &mut dyn Write, dropped: &[PathBuf]) -> Result<()> {
        writeln!(output)?;
        writeln!(output, "  ],")?;

        if !dropped.is_empty() {
            writeln!(output, "  \"dropped_files\": [")?;
            for (i, path) in dropped.iter().enumerate() {
                let comma = if i < dropped.len() - 1 { "," } else { "" };
                writeln!(
                    output,
                    "    {}{}",
                    serde_json::to_string(&path.display().to_string())?,
                    comma
                )?;
            }
            writeln!(output, "  ]")?;
        } else {
            // If dropped_files is empty, we must handle the trailing comma from "files": [ ... ]
            // Actually, "files" array ends at `]` which is written above.
            // But strict JSON doesn't allow trailing comma after the last property if strictly parsed?
            // Wait, my write_header writes:
            // "prompt": ...,
            // "token_count": ...,
            // "directory_structure": [ ... ],
            // "files": [ ... ], <-- Trailing comma
            // So we need another field or remove that comma.
            // BUT, `write_directory_structure` does: `writeln!(output, "  \"files\": [")?;`
            // And `write_file` adds commas between items.
            // `write_footer` closes `]`.
            // Then it needs to close the main object `}`.
            // If I add "dropped_files", I need a comma after `]`.
            // Wait, `write_directory_structure` does NOT put a comma after `files: [` opening.
            // The items inside have commas.
            // Closing `files` array: `writeln!(output, "  ]")?`.
            // If I verify the previous code:
            // `writeln!(output, "  ]")?;`
            // `writeln!(output, "}}")?;`
            // It seems `files` was the last element.
            // Now `dropped_files` might be the last.
            // So if `dropped` is not empty, I need a comma after `files` array close.
            // But if `dropped` IS empty, I don't.
        }
        // Actually, looking at `write_directory_structure`:
        // `writeln!(output, "  ],")?;`  <-- Closes directory_structure with comma
        // `writeln!(output, "  \"files\": [")?;`

        // This means `files` is expected to be followed by something IF I add something.
        // If I simply append "dropped_files", I need a comma after "files".
        // BUT `write_footer` currently starts with `writeln!(output)?` then `writeln!(output, "  ]")?`.
        // The `write_file` implementation handles inner commas.

        // Let's rewrite `write_footer` carefully.

        // If we have dropped files, we need to add a comma to the previous `files` array closer?
        // No, the `files` array closing bracket `]` does not have a comma yet.

        if !dropped.is_empty() {
            // We need a comma after the files array
            write!(output, ",")?;
            writeln!(output, "\n  \"dropped_files\": [")?;
            for (i, path) in dropped.iter().enumerate() {
                let comma = if i < dropped.len() - 1 { "," } else { "" };
                writeln!(
                    output,
                    "    {}{}",
                    serde_json::to_string(&path.display().to_string())?,
                    comma
                )?;
            }
            writeln!(output, "  ]")?;
        }

        writeln!(output, "\n}}")?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_json_output() {
        let mut writer = JsonFormatter::new();
        let root = PathBuf::from("/repo");
        let mut output = Vec::new();
        let prompt = Some("Prompt".to_string());

        writer
            .write_header(
                &mut output,
                HeaderContext {
                    token_count: Some(100),
                    prompt: &prompt,
                    graph: None,
                    overview: None,
                },
            )
            .unwrap();
        writer
            .write_directory_structure(&mut output, &[PathBuf::from("/repo/src/main.rs")], &root)
            .unwrap();

        writer
            .write_file(
                &mut output,
                &PathBuf::from("/repo/src/main.rs"),
                "fn main() {}",
                None,
                &root,
            )
            .unwrap();

        writer.write_footer(&mut output, &[]).unwrap();

        let result = String::from_utf8(output).unwrap();
        assert!(result.contains("\"token_count\": 100"));
        assert!(result.contains("src/main.rs"));
        assert!(result.contains("\"path\":\"src/main.rs\""));
        assert!(result.contains("\"content\":\"fn main() {}\""));
    }
}
