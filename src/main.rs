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
#[command(author, version, about="The Ultimate Repository Packer (The Abyss)", long_about = None)]
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

    // Compression Logic merge
    if args.smart {
        config.compression = CompressionMode::Smart;
    } else if args.compress {
        config.compression = CompressionMode::Simple;
    }
    // Else keep config value

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

    if args.tui {
        abyss::tui::start_tui(config)?;
    } else {
        run(config)?;
    }

    Ok(())
}
