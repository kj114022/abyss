//! Output format modules for abyss

pub mod json;
pub mod markdown;
pub mod mermaid;
pub mod plain;

use anyhow::Result;
use std::io::Write;
use std::path::{Path, PathBuf};

// Re-export XML functions with original names for backward compatibility
pub use self::xml::*;

use crate::config::OutputFormat;

/// Escapes special XML characters for use in attribute values
fn escape_xml_attr(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[derive(Debug, Clone)]
pub struct RepoOverview {
    pub purpose: Option<String>,
    pub key_files: Vec<(PathBuf, String)>,
    pub changes: Option<Vec<String>>, // New field for recent commits
}

pub struct HeaderContext<'a> {
    pub token_count: Option<usize>,
    pub prompt: &'a Option<String>,
    pub graph: Option<&'a str>,
    pub overview: Option<&'a RepoOverview>,
}

pub trait Formatter {
    fn write_header(&mut self, output: &mut dyn Write, context: HeaderContext) -> Result<()>;

    fn write_directory_structure(
        &mut self,
        output: &mut dyn Write,
        files: &[PathBuf],
        repo_root: &Path,
    ) -> Result<()>;

    fn write_file(
        &mut self,
        output: &mut dyn Write,
        path: &Path,
        content: &str,
        summary: Option<&str>,
        repo_root: &Path,
    ) -> Result<()>;

    fn write_footer(&mut self, output: &mut dyn Write, dropped: &[PathBuf]) -> Result<()>;
}

pub fn create_formatter(format: OutputFormat) -> Box<dyn Formatter> {
    match format {
        OutputFormat::Xml => Box::new(xml::XmlFormatter),
        OutputFormat::Json => Box::new(json::JsonFormatter::new()),
        OutputFormat::Markdown => Box::new(markdown::MarkdownFormatter),
        OutputFormat::Plain => Box::new(plain::PlainFormatter),
    }
}

/// XML format functions (default)
pub mod xml {
    use super::*;

    pub struct XmlFormatter;

    impl Formatter for XmlFormatter {
        fn write_header(&mut self, output: &mut dyn Write, context: HeaderContext) -> Result<()> {
            writeln!(output, "<abyss>")?;

            // Executive Summary (XML)
            if let Some(overview) = context.overview {
                writeln!(output, "<executive_summary>")?;
                if let Some(purpose) = &overview.purpose {
                    writeln!(output, "    <purpose><![CDATA[{}]]></purpose>", purpose)?;
                }
                if !overview.key_files.is_empty() {
                    writeln!(output, "    <key_modules>")?;
                    for (path, summary) in &overview.key_files {
                        writeln!(
                            output,
                            "        <module path=\"{}\" summary=\"{}\" />",
                            path.display(),
                            summary
                        )?;
                    }
                    writeln!(output, "    </key_modules>")?;
                }
                if let Some(changes) = &overview.changes {
                    writeln!(output, "    <recent_changes>")?;
                    for msg in changes.iter().take(5) {
                        writeln!(output, "        <change><![CDATA[{}]]></change>", msg)?;
                    }
                    writeln!(output, "    </recent_changes>")?;
                }
                writeln!(output, "</executive_summary>")?;
            }

            if let Some(g) = context.graph {
                writeln!(output, "<graph>")?;
                writeln!(output, "    <![CDATA[")?;
                writeln!(output, "{}", g)?;
                writeln!(output, "    ]]>")?;
                writeln!(output, "</graph>")?;
            }
            if let Some(p) = context.prompt {
                writeln!(output, "<prompt>")?;
                writeln!(output, "    <![CDATA[")?;
                let escaped = p.replace("]]>", "]]]]><![CDATA[>");
                writeln!(output, "{}", escaped)?;
                writeln!(output, "    ]]>")?;
                writeln!(output, "</prompt>")?;
            }
            if let Some(count) = context.token_count {
                writeln!(output, "<token_count>{}</token_count>", count)?;
            }
            Ok(())
        }

        fn write_directory_structure(
            &mut self,
            output: &mut dyn Write,
            files: &[PathBuf],
            repo_root: &Path,
        ) -> Result<()> {
            writeln!(output, "<directory_structure>")?;
            for path in files {
                let relative = path.strip_prefix(repo_root).unwrap_or(path);
                writeln!(output, "{}", relative.display())?;
            }
            writeln!(output, "</directory_structure>")?;
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

            if let Some(s) = summary {
                writeln!(
                    output,
                    "<file path=\"{}\" summary=\"{}\">",
                    relative.display(),
                    escape_xml_attr(s)
                )?;
            } else {
                writeln!(output, "<file path=\"{}\">", relative.display())?;
            }
            writeln!(output, "    <![CDATA[")?;
            // Escape CDATA terminators if present
            let escaped = content.replace("]]>", "]]]]><![CDATA[>");
            writeln!(output, "{}", escaped)?;
            writeln!(output, "    ]]>")?;
            writeln!(output, "</file>")?;
            Ok(())
        }

        fn write_footer(&mut self, output: &mut dyn Write, dropped: &[PathBuf]) -> Result<()> {
            if !dropped.is_empty() {
                writeln!(output, "<dropped_files>")?;
                for path in dropped {
                    writeln!(output, "    <file>{}</file>", path.display())?;
                }
                writeln!(output, "</dropped_files>")?;
            }
            writeln!(output, "</abyss>")?;
            Ok(())
        }
    }

    // Legacy functions for backward compatibility (if needed by tests)
    pub fn write_header(
        output: &mut impl Write,
        token_count: Option<usize>,
        prompt: &Option<String>,
        graph: Option<&str>,
    ) -> Result<()> {
        XmlFormatter.write_header(
            output,
            HeaderContext {
                token_count,
                prompt,
                graph,
                overview: None,
            },
        )
    }
    pub fn write_footer(output: &mut impl Write, dropped: &[PathBuf]) -> Result<()> {
        XmlFormatter.write_footer(output, dropped)
    }
    pub fn write_file(
        output: &mut impl Write,
        path: &Path,
        content: &str,
        root: &Path,
    ) -> Result<()> {
        XmlFormatter.write_file(output, path, content, None, root)
    }
    pub fn write_directory_structure(o: &mut impl Write, f: &[PathBuf], r: &Path) -> Result<()> {
        XmlFormatter.write_directory_structure(o, f, r)
    }
}
