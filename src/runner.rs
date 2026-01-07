use crate::config::{AbyssConfig, CompressionMode};
// Re-export ScanEvent from core for backward compatibility
pub use crate::core::ScanEvent;

use crate::utils::ast::compress_ast;
use crate::utils::clipboard::copy_to_clipboard;
use crate::utils::compression::compress_content;
use crate::utils::concepts::extract_concepts;
use crate::utils::git_stats::get_git_stats;
use crate::utils::tokens::count_tokens;
use anyhow::Result;
use crossbeam_channel::Sender;
use rayon::prelude::*;
use std::collections::HashMap;
use std::fs as std_fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicUsize, Ordering};

fn get_modified_time(path: &Path) -> Option<u64> {
    std_fs::metadata(path)
        .ok()
        .and_then(|m| m.modified().ok())
        .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|d| d.as_secs())
}

/// Main entry point for the Abyss scanner in CLI mode.
///
/// This function starts the scanning process in a background thread and consumes events
/// on the main thread to print progress if `verbose` is enabled. It handles the efficient
/// traversal of directories, semantic ranking, compression, and output generation.
pub fn run(config: AbyssConfig) -> Result<()> {
    use indicatif::{ProgressBar, ProgressStyle};

    let (tx, rx) = crossbeam_channel::unbounded();

    // Spawn thread to run scan
    let config_clone = config.clone();
    std::thread::spawn(move || {
        if let Err(e) = run_scan(config_clone, Some(tx.clone())) {
            let _ = tx.send(ScanEvent::Error(e.to_string()));
        }
    });

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.cyan} {msg}")
            .unwrap()
            .tick_chars("⠋⠙⠹⠸⠼⠴⠦⠧⠇⠏"),
    );
    pb.enable_steady_tick(std::time::Duration::from_millis(80));

    let mut total_files = 0;
    let mut processed = 0;

    for event in rx {
        match event {
            ScanEvent::StartScanning => {
                pb.set_message("Scanning...");
            }
            ScanEvent::FilesFound(n) => {
                total_files = n;
                pb.set_message(format!("Found {} files", n));
            }
            ScanEvent::FileProcessed(_) => {
                processed += 1;
                if config.verbose {
                    pb.set_message(format!("[{}/{}] Processing...", processed, total_files));
                }
            }
            ScanEvent::TokenCountUpdate(t) => {
                pb.set_message(format!("{} tokens", t));
            }
            ScanEvent::Complete(msg) => {
                pb.finish_with_message(format!("Done: {}", msg));
            }
            ScanEvent::Error(e) => {
                pb.finish_and_clear();
                eprintln!("Error: {}", e);
            }
        }
    }

    Ok(())
}

struct OutputState {
    file: File,
    path: PathBuf,
    current_tokens: usize,
    chunk_index: usize,
    split_limit: Option<usize>,
    created_files: Vec<PathBuf>,
    prompt: Option<String>,
    formatter: Box<dyn crate::format::Formatter>,
    format_type: crate::config::OutputFormat,
    bundle_files: Option<Vec<(PathBuf, String)>>,
}

impl OutputState {
    fn new(
        config: &AbyssConfig,
        start_token_count: Option<usize>,
        graph: Option<&str>,
        overview: Option<&crate::format::RepoOverview>,
    ) -> Result<Self> {
        let path = config.output.clone();
        let mut file = File::create(&path)?;
        let mut formatter = crate::format::create_formatter(config.output_format);

        use crate::format::HeaderContext;
        formatter.write_header(
            &mut file,
            HeaderContext {
                token_count: start_token_count,
                prompt: &config.prompt,
                graph,
                overview,
            },
        )?;

        // Initialize bundle collection if bundle path is set
        let bundle_files = if config.bundle.is_some() {
            Some(Vec::new())
        } else {
            None
        };

        Ok(Self {
            file,
            path,
            current_tokens: 0,
            chunk_index: 0,
            split_limit: config.split_tokens,
            created_files: vec![config.output.clone()],
            prompt: config.prompt.clone(),
            formatter,
            format_type: config.output_format,
            bundle_files,
        })
    }

