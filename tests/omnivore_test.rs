use abyss::config::{AbyssConfig, CompressionMode, OutputFormat};
use abyss::run;
use image::{ImageBuffer, Rgb};
use std::fs;
use tempfile::tempdir;

#[test]
fn test_image_ingestion() {
    let dir = tempdir().unwrap();
    let root = dir.path();
    let output_file = root.join("output.xml");

    // 1. Create a valid PNG
    let img_path = root.join("test_logo.png");
    let mut img = ImageBuffer::<Rgb<u8>, Vec<u8>>::new(100, 50);
    // Fill with red
    for pixel in img.pixels_mut() {
        *pixel = Rgb([255, 0, 0]);
    }
    img.save(&img_path).unwrap();

    // 2. Configure Abyss
    let config = AbyssConfig {
        path: root.to_path_buf(),
        output: output_file.clone(),
        ignore_patterns: vec![],
        include_patterns: vec![],
        no_tokens: true, // Speed up
        clipboard_copy: false,
        compression: CompressionMode::None,
        split_tokens: None,
        verbose: true,
        is_remote: false,
        smart_limit: None,
        output_format: OutputFormat::Xml,
        max_file_size: None,
        max_depth: None,
        prompt: None,
        max_tokens: None,
        redact: false,
        diff: None,
        graph: false,
        compression_level: abyss::config::CompressionLevel::None,
        bundle: None,
        explain_diff: false,
    };

    // 3. Run
    run(config).unwrap();

    // 4. Verify Output
    let content = fs::read_to_string(output_file).unwrap();
    println!("Output content:\n{content}");

    // Should contain the image description
    assert!(content.contains("test_logo.png"));
    assert!(content.contains("[Image: test_logo.png | 100x50 | PNG]"));
}
