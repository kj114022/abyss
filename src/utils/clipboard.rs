use anyhow::{Context, Result};
use arboard::Clipboard;

pub fn copy_to_clipboard(content: &str) -> Result<()> {
    let mut clipboard = Clipboard::new().context("Failed to initialize clipboard")?;
    clipboard
        .set_text(content)
        .context("Failed to set clipboard text")?;
    Ok(())
}