    fn check_rotate(&mut self, next_tokens: usize, base_path: &std::path::Path) -> Result<()> {
        let limit = match self.split_limit {
            Some(l) => l,
            None => return Ok(()),
        };

        if self.current_tokens + next_tokens <= limit || self.current_tokens == 0 {
            return Ok(());
        }

        // Close current
        self.formatter.write_footer(&mut self.file, &[])?;

        // Open next
        self.chunk_index += 1;
        let base_name = base_path.file_stem().unwrap_or_default().to_string_lossy();
        let extension = base_path.extension().unwrap_or_default().to_string_lossy();
        let ext_str = if extension.is_empty() {
            String::new()
        } else {
            format!(".{}", extension)
        };

        let part_path = base_path.with_file_name(format!(
            "{}-part-{}{}",
            base_name,
            self.chunk_index + 1,
            ext_str
        ));

        let mut file = File::create(&part_path)?;

        // Create new formatter instance for new file (resets state like first_file for JSON)
        self.formatter = crate::format::create_formatter(self.format_type);

        use crate::format::HeaderContext;
        self.formatter.write_header(
            &mut file,
            HeaderContext {
                token_count: None,
                prompt: &self.prompt,
                graph: None,
                overview: None,
            },
        )?;

        self.file = file;
        self.path = part_path.clone();
        self.created_files.push(part_path);
        self.current_tokens = 0;

        Ok(())
    }

    fn write(
        &mut self,
        path: &std::path::Path,
        content: &str,
        summary: Option<&str>,
        repo_root: &std::path::Path,
        tokens: usize,
    ) -> Result<()> {
        self.formatter
            .write_file(&mut self.file, path, content, summary, repo_root)?;
        self.current_tokens += tokens;

        if let Some(files) = &mut self.bundle_files {
            files.push((path.to_path_buf(), content.to_string()));
        }

        Ok(())
    }

    fn finish(&mut self, dropped: &[PathBuf]) -> Result<()> {
        self.formatter.write_footer(&mut self.file, dropped)?;
        Ok(())
    }

    fn write_directory_structure(
        &mut self,
        paths: &[PathBuf],
        root: &std::path::Path,
    ) -> Result<()> {
        self.formatter
            .write_directory_structure(&mut self.file, paths, root)
    }
}

/// Discovers and sorts files according to configuration
///
/// This is a re-export for backward compatibility. The actual implementation
/// is in `core::scanner::discover_files`.
pub fn discover_files(
    config: &AbyssConfig,
    tx: Option<Sender<ScanEvent>>,
) -> Result<(Vec<(PathBuf, PathBuf)>, Vec<PathBuf>)> {
    crate::core::scanner::discover_files(config, tx)
}

pub fn run_scan(config: AbyssConfig, sender: Option<Sender<ScanEvent>>) -> Result<()> {
    let (files, dropped) = discover_files(&config, sender.clone())?;
    process_files(files, dropped, config, sender)
}

