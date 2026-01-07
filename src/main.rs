use abyss::config::OutputFormat;
use abyss::git::{clone_repo, is_remote_url};
use abyss::{AbyssConfig, CompressionMode, run};
use anyhow::Result;
use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliOutputFormat {
    Xml,
    Json,
    Md,
    Plain,
}

impl From<CliOutputFormat> for OutputFormat {
    fn from(f: CliOutputFormat) -> Self {
        match f {
            CliOutputFormat::Xml => OutputFormat::Xml,
            CliOutputFormat::Json => OutputFormat::Json,
            CliOutputFormat::Md => OutputFormat::Markdown,
            CliOutputFormat::Plain => OutputFormat::Plain,
        }
    }
}

#[derive(Parser, Debug)]
#[command(
    author,
    version,
    about = "abyss - The LLM Context Compiler\n\nTransform codebases into semantically-ordered, token-optimized context for LLMs.\n\nFeatures:\n  • Dependency-aware ordering (topological sort)\n  • Architectural centrality (PageRank)\n  • Git intelligence (churn analysis)\n  • AST-aware compression (preserve interfaces)\n  • Token budget optimization (knapsack algorithm)",
    long_about = None
)]
struct Args {
    /// Directory or Remote URL to scan
    path: Option<String>,

    /// Output file path
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Output format
    #[arg(short, long, value_enum)]
    format: Option<CliOutputFormat>,

    /// Add ignore pattern (glob)
    #[arg(long)]
    ignore: Vec<String>,

    /// Add include pattern (glob) - only include matching files
    #[arg(long)]
    include: Vec<String>,

    /// Maximum file size in bytes (skip larger files)
    #[arg(long)]
    max_size: Option<usize>,

    /// Maximum directory depth to traverse
    #[arg(long)]
    max_depth: Option<usize>,

    /// Disable token counting
    #[arg(long)]
    no_tokens: bool,

    /// Copy output to clipboard
    /// Copy output to clipboard
    #[arg(short, long)]
    copy: bool,

    /// Redact secrets and PII
    #[arg(long)]
    redact: bool,

    /// Compress output (remove comments/whitespace)
    #[arg(long)]
    compress: bool,

    /// Use AST-aware smart compression
    #[arg(long)]
    smart: bool,

    /// Compression level: none, light, standard, aggressive
    /// - none: full source code
    /// - light: remove comments and extra whitespace
    /// - standard: remove comments, whitespace, and simple boilerplate
    /// - aggressive: replace function bodies with placeholders
    #[arg(long, value_name = "LEVEL")]
    compress_level: Option<String>,

    /// Split output into chunks of N tokens
    #[arg(long)]
    split: Option<usize>,

    /// Launch in TUI mode
    #[arg(long)]
    tui: bool,

    /// Verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Prepend a custom prompt/instruction
    #[arg(long)]
    prompt: Option<String>,

    /// Read prompt from file
    #[arg(long)]
    prompt_file: Option<PathBuf>,

    /// Scan only changed files relative to this Git reference (e.g. "main", "HEAD~1")
    #[arg(long)]
    diff: Option<String>,

    /// Maximum tokens to include in output (e.g. 128000)
    #[arg(long)]
    max_tokens: Option<usize>,

    /// Enable dependency graph generation
    #[arg(long)]
    graph: bool,

    /// Generate shell completions (bash, zsh, fish, powershell)
    #[arg(long, value_name = "SHELL")]
    completions: Option<String>,

    /// GPT-4 preset (128K tokens)
    #[arg(long)]
    gpt: bool,

    /// Claude preset (200K tokens)
    #[arg(long)]
    claude: bool,

    /// Gemini preset (1M tokens)
    #[arg(long)]
    gemini: bool,

    /// Show pre-flight analysis without processing (dry run)
    #[arg(long)]
    dry_run: bool,

    /// Analyze context quality and exit
    #[arg(long)]
    analyze_quality: bool,

    /// Query-driven context: find files relevant to a question
    /// Example: --query "how does authentication work?"
    #[arg(long)]
    query: Option<String>,

    /// Show impact analysis for changed files (use with --diff)
    #[arg(long)]
    show_impact: bool,

    /// Output in Cursor-compatible JSON format
    #[arg(long)]
    cursor: bool,

    /// Context tier: summary, detailed, or full
    /// - summary: signatures only (~10% size)
    /// - detailed: interfaces + key implementations (~30% size)
    /// - full: complete source code (default)
    #[arg(long, value_name = "TIER")]
    tier: Option<String>,

    /// Watch mode: regenerate context on file changes
    #[arg(long)]
    watch: bool,

