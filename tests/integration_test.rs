use abyss::{AbyssConfig, CompressionMode, OutputFormat, run};
use std::fs;
use tempfile::TempDir;

#[test]
fn test_end_to_end_scan() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();

    // Create some files
    fs::write(root.join("main.rs"), "fn main() {}")?;
    fs::write(root.join("Cargo.toml"), "[package]\nname = \"test\"")?;
    fs::create_dir(root.join("src"))?;
    fs::write(root.join("src/lib.rs"), "pub fn test() {}")?;

    // Create ignored file
    fs::write(root.join("secret.env"), "SECRET=123")?;

    let output_path = root.join("output.xml");
    let config = AbyssConfig {
        path: root.to_path_buf(),
        output: output_path.clone(),
        ignore_patterns: vec!["*.env".to_string()],
        include_patterns: vec![],
        no_tokens: false,
        clipboard_copy: false,
        compression: CompressionMode::None,
        smart_limit: None,
        split_tokens: None,
        verbose: true,
        is_remote: false,
        output_format: OutputFormat::Xml,
        max_file_size: None,
        max_depth: None,
        prompt: None,
        redact: false,
        diff: None,
        max_tokens: None,
        graph: false,
        compression_level: abyss::config::CompressionLevel::None,
        bundle: None,
        explain_diff: false,
    };

    run(config)?;

    assert!(output_path.exists());
    let content = fs::read_to_string(output_path)?;

    assert!(content.contains("<directory_structure>"));
    assert!(content.contains("main.rs"));
    assert!(content.contains("Cargo.toml"));
    assert!(content.contains("src/lib.rs"));
    assert!(!content.contains("secret.env"));
    assert!(content.contains("fn main() {}"));

    Ok(())
}

#[test]
fn test_compression_integration() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();
    let file_path = root.join("code.rs");
    fs::write(&file_path, "fn code() {\n // comment \n}")?;

    let output_path = root.join("compressed.xml");
    let config = AbyssConfig {
        path: root.to_path_buf(),
        output: output_path.clone(),
        ignore_patterns: vec![],
        include_patterns: vec![],
        no_tokens: true,
        clipboard_copy: false,
        compression: CompressionMode::Simple,
        smart_limit: None,
        split_tokens: None,
        verbose: false,
        is_remote: false,
        output_format: OutputFormat::Xml,
        max_file_size: None,
        max_depth: None,
        prompt: None,
        redact: false,
        diff: None,
        max_tokens: None,
        graph: false,
        compression_level: abyss::config::CompressionLevel::None,
        bundle: None,
        explain_diff: false,
    };

    run(config)?;

    let content = fs::read_to_string(output_path)?;
    assert!(!content.contains("// comment"));

    Ok(())
}

#[test]
fn test_prompt_integration() -> anyhow::Result<()> {
    let temp_dir = TempDir::new()?;
    let root = temp_dir.path();
    let file_path = root.join("test.txt");
    fs::write(&file_path, "content")?;

    let output_path = root.join("prompt.md");
    let config = AbyssConfig {
        path: root.to_path_buf(),
        output: output_path.clone(),
        ignore_patterns: vec![],
        include_patterns: vec![],
        no_tokens: true,
        clipboard_copy: false,
        compression: CompressionMode::None,
        smart_limit: None,
        split_tokens: None,
        verbose: false,
        is_remote: false,
        output_format: OutputFormat::Markdown,
        max_file_size: None,
        max_depth: None,
        prompt: Some("Analyze this code for bugs.".to_string()),
        redact: false,
        diff: None,
        max_tokens: None,
        graph: false,
        compression_level: abyss::config::CompressionLevel::None,
        bundle: None,
        explain_diff: false,
    };

    run(config)?;

    let content = fs::read_to_string(output_path)?;
    assert!(content.contains("> **Instruction**"));
    assert!(content.contains("> Analyze this code for bugs."));

    Ok(())
}
