use anyhow::Result;
use std::path::Path;

/// Extracts text from a PDF file.
pub fn extract_text(path: &Path) -> Result<String> {
    // let bytes = std::fs::read(path).context("Failed to read PDF file")?;

    // Attempt to extract text using pdf-extract
    // Note: pdf-extract is a wrapper around pdf, but we check if it works.
    match pdf_extract::extract_text(path) {
        Ok(text) => {
            if text.trim().is_empty() {
                Ok("[PDF: content appears empty or scanned]".to_string())
            } else {
                Ok(text)
            }
        }
        Err(_) => {
            // Fallback or error message
            Ok("[PDF: Text extraction failed - possibly scanned or encrypted]".to_string())
        }
    }
}
