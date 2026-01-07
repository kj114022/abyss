# abyss

**LLM Context Compiler**

Transform entire codebases into semantically-ordered, dependency-aware, optimized context for Large Language Models.

---

## The Problem

When feeding code to LLMs (Claude, GPT, etc.), **order matters**:

- Alphabetical file ordering shows usage before definitions
- Random order confuses the model's understanding
- Token budgets get wasted on low-value boilerplate
- Critical architectural context gets buried

**Most tools just concatenate files. That's not good enough.**

---

## The Solution

abyss is a **semantic compiler for code context** that:

1. **Builds dependency graphs** - understands imports and module relationships
2. **Ranks files semantically** - README first, core before utils, tests last
3. **Sorts topologically** - definitions always before usage
4. **Optimizes token budgets** - knapsack algorithm for maximum value
5. **Compresses intelligently** - AST-aware compression preserves signatures, removes bodies

**Result:** LLMs get context in the order they need to understand your codebase.

---

## Installation

```bash
# Quick start (npx)
npx abyss-cli . -o context.xml

# Or install from source
git clone https://github.com/kj114022/abyss
cd abyss
cargo install --path .
```

**Requirements:** Node.js (for npx) OR Rust 1.70+, Git (optional, for diff features)

---

## Quick Start

```bash
# Basic scan - current directory
abyss . -o context.xml

# With token budget and compression
abyss . --max-tokens 128000 --smart -o optimized.xml

# Interactive TUI mode
abyss . --tui

# Only changed files (diff mode)
abyss . --diff main -o changes.xml
```

---

## Key Features

### 1. Dependency-Aware Ordering

Traditional tools do this:
```
utils/helper.rs      (uses Config)
config.rs            (defines Config)  ← Wrong order!
main.rs              (uses everything)
```

abyss does this:
```
config.rs            (defines Config)
utils/helper.rs      (uses Config)     ← Correct order!
main.rs              (uses everything)
```

### 2. Semantic Ranking

Files are scored by importance:
- `README.md` → 1000 (documentation first)
- `Cargo.toml` → 800 (project config)
- `src/main.rs` → 700 (entry points)
- `src/core/*` → 600 (core logic)
- `src/utils/*` → 400 (utilities)
- `tests/*` → 100 (tests last)

### 3. Token Budget Optimization

```bash
abyss . --max-tokens 50000 -o optimized.xml
```

Uses knapsack algorithm to select highest-value files within budget. Logs dropped files.

### 4. AST-Aware Compression

Preserves function signatures, removes implementation:

```rust
// Before
fn complex_logic(x: i32) -> Result<String> {
    let mut result = String::new();
    for i in 0..x {
        // 50 lines of logic...
    }
    Ok(result)
}

// After (--smart flag)
fn complex_logic(x: i32) -> Result<String> { /* ... */ }
```

Supports: Rust, Python, JavaScript/TypeScript, Go, C/C++

### 5. Multiple Output Formats

```bash
abyss . -f xml      # Structured (default)
abyss . -f json     # API-friendly
abyss . -f md       # Human-readable with syntax highlighting
abyss . -f plain    # Minimal overhead
```

---

## Common Use Cases

### Code Review Preparation

```bash
# Get context for changed files only
abyss . --diff origin/main -o review.xml
```

### Documentation Generation

```bash
# Full codebase with dependency graph
abyss . --graph --format md -o architecture.md
```

### LLM-Assisted Refactoring

```bash
# Focus on specific subsystem
abyss . --include "src/auth/**" --max-tokens 100000 -o auth-system.xml
```

### Onboarding New Developers

```bash
# Generate comprehensive overview with summaries
abyss . --smart --graph -o codebase-overview.xml
```

### Security Audits

```bash
# High-churn files (frequently changed = higher risk)
abyss . --max-tokens 50000 -o security-review.xml
# Files are automatically ranked by git churn + centrality
```

---

## Advanced Usage

### Configuration File

Create `abyss.toml` in project root:

