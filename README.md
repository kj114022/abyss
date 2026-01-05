# abyss

A semantic context engine for Large Language Models.
Optimizes codebase context ingestion through graph-theoretic ranking, AST-aware compression, and multi-modal analysis.

## Core Architecture

Abyss is not a concatenation tool. It is a relevance engine that transforms raw source code into high-signal prompts by prioritizing information density and structural centrality.

### 1. Semantic Relevance Engine
The system ranks files based on a weighted multi-variable score:
- **PageRank Centrality**: Computes the dependency graph to weight "core" modules (highly referenced) over "leaf" nodes.
- **Shannon Entropy**: Measures information density (byte distribution) to distinguish algorithmic logic from boilerplate.
- **Topological Sorting**: Context is assembled in dependency order (libraries before consumers) to maximize LLM comprehension.
- **Git Heuristics**: Files with high churn or recent modifications are boosted in relevance.

### 2. AST-Aware Compression
Reduces token density without information loss:
- **Smart Mode**: Parses source code (Rust, TypeScript, Python) to strip comments and normalize whitespace while preserving function signatures and structural hierarchy.
- **Knapsack Filtering**: Enforces strict token limits (e.g., 32k) by discarding lowest-ranking files, ensuring the context window contains the most "expensive" information first.

### 3. Omnivore Engine (Multi-Modal)
- **PDF Extraction**: Integrated `pdf-extract` for specification and paper analysis.
- **Binary Classification**: Heuristic detection of binary assets (images, compiled objects) to exclude them from text context, inserting metadata placeholders instead.

## Installation

Building from source requires a robust Rust toolchain.

```bash
cargo install --path .
```

## Usage

### Interactive Dashboard (TUI)
Launch the mission control interface for precise file selection and configuration.

```bash
abyss --tui
```

**Key Bindings:**
- `t`: Enter Task Mode (fuzzy search/filter).
- `Ctrl+a`: Select all visible files in Task Mode.
- `Mouse`: Scroll lists and configurations.
- `r`: Trigger re-scan of selected files.

### CLI Automation
Generate XML context for the current directory, optimized for a 32k token window.

```bash
abyss . --max-tokens 32000 --output context.xml
```

### Configuration
Abyss defaults to sensible production standards. Override via CLI or `abyss.toml`.

| Flag | Description |
|------|-------------|
| `--graph` | Inject Mermaid dependency diagrams into the output context. |
| `--diff <REF>` | Restrict scan to files changed relative to Git REF (e.g., `HEAD~1`). |
| `--smart` | Enable AST-aware compression (removes comments, compacts code). |
| `--no-tokens` | Disable token counting for maximum IO throughput. |

## Engineering Standards

This project adheres to strict production engineering constraints:
- **Zero Ambiguity**: Defaults are explicit. Behavior is deterministic.
- **Observability**: Verbose logging explains exclusion decisions.
- **Performance**: Parallel directory walking, cached syntax highlighting, and zero-copy string handling where possible.

## License
Proprietary. All Rights Reserved.
Unauthorized copying, modification, distribution, or use of this software is strictly prohibited.