    /// Export as portable bundle (JSON or .tar.gz based on extension)
    #[arg(long, value_name = "PATH")]
    bundle: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();

    // Handle completions generation early exit
    if let Some(shell) = &args.completions {
        use clap::CommandFactory;
        use clap_complete::{Shell, generate};
        use std::io;

        let shell = match shell.to_lowercase().as_str() {
            "bash" => Shell::Bash,
            "zsh" => Shell::Zsh,
            "fish" => Shell::Fish,
            "powershell" | "ps" => Shell::PowerShell,
            _ => {
                eprintln!(
                    "Unsupported shell: {}. Use: bash, zsh, fish, powershell",
                    shell
                );
                std::process::exit(1);
            }
        };

        let mut cmd = Args::command();
        generate(shell, &mut cmd, "abyss", &mut io::stdout());
        return Ok(());
    }

    // 1. Load from file or default
    let mut config = AbyssConfig::load_from_file().unwrap_or_default();

    // 2. Override with CLI args
    if let Some(p) = args.path {
        config.path = PathBuf::from(p);
    }
    if let Some(o) = args.output {
        config.output = o;
    }
    if let Some(f) = args.format {
        config.output_format = f.into();
    }
    if !args.ignore.is_empty() {
        // CLI ignores ADD to config ignores
        config.ignore_patterns.extend(args.ignore);
    }
    if !args.include.is_empty() {
        config.include_patterns = args.include;
    }
    if let Some(s) = args.max_size {
        config.max_file_size = Some(s);
    }
    if let Some(d) = args.max_depth {
        config.max_depth = Some(d);
    }
    if args.no_tokens {
        config.no_tokens = true;
    }
    if args.copy {
        config.clipboard_copy = true;
    }
    if args.verbose {
        config.verbose = true;
    }
    if args.redact {
        config.redact = true;
    }
    if let Some(s) = args.split {
        config.split_tokens = Some(s);
    }

    // Prompt merging
    if let Some(p) = args.prompt {
        config.prompt = Some(p);
    } else if let Some(path) = args.prompt_file {
        config.prompt = std::fs::read_to_string(path).ok();
    }

    // Diff mode
    if let Some(d) = args.diff {
        config.diff = Some(d);
    }

    if let Some(mt) = args.max_tokens {
        config.max_tokens = Some(mt);
    }

    // Direct model preset flags
    if args.gpt {
        config.max_tokens = Some(128_000);
    } else if args.claude {
        config.max_tokens = Some(200_000);
    } else if args.gemini {
        config.max_tokens = Some(1_000_000);
    }

    // Environment variable fallback
    if config.max_tokens.is_none()
        && let Some(tokens) = std::env::var("ABYSS_MAX_TOKENS")
            .ok()
            .and_then(|v| v.parse().ok())
    {
        config.max_tokens = Some(tokens);
    }

    if args.graph {
        config.graph = true;
    }

    // Compression Logic merge (with backward compatibility)
    // Priority: --compress-level > --smart > --compress
    if let Some(level_str) = &args.compress_level {
        if let Some(level) = abyss::config::CompressionLevel::from_str(level_str) {
            config.compression_level = level;
            config.compression = level.to_compression_mode();
        } else {
            eprintln!("Warning: Invalid compression level '{}', using none", level_str);
        }
    } else if args.smart {
        config.compression = CompressionMode::Smart;
        config.compression_level = abyss::config::CompressionLevel::Aggressive;
    } else if args.compress {
        config.compression = CompressionMode::Simple;
        config.compression_level = abyss::config::CompressionLevel::Light;
    }
    // Else keep config value

    // Handle Cursor format (forces JSON output)
    if args.cursor {
        config.output_format = abyss::config::OutputFormat::Json;
    }

    // Handle Remote URL here in the binary layer
    let path_str = config.path.to_string_lossy().to_string();
    let (_temp_dir, path_buf): (Option<tempfile::TempDir>, PathBuf) = if is_remote_url(&path_str) {
        if config.verbose {
            println!("Detected remote URL. Cloning...");
        }
        let temp_dir = clone_repo(&path_str)?;
        let p = temp_dir.path().to_path_buf();
        if config.verbose {
            println!("Cloned to temporary directory: {:?}", p);
        }
        (Some(temp_dir), p)
    } else {
        let p = PathBuf::from(&path_str);
        (None, p)
    };

    // Update path in final config object
    config.path = path_buf;
    config.is_remote = _temp_dir.is_some();
    // hold temp_dir until end of scope

