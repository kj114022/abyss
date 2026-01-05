use crate::config::{AbyssConfig, CompressionMode};
use crate::fs::walk_directory;
use crate::utils::ast::compress_ast;
use crate::utils::clipboard::copy_to_clipboard;
use crate::utils::compression::compress_content;
use crate::utils::concepts::extract_concepts;
use crate::utils::git_stats::{get_diff_files, get_git_stats};
use crate::utils::tokens::{count_tokens, estimate_cost};
use anyhow::{Context, Result};
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

#[derive(Debug, Clone)]
pub enum ScanEvent {
    StartScanning,
    FilesFound(usize),
    FileProcessed(PathBuf),
    TokenCountUpdate(usize),
    Complete(String), // Returns summary or output path
    Error(String),
}

/// Main entry point for the Abyss scanner in CLI mode.
///
/// This function starts the scanning process in a background thread and consumes events
/// on the main thread to print progress if `verbose` is enabled. It handles the efficient
/// traversal of directories, semantic ranking, compression, and output generation.
pub fn run(config: AbyssConfig) -> Result<()> {
    let (tx, rx) = crossbeam_channel::unbounded();

    // Spawn thread to run scan
    let config_clone = config.clone();
    std::thread::spawn(move || {
        if let Err(e) = run_scan(config_clone, Some(tx.clone())) {
            let _ = tx.send(ScanEvent::Error(e.to_string()));
        }
    });

    let mut total_tokens = 0;

    for event in rx {
        match event {
            ScanEvent::StartScanning => {
                if config.verbose {
                    println!("Scanning started...")
                }
            }
            ScanEvent::FilesFound(n) => {
                if config.verbose {
                    println!("Found {} files.", n)
                }
            }
            ScanEvent::FileProcessed(p) => {
                if config.verbose {
                    println!("Processed: {:?}", p)
                }
            }
            ScanEvent::TokenCountUpdate(t) => {
                total_tokens = t;
                if config.verbose {
                    println!("Total tokens: {}", t)
                }
            }
            ScanEvent::Complete(msg) => {
                if config.verbose {
                    println!("{}", msg)
                }

                // Print Cost Estimate
                if !config.no_tokens && total_tokens > 0 {
                    println!("\nEstimated Cost (Input):");
                    for estimate in estimate_cost(total_tokens) {
                        println!("  - {}: ${:.4}", estimate.model_name, estimate.cost_usd);
                    }
                }
            }
            ScanEvent::Error(e) => eprintln!("Error: {}", e),
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
pub fn discover_files(
    config: &AbyssConfig,
    tx: Option<Sender<ScanEvent>>,
) -> Result<(Vec<PathBuf>, PathBuf, Vec<PathBuf>)> {
    let notify = |e: ScanEvent| {
        if let Some(ref tx) = tx {
            let _ = tx.send(e);
        }
    };

    notify(ScanEvent::StartScanning);

    let root_path = config
        .path
        .canonicalize()
        .with_context(|| format!("Failed to find directory: {:?}", config.path))?;

    // 1. Walk directory
    let mut paths = walk_directory(&root_path, &config.ignore_patterns)?;
    notify(ScanEvent::FilesFound(paths.len()));

    // 2. Filter by Diff
    #[allow(clippy::collapsible_if)]
    if let Some(ref target) = config.diff {
        if let Some(diff_files) = get_diff_files(&root_path, target) {
            let diff_set: std::collections::HashSet<PathBuf> =
                diff_files.into_iter().map(PathBuf::from).collect();
            paths.retain(|p| {
                if let Ok(relative) = p.strip_prefix(&root_path) {
                    diff_set.contains(relative)
                } else {
                    false
                }
            });
            if config.verbose {
                println!("Filtered by diff: {} remain", paths.len());
            }
        }
    }

    // 3. Git Stats
    let git_stats = get_git_stats(&root_path);

    // 4. Build Intelligence (Graph + Scores)
    // Compute semantic rankings for all files to guide topological sorting.

    let mut graph = crate::utils::graph::DependencyGraph::new();
    let mut scores: HashMap<PathBuf, crate::utils::rank::FileScore> = HashMap::new();

    // Pre-calculate Heuristic & Churn
    for path in &paths {
        let mut score = crate::utils::rank::FileScore {
            heuristic: crate::utils::rank::heuristic_score(path),
            ..Default::default()
        };

        if let Some(s) = git_stats.get(path) {
            score.churn = std::cmp::min(s.churn_score * 5, 200) as i32;
        }
        scores.insert(path.clone(), score);
    }

    // Scan content for Entropy & Dependencies
    // Note: Sequential execution chosen to maintain Graph integrity.

    for path in &paths {
        graph.add_node(path.clone());
        if let Ok(content) = std_fs::read_to_string(path) {
            // Dependencies
            let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");

            // Entropy & Tokens & Summary
            if let Some(s) = scores.get_mut(path) {
                s.entropy = crate::utils::rank::calculate_entropy(&content);
                if !config.no_tokens {
                    s.tokens = crate::utils::tokens::count_tokens(&content).unwrap_or(0);
                } else {
                    // Heuristic: ~4 chars per token
                    s.tokens = content.len() / 4;
                }
            }

            // Extract Imports
            let imports = crate::utils::dependencies::extract_imports(&content, extension);

            for import in imports {
                if let Some(dep_path) =
                    crate::utils::dependencies::resolve_import(&import, path, &root_path)
                {
                    graph.add_edge(path.clone(), dep_path);
                }
            }
        }
    }

    // Calculate PageRank
    let pagerank = graph.calculate_pagerank();
    for (path, pr) in pagerank {
        if let Some(s) = scores.get_mut(&path) {
            s.pagerank = pr;
        }
    }

    // 5. Sort Topologically with Ranking Tie-Breaker
    // Comparator: Higher scores appear earlier in the stream for independent nodes.

    paths = graph.sort_topologically(|a, b| {
        let score_a = scores.get(a).map(|s| s.final_score()).unwrap_or(0.0);
        let score_b = scores.get(b).map(|s| s.final_score()).unwrap_or(0.0);

        score_b
            .partial_cmp(&score_a)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| a.cmp(b))
    });

    // 6. Smart Context Filtering (Knapsack)
    let mut selected_paths = Vec::new();
    let mut dropped_paths = Vec::new();
    let mut _current_tokens = 0;

    // Knapsack Process:
    // Files are added in topological order (with high-value items prioritized within independent groups).
    // Dependents typically appear later, but high PageRank ensures central files are prioritized where possible.
    // But dependent chains constrain order.
    // `mod.rs` (depends on utils) -> `mod.rs` MUST come after `utils`.
    // If `utils` fills the budget -> `mod.rs` is dropped.
    // This is bad. `mod.rs` provides the map.
    // Strategy: Maybe we should calculate "Cumulative Tokens" and mark files to keep?
    // But we can't break topological order for output (Rust compiler order).
    // So we must output in Topo Order.
    // But we can SELECT based on Score/Knapsack, effectively "Skipping" lines in the output list.
    // YES.
    // We shouldn't truncate the LIST. We should FILTER the LIST.
    // Filter condition: Is this file "worth it"?
    // Global Knapsack:
    // 1. Gather all (Path, Score, Tokens).
    // 2. Select Top-N items that fit in MaxTokens.
    // 3. Output selected items in Topological Order.

    if let Some(max_tokens) = config.max_tokens {
        // Collect candidates
        let mut candidates: Vec<&PathBuf> = paths.iter().collect();
        // Sort by Score Descending (Ignore topology for selection)
        candidates.sort_by(|a, b| {
            let score_a = scores.get(*a).map(|s| s.final_score()).unwrap_or(0.0);
            let score_b = scores.get(*b).map(|s| s.final_score()).unwrap_or(0.0);
            score_b
                .partial_cmp(&score_a)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        let mut keep_set = std::collections::HashSet::new();
        let mut used_tokens = 0;

        for path in candidates {
            let tokens = scores.get(path).map(|s| s.tokens).unwrap_or(0);
            if used_tokens + tokens <= max_tokens {
                keep_set.insert(path.clone());
                used_tokens += tokens;
            } else {
                // If file is critical (heuristic > 900 e.g. README), maybe squeeze it in?
                // For now strict limit.
            }
        }

        for path in paths {
            if keep_set.contains(&path) {
                selected_paths.push(path);
            } else {
                dropped_paths.push(path);
            }
        }
        _current_tokens = used_tokens;
    } else {
        selected_paths = paths;
    }

    notify(ScanEvent::FilesFound(selected_paths.len()));
    Ok((selected_paths, root_path, dropped_paths))
}

/// Processes the selected files and generates output
pub fn process_files(
    paths: Vec<PathBuf>,
    dropped_files: Vec<PathBuf>,
    root_path: PathBuf,
    config: AbyssConfig,
    tx: Option<Sender<ScanEvent>>,
) -> Result<()> {
    let notify = |e: ScanEvent| {
        if let Some(ref tx) = tx {
            let _ = tx.send(e);
        }
    };

    // 1. Setup Streaming
    type ScanResult = Option<(PathBuf, String, Option<String>, usize)>;
    let (data_tx, data_rx) = crossbeam_channel::unbounded::<(usize, ScanResult)>();
    let total_tokens_atomic = AtomicUsize::new(0);

    // 2. Cache Setup
    let cache = std::sync::Arc::new(crate::utils::cache::Cache::load());
    let config_sig = format!("{:?}", config);

    // 3. Parallel Process
    let paths_clone = paths.clone();
    let config_ref = &config;
    let notify_ref = &notify;
    let total_tokens_ref = &total_tokens_atomic;
    let cache_ref = &cache;
    let config_sig_ref = &config_sig;

    std::thread::scope(|s| {
        s.spawn(move || {
            paths_clone
                .par_iter()
                .enumerate()
                .for_each(|(index, path)| {
                        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("").to_lowercase();
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
                        } else if ["png", "jpg", "jpeg", "gif", "webp", "bmp", "ico", "tiff"].contains(&extension.as_str()) {
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

                        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                        let concepts = extract_concepts(&content, extension);

                        match config_ref.compression {
                            CompressionMode::Simple => {
                                content = compress_content(&content);
                            }
                            CompressionMode::Smart => {
                                content = compress_ast(&content, extension);
                            }
                            CompressionMode::None => {}
                        }

                        if !concepts.is_empty() {
                            let concept_str = concepts.join(", ");
                            let (prefix, suffix) = match extension {
                                "py" | "rb" | "sh" | "yaml" | "toml" | "dockerfile"
                                | "makefile" => ("#", ""),
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
                            let current =
                                total_tokens_ref.fetch_add(count, Ordering::Relaxed) + count;
                            notify_ref(ScanEvent::TokenCountUpdate(current));
                        }

                        let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                        let summary = crate::utils::summary::summarize_content(&content, extension);

                        notify_ref(ScanEvent::FileProcessed(path.clone()));
                        let _ =
                            data_tx.send((index, Some((path.clone(), content, summary, count))));
                });
            drop(data_tx);
        });

        // 4. Consumer
        let mermaid_graph = if config_ref.graph {
            let graph = crate::utils::dependencies::build_dependency_graph(&paths, &root_path);
            Some(crate::format::mermaid::generate_diagram(&graph, &root_path))
        } else {
            None
        };

        // 5. Generate Executive Summary
        let mut key_files = Vec::new();
        let mut purpose = None;

        for path in paths.iter().take(5) {
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

        if let Err(e) = out_state.write_directory_structure(&paths, &root_path) {
            notify(ScanEvent::Error(e.to_string()));
            return;
        }

        let mut buffer: HashMap<usize, ScanResult> = HashMap::new();
        let mut next_idx = 0;
        let total_files = paths.len();

        while next_idx < total_files {
            while buffer.contains_key(&next_idx) {
                if let Some(Some((path, content, summary, tokens))) = buffer.remove(&next_idx) {
                    if let Err(e) = out_state.check_rotate(tokens, &config.output) {
                        notify(ScanEvent::Error(e.to_string()));
                    }
                    // Pass summary as deref for Option<&str>
                    let summary_ref = summary.as_deref();
                    if let Err(e) =
                        out_state.write(&path, &content, summary_ref, &root_path, tokens)
                    {
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

/// Main entry point for the Abyss scanner (legacy wrapper)
pub fn run_scan(config: AbyssConfig, tx: Option<Sender<ScanEvent>>) -> Result<()> {
    let (paths, root_path, dropped) = discover_files(&config, tx.clone())?;
    process_files(paths, dropped, root_path, config, tx)
}