```toml
path = "."
output = "context.xml"
output_format = "Xml"  # Xml | Json | Markdown | Plain

max_tokens = 128000
compression = "Smart"  # None | Simple | Smart
redact = true          # Remove secrets/API keys

ignore_patterns = ["*.test.ts", "mock_*"]
include_patterns = []  # If set, only these files included

[git]
diff = "main"   # Compare against this branch
graph = true    # Generate dependency graph
```

Then just run: `abyss`

### Privacy & Security

```bash
# Automatically redact sensitive data
abyss . --redact -o safe.xml
```

Removes:
- API keys and tokens
- Email addresses
- AWS credentials
- Private keys

### Custom Prompts

```bash
# Prepend instruction to LLM
abyss . --prompt "Focus on security vulnerabilities" -o analysis.xml

# Or from file
abyss . --prompt-file instructions.txt -o analysis.xml
```

### Copy to Clipboard

```bash
# Quick copy for pasting
abyss . --copy --format plain
```

### Split Large Outputs

```bash
# Split into 100K token chunks
abyss . --split 100000 -o chunks.xml
# Creates: chunks-part-1.xml, chunks-part-2.xml, etc.
```

---

## How It Works

### Intelligence Pipeline

```
1. Discovery         → Walk directory tree (respects .gitignore)
2. Analysis          → Extract imports, measure entropy, count tokens
3. Graph Building    → Build dependency graph from imports
4. Scoring           → Combine heuristics + PageRank + git churn
5. Topological Sort  → Order files (definitions before usage)
6. Budget Selection  → Knapsack algorithm if --max-tokens set
7. Processing        → Parallel compression + summarization
8. Output            → Stream to format (XML/JSON/Markdown/Plain)
```

### Supported Languages

**Full AST parsing:**
- Rust
- Python
- JavaScript/TypeScript
- Go
- C/C++

**Regex fallback:** Other languages

---

## CLI Reference

```
Usage: abyss [PATH] [OPTIONS]

Arguments:
  [PATH]  Directory to scan [default: .]

Options:
  -o, --output <FILE>         Output file [default: abyss-output.xml]
  -f, --format <FORMAT>       Output format: xml|json|md|plain
      --max-tokens <N>        Token budget limit
      --split <N>             Split into N-token chunks
      --diff <REF>            Only files changed vs git ref
      --graph                 Generate dependency graph
      --smart                 AST-aware compression
      --compress              Simple compression (comments only)
      --redact                Remove secrets/PII
      --ignore <PATTERN>      Ignore pattern (repeatable)
      --include <PATTERN>     Include only (repeatable)
      --prompt <TEXT>         Prepend instruction
      --prompt-file <FILE>    Read prompt from file
  -c, --copy                  Copy to clipboard
      --tui                   Interactive mode
      --no-tokens             Skip token counting (faster)
  -v, --verbose               Verbose output
  -h, --help                  Print help
```

---

## Performance

Benchmarks on 50K LOC Rust project (M2, 16GB):

| Mode | Time | Files | Tokens | Output |
|------|------|-------|--------|--------|
| Standard | 2.3s | 487 | 156K | 2.1 MB |
| Smart Compression | 3.1s | 487 | 89K | 1.2 MB |
| Token Budget (100K) | 1.8s | 312 | 98K | 1.4 MB |

---

## Why Not Just Use...?

### Concentration Tools?
- **others:** Only Basic concatenation, alphabetical order
- **abyss:** Dependency-aware, semantic ranking, token optimization

### IDE integrations (Cursor, Copilot)?
- **IDE tools:** Real-time assistance, open files only
- **abyss:** Full codebase analysis, reproducible context files

### Manual file selection?
- **Manual:** 10-30 minutes, might miss dependencies
- **abyss:** 2 seconds, guaranteed dependency coverage

---

## License

MIT License - see [LICENSE](LICENSE)

---

## Try It Out

```bash
# Development
cargo build
cargo test
```