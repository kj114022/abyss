# Abyss Command Reference

**The LLM Context Compiler**

---

## Quick Start

```bash
# Basic scan
abyss . -o context.xml

# With token budget and smart compression
abyss . --smart --max-tokens 128000 -o optimized.xml

# Interactive TUI mode
abyss . --tui

# LLM presets
abyss . --claude -o claude.xml    # 200K tokens
abyss . --gpt -o gpt.xml          # 128K tokens  
abyss . --gemini -o gemini.xml    # 1M tokens
```

---

## Complete CLI Reference

### Syntax

```
abyss [OPTIONS] [PATH]
```

**Arguments:**
- `[PATH]` - Directory or remote URL to scan (default: `.`)

---

### Core Options

| Option | Description |
|--------|-------------|
| `-o, --output <FILE>` | Output file path |
| `-f, --format <FORMAT>` | Output format: `xml`, `json`, `md`, `plain` |
| `-c, --copy` | Copy output to clipboard |
| `-v, --verbose` | Verbose output |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

---

### Filtering

| Option | Description |
|--------|-------------|
| `--ignore <PATTERN>` | Add ignore pattern (glob). Repeatable. |
| `--include <PATTERN>` | Only include matching files. Repeatable. |
| `--max-size <BYTES>` | Skip files larger than N bytes |
| `--max-depth <N>` | Maximum directory depth to traverse |

---

### Token Management

| Option | Description |
|--------|-------------|
| `--max-tokens <N>` | Token budget limit (uses knapsack algorithm) |
| `--split <N>` | Split output into chunks of N tokens |
| `--no-tokens` | Disable token counting (2x faster) |

**LLM Presets:**
| Option | Token Limit |
|--------|-------------|
| `--gpt` | 128,000 tokens |
| `--claude` | 200,000 tokens |
| `--gemini` | 1,000,000 tokens |

---

### Compression

| Option | Description |
|--------|-------------|
| `--compress` | Simple compression (remove comments/whitespace) |
| `--smart` | AST-aware compression (preserve signatures, remove bodies) |
| `--compress-level <LEVEL>` | Fine-grained control: `none`, `light`, `standard`, `aggressive` |
| `--tier <TIER>` | Context tier: `summary`, `detailed`, `full` |

**Compression Levels:**
- `none` - Full source code
- `light` - Remove comments and extra whitespace
- `standard` - Remove comments, whitespace, and simple boilerplate
- `aggressive` - Replace function bodies with placeholders

**Context Tiers:**
- `summary` - Signatures only (~10% size)
- `detailed` - Interfaces + key implementations (~30% size)
- `full` - Complete source code (default)

---

### Git Integration

| Option | Description |
|--------|-------------|
| `--diff <REF>` | Only scan files changed vs git ref (e.g., `main`, `HEAD~1`) |
| `--graph` | Generate Mermaid dependency graph |
| `--show-impact` | Show impact analysis for changed files (use with `--diff`) |
| `--explain-diff` | Add semantic explanation of changes |

---

### Privacy & Security

| Option | Description |
|--------|-------------|
| `--redact` | Remove secrets, API keys, emails, AWS credentials |

**Redacted Patterns:**
- OpenAI keys (`sk-...`)
- AWS Access Key IDs (`AKIA...`)
- AWS Secret Access Keys
- Email addresses
- Private key headers

---

### Advanced Features

| Option | Description |
|--------|-------------|
| `--prompt <TEXT>` | Prepend custom instruction to output |
| `--prompt-file <FILE>` | Read prompt from file |
| `--query <QUESTION>` | Query-driven context: find files relevant to a question |
| `--watch` | Watch mode: regenerate context on file changes |
| `--bundle <PATH>` | Export as portable bundle (JSON or .tar.gz) |
| `--cursor` | Output in Cursor-compatible JSON format |

---

### Analysis & Debugging

| Option | Description |
|--------|-------------|
| `--dry-run` | Show pre-flight analysis without processing |
| `--analyze-quality` | Analyze context quality and exit |
| `--completions <SHELL>` | Generate shell completions (`bash`, `zsh`, `fish`, `powershell`) |

---

## Configuration File

Create `abyss.toml` in project root:

```toml
path = "."
output = "context.xml"
output_format = "Xml"  # Xml | Json | Markdown | Plain

max_tokens = 128000
compression = "Smart"  # None | Simple | Smart
redact = true

ignore_patterns = ["*.test.ts", "mock_*", "dist/"]
include_patterns = ["src/**/*.rs"]

[git]
diff = "main"
graph = true
explain_diff = true
```

---

## Magic Patterns

### Code Review
```bash
abyss . --diff origin/main --graph --show-impact -o review.xml
```

### Architecture Overview
```bash
abyss . --smart --graph -f md --tier summary -o architecture.md
```

### Security Audit
```bash
abyss . --redact --max-tokens 100000 --query "authentication and authorization" -o security.xml
```

### Onboarding Documentation
```bash
abyss . --smart --prompt "Explain the high-level architecture" -o onboarding.md
```

### Continuous Integration
```bash
abyss . --diff HEAD~1 --analyze-quality --dry-run
```

### Watch Mode Development
```bash
abyss . --watch --smart -o context.xml
```

---

## Shell Completions

```bash
# Bash
abyss --completions bash > ~/.local/share/bash-completion/completions/abyss

# Zsh
abyss --completions zsh > ~/.zfunc/_abyss

# Fish
abyss --completions fish > ~/.config/fish/completions/abyss.fish

# PowerShell
abyss --completions powershell > abyss.ps1
```

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | Error (file not found, invalid config, etc.) |

---

## Environment Variables

| Variable | Description |
|----------|-------------|
| `ABYSS_CACHE_DIR` | Override cache directory (default: `~/.cache/abyss`) |

---

## Performance Tips

1. **Use `--no-tokens`** for fastest scans when you don't need budget limits
2. **Use `--smart`** to reduce output size by 40-60%
3. **Cache is automatic** - second runs on same repo are instant
4. **Use `--include`** to focus on specific directories

---

**End!**