/// Processes the selected files and generates output
pub fn process_files(
    mut files: Vec<(PathBuf, PathBuf)>,
    mut dropped_files: Vec<PathBuf>,
    config: AbyssConfig,
    tx: Option<Sender<ScanEvent>>,
) -> Result<()> {
    let notify = |e: ScanEvent| {
        if let Some(ref tx) = tx {
            let _ = tx.send(e);
        }
    };

    // 1. Intelligence Phase (Graph, Scores, Ranking)
    let mut git_stats_map = HashMap::new();
    let roots: std::collections::HashSet<_> = files.iter().map(|(_, root)| root.clone()).collect();
    for root in roots {
        let stats = get_git_stats(&root);
        git_stats_map.insert(root, stats);
    }

    let mut graph = crate::utils::graph::DependencyGraph::new();
    let mut scores: HashMap<PathBuf, crate::utils::rank::FileScore> = HashMap::new();

    // Pre-calculate Heuristic & Churn
    for (path, root) in &files {
        let mut score = crate::utils::rank::FileScore {
            heuristic: crate::utils::rank::heuristic_score(path),
            ..Default::default()
        };

        if let Some(stats) = git_stats_map.get(root)
            && let Some(s) = stats.get(path)
        {
            score.churn = std::cmp::min(s.churn_score * 5, 200) as i32;
        }
        scores.insert(path.clone(), score);
    }

    // Scan content for Entropy & Dependencies (Parallel)
    struct FileAnalysis {
        path: PathBuf,
        root: PathBuf,
        entropy: f64,
        tokens: usize,
        imports: Vec<String>,
        #[allow(dead_code)]
        extension: String,
        #[allow(dead_code)]
        content: String,
    }

    let no_tokens = config.no_tokens;
    let analyses: Vec<FileAnalysis> = files
        .par_iter()
        .filter_map(|(path, root)| {
            let content = std_fs::read_to_string(path).ok()?;
            let extension = path
                .extension()
                .and_then(|s| s.to_str())
                .unwrap_or("")
                .to_string();

            let entropy = crate::utils::rank::calculate_entropy(&content);
            let tokens = if !no_tokens {
                crate::utils::tokens::estimate_tokens(&content)
            } else {
                content.len() / 4
            };
            let imports = crate::utils::dependencies::extract_imports(&content, &extension);

            Some(FileAnalysis {
                path: path.clone(),
                root: root.clone(),
                entropy,
                tokens,
                imports,
                extension,
                content,
            })
        })
        .collect();

    // Build graph atomically
    for analysis in &analyses {
        graph.add_node(analysis.path.clone());

        if let Some(s) = scores.get_mut(&analysis.path) {
            s.entropy = analysis.entropy;
            s.tokens = analysis.tokens;
        }

        for import in &analysis.imports {
            if let Some(resolved) =
                crate::utils::dependencies::resolve_import(import, &analysis.path, &analysis.root)
            {
                graph.add_edge(analysis.path.clone(), resolved);
            }
        }
    }

    // PageRank
    let page_ranks = graph.calculate_pagerank();
    for (path, score) in &page_ranks {
        if let Some(s) = scores.get_mut(path) {
            s.pagerank = *score;
        }
    }

    // Sort & Knapsack
    let all_paths: Vec<PathBuf> = files.iter().map(|(p, _)| p.clone()).collect();
    let sorted_paths = crate::utils::rank::sort_files(&all_paths, &scores, &graph);

    // Re-order and Filter
    let mut final_files = Vec::new();
    let mut current_total_tokens = 0;

    let max_tokens = config.max_tokens.unwrap_or(usize::MAX);

    // Create candidate list (sorted by topological order, but we want to filter by priority if budget constrained)
    // Actually, knapsack logic was to prioritize high score items.
    // If max_tokens is set, we sort by score first to select candidates, then output in topo order.

    let mut selected_set = std::collections::HashSet::new();

    if config.max_tokens.is_some() {
        let mut candidates: Vec<&PathBuf> = all_paths.iter().collect();
        candidates.sort_by(|a, b| {
            let score_a = scores.get(*a).map(|s| s.final_score()).unwrap_or(0.0);
            let score_b = scores.get(*b).map(|s| s.final_score()).unwrap_or(0.0);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        for path in candidates {
            let t = scores.get(path).map(|s| s.tokens).unwrap_or(0);
            if current_total_tokens + t <= max_tokens {
                selected_set.insert(path.clone());
                current_total_tokens += t;
            } else {
                dropped_files.push(path.clone());
            }
        }
    } else {
        selected_set = all_paths.iter().cloned().collect();
    }

    for path in sorted_paths {
        if selected_set.contains(&path)
            && let Some((_, root)) = files.iter().find(|(p, _)| p == &path)
        {
            final_files.push((path, root.clone()));
        }
    }

    files = final_files;
    notify(ScanEvent::FilesFound(files.len()));

    // 2. Setup Streaming and Output
    type ScanResult = Option<(PathBuf, String, Option<String>, usize)>;
    let (data_tx, data_rx) = crossbeam_channel::unbounded::<(usize, ScanResult)>();
    let total_tokens_atomic = AtomicUsize::new(0);

    let cache = std::sync::Arc::new(crate::utils::cache::Cache::load());
    let config_sig = format!("{:?}", config);

    // 3. Parallel Process Content (Again, for output generation with caching)
    let files_clone = files.clone();
    let config_ref = &config;
    let notify_ref = &notify;
    let total_tokens_ref = &total_tokens_atomic;
    let cache_ref = &cache;
    let config_sig_ref = &config_sig;

    std::thread::scope(|s| {
        s.spawn(move || {
            files_clone
                .par_iter()
                .enumerate()
                .for_each(|(index, (path, _root))| {
                    // Get content from analysis if available?
                    // Analyses vector is local to previous scope.
                    // We re-read or cache.
                    // Original logic re-read and processed concepts/summary.

                    // ... (Standard Content Processing Logic)
                    let extension = path
                        .extension()
                        .and_then(|s| s.to_str())
                        .unwrap_or("")
                        .to_lowercase();
                    let mut content;

                    if extension == "pdf" {
                        match crate::utils::pdf::extract_text(path) {
                            Ok(text) => content = text,
                            Err(e) => {
                                eprintln!("Failed to extract PDF: {}", e);
                                let _ = data_tx.send((index, None));
                                return;
                            }
                        }
                    } else if ["png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "tiff"]
                        .contains(&extension.as_str())
                    {
                        match crate::utils::image::describe_image(path) {
                            Ok(desc) => content = desc,
                            Err(e) => {
                                eprintln!("Failed to describe image: {}", e);
                                let _ = data_tx.send((index, None));
                                return;
                            }
                        }
                    } else {
                        // Standard Text/Binary File
                        match std_fs::read(path) {
                            Ok(bytes) => {
                                if crate::utils::binary::is_binary(&bytes) {
                                    let _ = data_tx.send((index, None));
                                    return;
                                }
                                content = String::from_utf8_lossy(&bytes).to_string();
                            }
                            Err(_) => {
                                let _ = data_tx.send((index, None));
                                return;
                            }
                        }
                    }

                    let modified_time = get_modified_time(path).unwrap_or(0);
                    let mut cached_entry = None;

                    if modified_time > 0 {
                        let hash =
                            crate::utils::cache::Cache::compute_hash(&content, config_sig_ref);
                        #[allow(clippy::collapsible_if)]
                        if let Some(entry) = cache_ref.get(&path.to_string_lossy()) {
                            if entry.modified == modified_time && entry.hash == hash {
                                cached_entry = Some(entry.tokens);
                            }
                        }
                    }

                    if config_ref.redact {
                        content = crate::utils::privacy::redact_content(&content);
                    }

                    let extension_str = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                    let concepts = extract_concepts(&content, extension_str);

                    match config_ref.compression {
                        CompressionMode::Simple => {
                            content = compress_content(&content);
                        }
                        CompressionMode::Smart => {
                            content = compress_ast(&content, extension_str);
                        }
                        CompressionMode::None => {}
                    }

                    if !config_ref
                        .compression_level
                        .to_compression_mode()
                        .eq(&CompressionMode::None)
                    {
                        content = crate::utils::compression::compress_by_level(
                            &content,
                            config_ref.compression_level,
                            extension_str,
                        );
                    }

                    if !concepts.is_empty() {
                        let concept_str = concepts.join(", ");
                        let (prefix, suffix) = match extension_str {
                            "py" | "rb" | "sh" | "yaml" | "toml" | "dockerfile" | "makefile" => {
                                ("#", "")
                            }
                            "html" | "xml" | "md" => ("<!--", " -->"),
                            _ => ("//", ""),
                        };
                        content = format!(
                            "{} Concepts: {}{}\n{}",
                            prefix, concept_str, suffix, content
                        );
                    }

                    let count = if !config_ref.no_tokens {
                        if let Some(tokens) = cached_entry {
                            tokens
                        } else if let Ok(c) = count_tokens(&content) {
                            if modified_time > 0 {
                                let hash = crate::utils::cache::Cache::compute_hash(
                                    &String::from_utf8_lossy(
                                        &std_fs::read(path).unwrap_or_default(),
                                    ),
                                    config_sig_ref,
                                );
                                cache_ref.update(
                                    path.to_string_lossy().to_string(),
                                    crate::utils::cache::CacheEntry {
                                        hash,
                                        tokens: c,
                                        modified: modified_time,
                                    },
                                );
                            }
                            c
                        } else {
                            0
                        }
                    } else {
                        0
                    };

                    if count > 0 || !config_ref.no_tokens {
                        let current = total_tokens_ref.fetch_add(count, Ordering::Relaxed) + count;
                        notify_ref(ScanEvent::TokenCountUpdate(current));
                    }

                    let summary = crate::utils::summary::summarize_content(&content, extension_str);

                    notify_ref(ScanEvent::FileProcessed(path.clone()));
                    let _ = data_tx.send((index, Some((path.clone(), content, summary, count))));
                });
            drop(data_tx);
        });

        // 4. Consumer
        let mermaid_graph = if config_ref.graph {
            // Only generate graph for the first root or merged?
            // Merged graph from earlier intelligence phase is better, but here we just re-generate purely for display
            // Let's use the files we have.
            let paths_only: Vec<PathBuf> = files.iter().map(|(p, _)| p.clone()).collect();
            // Root path is tricky. Use first root or config path.
            let display_root = &config.path;
            let graph =
                crate::utils::dependencies::build_dependency_graph(&paths_only, display_root);
            Some(crate::format::mermaid::generate_diagram(
                &graph,
                display_root,
            ))
        } else {
            None
        };

        // 5. Generate Executive Summary
        let mut key_files = Vec::new();
        let mut purpose = None;

        for (path, _) in files.iter().take(5) {
            let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
            if let Ok(content) = std_fs::read_to_string(path) {
                if let Some(s) = crate::utils::summary::summarize_content(&content, extension) {
                    key_files.push((path.clone(), s));
                }

                let filename = path
                    .file_name()
                    .and_then(|f| f.to_str())
                    .unwrap_or("")
                    .to_lowercase();
                if (filename == "readme.md" || filename == "readme.txt") && purpose.is_none() {
                    purpose = crate::utils::summary::extract_readme_purpose(&content);
                }
            }
        }

        let overview = if !key_files.is_empty() || purpose.is_some() {
            Some(crate::format::RepoOverview {
                purpose,
                key_files,
                changes: None,
            })
        } else {
            None
        };

        let mut out_state =
            match OutputState::new(&config, None, mermaid_graph.as_deref(), overview.as_ref()) {
                Ok(s) => s,
                Err(e) => {
                    notify(ScanEvent::Error(format!("Failed to create output: {}", e)));
                    return;
                }
            };

        // Write dir structure
        let paths_only: Vec<PathBuf> = files.iter().map(|(p, _)| p.clone()).collect();
        // Use config.path as root for structure display if single repo, else common ancestor?
        // Simpler: Just use config.path.
        if let Err(e) = out_state.write_directory_structure(&paths_only, &config.path) {
            notify(ScanEvent::Error(e.to_string()));
            return;
        }

        let mut buffer: HashMap<usize, ScanResult> = HashMap::new();
        let mut next_idx = 0;
        let total_files = files.len();

        while next_idx < total_files {
            while buffer.contains_key(&next_idx) {
                if let Some(Some((path, content, summary, tokens))) = buffer.remove(&next_idx) {
                    if let Err(e) = out_state.check_rotate(tokens, &config.output) {
                        notify(ScanEvent::Error(e.to_string()));
                    }

                    let root = files[next_idx].1.clone();

                    let summary_ref = summary.as_deref();
                    if let Err(e) = out_state.write(&path, &content, summary_ref, &root, tokens) {
                        notify(ScanEvent::Error(e.to_string()));
                    }
                } else {
                    buffer.remove(&next_idx);
                }
                next_idx += 1;
            }
            if next_idx >= total_files {
                break;
            }
            match data_rx.recv() {
                Ok((idx, data)) => {
                    buffer.insert(idx, data);
                }
                Err(_) => {
                    break;
                }
            }
        }
        let _ = out_state.finish(&dropped_files);
        let _ = cache.save();

        // Save Bundle
        if let Some(bundle_path) = &config.bundle
            && let Some(files) = out_state.bundle_files
        {
            let compression_str = config.compression_level.to_string();
            let query = config.prompt.clone();
            // Add graph

            let bundle = crate::utils::bundle::Bundle::new(
                files,
                mermaid_graph.clone(),
                &compression_str,
                query,
            );

            let result = if bundle_path.to_string_lossy().ends_with(".json") {
                bundle.save_json(bundle_path)
            } else {
                bundle.save_tar_gz(bundle_path)
            };

            match result {
                Ok(_) => notify(ScanEvent::Complete(format!(
                    "Bundle saved to {:?}",
                    bundle_path
                ))),
                Err(e) => notify(ScanEvent::Error(format!("Failed to save bundle: {}", e))),
            }
        }

        if config.clipboard_copy {
            let mut full_text = String::new();
            for p in &out_state.created_files {
                if let Ok(s) = std_fs::read_to_string(p) {
                    full_text.push_str(&s);
                    full_text.push('\n');
                }
            }
            let _ = copy_to_clipboard(&full_text);
        }

        notify(ScanEvent::Complete(format!(
            "Written to {:?}",
            out_state.created_files
        )));
    });

    Ok(())
}
