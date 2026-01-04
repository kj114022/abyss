# Abyss

<div align="center">

![Abyss](https://img.shields.io/badge/Abyss-v1.0-blueviolet?style=for-the-badge&logo=rust)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg?style=for-the-badge)](https://opensource.org/licenses/MIT)

**The Cognitive Code Interface for AI Agents.**

*Not just a packer. A bridge between human codebases and artificial intelligence.*

</div>

---

## Overview

Abyss is a high-performance **context generation engine** built in Rust. It transforms raw source code into optimized, token-aware, and spatially organized prompts for Large Language Models (LLMs).

Unlike simple file concatenators, Abyss "reads" your code like a senior engineer. It understands **abstract syntax trees (ASTs)**, **git history**, **dependency graphs**, and **semantic relevance**, allowing you to feed tens of thousands of lines of code into an LLM without wasting context window or losing coherence.

**Key Capabilities:**
- **Smart Ranking**: Prioritizes "entry point" files (main.rs, index.ts) and frequently churned code.
- **AST Compression**: Strips implementation details while preserving signatures (Rust, JS, TS, Python).
- **Temporal Context**: Enriches code with "evolution narratives" derived from Git history.
- **Mission Control TUI**: A terminal interface for interactive file selection and configuration.

## Features

### Intelligence Layer
- **AST-Aware Compression**: Reduce token usage by 60% by stripping function bodies but keeping signatures. Perfect for "high-level architecture" prompts.
- **Temporal Awareness**: Automatically detects "hot spots" in your codebase based on recent commit activity.
- **Dependency Graphing**: Generates Mermaid diagrams of module relationships to help LLMs visualize architecture.
- **PII Redaction**: Built-in privacy layer automatically redacts API keys, email addresses, and secrets.

### Mission Control (TUI)
Launch the interactive dashboard with `abyss --tui`.
- **The Navigator**: Traverse your file tree with keyboard navigation.
- **The Inspector**: Preview files with syntax highlighting and metadata.
- **The Commander**: Adjust settings (depth, compression, format) in real-time and rescan instantly.

### Production Ready
- **Multi-Format**: Output to XML (default), JSON, Markdown, or Plain Text.
- **Cost Estimation**: Estimates API costs for GPT-4, Claude 3, and DeepSeek.
- **Parallel Processing**: Blazing fast optimization using Rayon for multi-threaded traversal.
- **Remote Cloning**: Seamlessly handles HTTPS and SSH git URLs.

## Installation

```bash
# From source
cargo install --path .
```

## Usage

### Quick Start
```bash
# Scan current directory and output to XML (stdout)
abyss .

# Copy to clipboard immediately
abyss . --copy
```

### Advanced Workflows

**1. The "Architectural View" (Low Token Cost)**
Get a high-level overview of a large repo without the noise.
```bash
abyss . --smart --format markdown --max-depth 2
```

**2. The "Bug Fix" Context (High Specificity)**
Focus on a specific module, including recent changes and relevant dependencies.
```bash
abyss src/auth --include "**/*.rs" --temporal --diff main
```

**3. Interactive Mode**
Select exactly which files to include using the TUI.
```bash
abyss . --tui
```

## Configuration

Abyss supports a rich set of CLI flags for granular control:

| Category | Flag | Description |
|----------|------|-------------|
| **Core** | `--format <fmt>` | `xml` (default), `json`, `md`, `plain`. |
| | `--output <file>` | Write to file instead of stdout. |
| | `--copy` | Copy output to system clipboard. |
| **Optimization** | `--smart` | Enable AST-based compression (signatures only). |
| | `--compress` | Simple whitespace/comment removal. |
| | `--no-tokens` | Skip token counting (faster). |
| **Filtering** | `--max-depth <N>` | Limit traversal depth. |
| | `--max-size <N>` | Skip files larger than N bytes. |
| | `--ignore <glob>` | Add custom ignore patterns. |
| | `--include <glob>` | Whitelist specific patterns. |
| **Intelligence** | `--temporal` | Include Git history context. |
| | `--diff <ref>` | Only include changed files vs git ref. |
| | `--cost` | Show estimated API costs. |

## Privacy & Security

Abyss is designed for enterprise use.
- **Local Processing**: All analysis happens locally on your machine.
- **Secret Redaction**: Regex-based filters automatically mask common API key patterns and PII.
- **Respects .gitignore**: Automatically honors your project's ignore files.

## Contributing

Contributions are welcome! Please ensure you pass all strict linting rules:

```bash
cargo clippy -- -D warnings
cargo test
```

## License

MIT License. See [LICENSE](LICENSE) for details.