    // Handle dry-run (pre-flight analysis)
    if args.dry_run {
        use abyss::runner::discover_files;
        
        let (files, _root, _dropped) = discover_files(&config, None)?;
        let analysis = abyss::utils::preflight::analyze(&config, &files);
        println!("{}", analysis);
        return Ok(());
    }

    // Handle analyze-quality
    if args.analyze_quality {
        use abyss::runner::discover_files;
        use abyss::utils::quality::analyze_quality;
        use abyss::utils::graph::DependencyGraph;
        
        let (files, root, _dropped) = discover_files(&config, None)?;
        
        // Build a simple dependency graph for quality analysis
        let mut graph = DependencyGraph::new();
        for path in &files {
            graph.add_node(path.clone());
        }
        
        // Estimate tokens for each file
        let file_tokens: Vec<_> = files
            .iter()
            .map(|f| {
                let size = std::fs::metadata(f).map(|m| m.len()).unwrap_or(0);
                (f.clone(), (size as usize) / 4)
            })
            .collect();
        
        let quality = analyze_quality(&files, &files, &graph, &file_tokens);
        println!("{}", quality);
        return Ok(());
    }

    // Handle query-driven context
    if let Some(query_str) = &args.query {
        use abyss::runner::discover_files;
        use abyss::utils::query::QueryEngine;
        use abyss::utils::graph::DependencyGraph;
        use std::collections::HashMap;

        let (files, root, _dropped) = discover_files(&config, None)?;

        // Build dependency graph for PageRank
        let mut graph = DependencyGraph::new();
        for path in &files {
            graph.add_node(path.clone());
        }

        // Create query engine
        let engine = QueryEngine::new(query_str, &graph);

        println!("Query: \"{}\"", query_str);
        println!("Keywords: {:?}", engine.keywords());
        println!("Expanded: {:?}", engine.expanded_keywords());
        println!();

        // Get token estimates for budget
        let file_tokens: HashMap<_, _> = files
            .iter()
            .map(|f| {
                let size = std::fs::metadata(f).map(|m| m.len()).unwrap_or(0);
                (f.clone(), (size as usize) / 4)
            })
            .collect();

        // Get relevant files within budget
        let max_tokens = config.max_tokens.unwrap_or(100_000);
        let relevant = engine.get_files_within_budget(&files, max_tokens, &file_tokens);

        println!("Found {} relevant files (within {} token budget):", relevant.len(), max_tokens);
        for (i, file) in relevant.iter().enumerate().take(20) {
            let rel_path = file.strip_prefix(&root).unwrap_or(file);
            println!("  {}. {}", i + 1, rel_path.display());
        }
        if relevant.len() > 20 {
            println!("  ... and {} more", relevant.len() - 20);
        }

        // If output specified, run with filtered files
        if config.output.to_string_lossy() != "output.xml" {
            println!();
            println!("Generating context for relevant files...");
            // Update config to filter to relevant files only
            // For now, just inform user
            println!("Use: abyss . --include [patterns] to filter");
        }

        return Ok(());
    }

    // Handle impact analysis
    if args.show_impact {
        use abyss::runner::discover_files;
        use abyss::utils::impact::ImpactAnalyzer;
        use abyss::utils::graph::DependencyGraph;
        use abyss::utils::git_stats::get_diff_files;

        let (files, root, _dropped) = discover_files(&config, None)?;

        // Build dependency graph
        let mut graph = DependencyGraph::new();
        for path in &files {
            graph.add_node(path.clone());
            if let Ok(content) = std::fs::read_to_string(path) {
                let extension = path.extension().and_then(|s| s.to_str()).unwrap_or("");
                let imports = abyss::utils::dependencies::extract_imports(&content, extension);
                for import in imports {
                    if let Some(dep_path) =
                        abyss::utils::dependencies::resolve_import(&import, path, &root)
                    {
                        graph.add_edge(path.clone(), dep_path);
                    }
                }
            }
        }

        // Get changed files from diff
        let diff_target = config.diff.as_deref().unwrap_or("HEAD~1");
        let changed_files = match get_diff_files(&root, diff_target) {
            Some(files) => files.into_iter().map(|s| root.join(s)).collect::<Vec<_>>(),
            None => {
                eprintln!("Could not get diff files. Use --diff to specify a reference.");
                return Ok(());
            }
        };

        if changed_files.is_empty() {
            println!("No changed files found relative to '{}'", diff_target);
            return Ok(());
        }

        // Run impact analysis
        let analyzer = ImpactAnalyzer::new(&graph);
        let analysis = analyzer.analyze(&changed_files, &files);

        println!("{}", analysis);
        return Ok(());
    }

    if args.tui {
        abyss::tui::start_tui(config)?;
    } else {
        run(config)?;
    }

    Ok(())
}
