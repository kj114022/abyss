//! Output format modules for abyss

pub mod json;
pub mod markdown;
pub mod plain;

use anyhow::Result;
use std::io::Write;
use std::path::{Path, PathBuf};

// Re-export XML functions with original names for backward compatibility
pub use self::xml::*;

use crate::config::OutputFormat;

pub trait Formatter {
    fn write_header(
        &mut self,
        output: &mut dyn Write,
        token_count: Option<usize>,
        prompt: &Option<String>,
    ) -> Result<()>;

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
        repo_root: &Path,
    ) -> Result<()>;

    fn write_footer(&mut self, output: &mut dyn Write) -> Result<()>;
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
        fn write_header(
            &mut self,
            output: &mut dyn Write,
            token_count: Option<usize>,
            prompt: &Option<String>,
        ) -> Result<()> {
            writeln!(output, "<abyss>")?;
            if let Some(p) = prompt {
                writeln!(output, "<prompt>")?;
                writeln!(output, "    <![CDATA[")?;
                let escaped = p.replace("]]>", "]]]]><![CDATA[>");
                writeln!(output, "{}", escaped)?;
                writeln!(output, "    ]]>")?;
                writeln!(output, "</prompt>")?;
            }
            if let Some(count) = token_count {
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
            repo_root: &Path,
        ) -> Result<()> {
            let relative = path.strip_prefix(repo_root).unwrap_or(path);

            writeln!(output, "<file path=\"{}\">", relative.display())?;
            writeln!(output, "    <![CDATA[")?;
            // Escape CDATA terminators if present
            let escaped = content.replace("]]>", "]]]]><![CDATA[>");
            writeln!(output, "{}", escaped)?;
            writeln!(output, "    ]]>")?;
            writeln!(output, "</file>")?;
            Ok(())
        }

        fn write_footer(&mut self, output: &mut dyn Write) -> Result<()> {
            writeln!(output, "</abyss>")?;
            Ok(())
        }
    }

    // Legacy functions for backward compatibility (if needed by tests)
    pub fn write_header(
        output: &mut impl Write,
        token_count: Option<usize>,
        prompt: &Option<String>,
    ) -> Result<()> {
        XmlFormatter.write_header(output, token_count, prompt)
    }
    pub fn write_footer(output: &mut impl Write) -> Result<()> {
        XmlFormatter.write_footer(output)
    }
    pub fn write_file(
        output: &mut impl Write,
        path: &Path,
        content: &str,
        root: &Path,
    ) -> Result<()> {
        XmlFormatter.write_file(output, path, content, root)
    }
    pub fn write_directory_structure(o: &mut impl Write, f: &[PathBuf], r: &Path) -> Result<()> {
        XmlFormatter.write_directory_structure(o, f, r)
    }
}
