use crate::config::AbyssConfig;
use anyhow::{Context, Result};
use std::process::Command;

pub struct DiffExplainer;

impl DiffExplainer {
    pub fn explain(config: &AbyssConfig) -> Result<String> {
        let root = &config.path;
        let target = config.diff.as_deref().unwrap_or("HEAD~1");

        let diff_output = Command::new("git")
            .args(["diff", "-U0", target])
            .current_dir(root)
            .output()
            .context("Failed to run git diff")?;

        let diff_str = String::from_utf8_lossy(&diff_output.stdout);
        let mut explanation = String::new();

        explanation.push_str(&format!("Semantic Diff Explanation (vs {})\n", target));
        explanation.push_str("=============================\n\n");

        let mut current_file;

        for line in diff_str.lines() {
            if line.starts_with("diff --git") {
                // New file
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 3 {
                    // a/path/to/file b/path/to/file
                    if let Some(path) = parts.last() {
                        let p = path.trim_start_matches("b/");
                        current_file = p.to_string();
                        explanation.push_str(&format!(
                            "\nFile: {}\nRefactoring/Change detected.\n",
                            current_file
                        ));
                    }
                }
            } else if line.starts_with("@@") {
                // Hunk header: @@ -10,5 +15,7 @@ fn foo() {
                // Determine function context from hunk header if possible
                // Git usually puts function name at the end of @@ line
                let parts: Vec<&str> = line.split("@@").collect();
                if parts.len() >= 3 {
                    let context = parts[2].trim();
                    if !context.is_empty() {
                        explanation.push_str(&format!("  - In context: `{}`\n", context));
                    }
                }
            } else if line.starts_with('+') && !line.starts_with("+++") {
                // Added line
                if is_definition(line) {
                    explanation
                        .push_str(&format!("  - Added definition: `{}`\n", line[1..].trim()));
                }
            } else if line.starts_with('-') && !line.starts_with("---") {
                // Deleted line
                if is_definition(line) {
                    explanation.push_str(&format!(
                        "  - Removed/Modified definition: `{}`\n",
                        line[1..].trim()
                    ));
                }
            }
        }

        Ok(explanation)
    }
}

fn is_definition(line: &str) -> bool {
    let s = line[1..].trim();
    // Rust
    if s.starts_with("fn ")
        || s.starts_with("struct ")
        || s.starts_with("enum ")
        || s.starts_with("impl ")
        || s.starts_with("trait ")
    {
        return true;
    }
    // JS/TS
    if s.starts_with("function ")
        || s.starts_with("class ")
        || s.starts_with("interface ")
        || s.starts_with("const ")
    {
        // loose check for const fn?
        if s.contains("=>") || s.contains("function") {
            return true;
        }
    }
    // Python
    if s.starts_with("def ") || s.starts_with("class ") {
        return true;
    }
    false
}
