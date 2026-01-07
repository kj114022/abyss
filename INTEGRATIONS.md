# Integrations Guide

This guide covers how to integrate abyss with popular LLM platforms, development tools, and CI/CD pipelines.

## Quick Links

### LLM Platforms
- [ChatGPT / OpenAI](#chatgpt--openai)
- [Claude / Anthropic](#claude--anthropic)
- [Gemini / Google](#gemini--google)
- [Grok / xAI](#grok--xai)
- [Cursor](#cursor)
- [All-in-One LLM Setup](#all-in-one-llm-setup)

### Developer Tools
- [GitHub Actions](#github-actions)
- [VSCode](#vscode)
- [Shell Integration](#shell-integration)
- [Pre-commit Hook](#pre-commit-hook)
- [CI/CD Examples](#cicd-examples)

---

## LLM Platform Integrations

Each LLM has different context windows and optimal formats. Here's how to use abyss with each:

| Platform | Context Window | Recommended Setup |
|----------|---------------|-------------------|
| GPT | ~ 128K tokens | `--gpt --format markdown` |
| Claude | ~ 200K tokens | `--claude --format xml` |
| Gemini | ~ 1M tokens | `--gemini --format markdown` |
| Grok | ~ 128K tokens | `--gpt --format markdown` |

---

## ChatGPT / OpenAI

### Quick Start

```bash
# GPT optimized (128K tokens)
abyss . --gpt -o context.md

# With query focus
abyss . --gpt --query "authentication system" -o auth.md

# PR review
abyss . --gpt --diff main -o pr.md
```

### Usage with ChatGPT

1. Generate context:
   ```bash
   abyss . --gpt --format markdown -o context.md
   ```

2. Open ChatGPT (web or API)

3. Paste the context and ask:
   ```
   Here's my codebase context:
   
   [paste context.md contents]
   
   Question: How does the authentication flow work?
   ```

### API Integration

```python
import openai
from pathlib import Path

# Generate context
import subprocess
subprocess.run(["abyss", ".", "--gpt", "-o", "context.md"])

context = Path("context.md").read_text()

response = openai.ChatCompletion.create(
    model="gpt",
    messages=[
        {"role": "system", "content": "You are a code expert. Use the provided codebase context to answer questions."},
        {"role": "user", "content": f"Codebase context:\n\n{context}\n\nHow does error handling work?"}
    ]
)
```

---

## Claude / Anthropic

### Quick Start

```bash
# Claude optimized (200K tokens, prefers XML)
abyss . --claude -o context.xml

# With compression for faster processing
abyss . --claude --compress-level light -o context.xml

# Focused context
abyss . --claude --query "database schema" -o db.xml
```

### Usage with Claude

Claude works exceptionally well with XML format (its training included significant XML data):

1. Generate context:
   ```bash
   abyss . --claude --format xml -o context.xml
   ```

2. In Claude, use the artifact structure:
   ```
   <codebase>
   [paste context.xml contents]
   </codebase>
   
   Analyze the authentication flow and suggest improvements.
   ```

### API Integration

```python
import anthropic
from pathlib import Path

client = anthropic.Client()

# Generate context
import subprocess
subprocess.run(["abyss", ".", "--claude", "-o", "context.xml"])

context = Path("context.xml").read_text()

response = client.messages.create(
    model="claude-sonnet-20240620",
    max_tokens=4096,
    messages=[
        {
            "role": "user",
            "content": f"<codebase>\n{context}\n</codebase>\n\nExplain the dependency injection pattern used here."
        }
    ]
)
```

---

## Gemini / Google

### Quick Start

```bash
# Gemini optimized (1M tokens - full codebase!)
abyss . --gemini -o context.md

# With NO compression (Gemini can handle it)
abyss . --gemini --compress-level none -o full-context.md

# Include everything
abyss . --gemini --max-tokens 500000 -o detailed.md
```

### Why Gemini is Special

Gemini Pro's 1M token context window means you can include your ENTIRE codebase:

```bash
# Include absolutely everything
abyss . --gemini --compress-level none --format markdown -o everything.md
```

### Usage with Gemini

1. Generate full context:
   ```bash
   abyss . --gemini -o context.md
   ```

2. In Gemini (AI Studio or API):
   ```
   I'm providing my complete codebase for analysis:
   
   [paste context.md contents]
   
   Create a comprehensive architecture diagram and identify potential issues.
   ```

### API Integration

```python
import google.generativeai as genai
from pathlib import Path

genai.configure(api_key="YOUR_API_KEY")

# Generate context (full codebase!)
import subprocess
subprocess.run(["abyss", ".", "--gemini", "-o", "context.md"])

context = Path("context.md").read_text()

model = genai.GenerativeModel('gemini-1.5-pro')
response = model.generate_content(
    f"Codebase:\n\n{context}\n\nProvide a complete security audit."
)
```

---

## Grok / xAI

### Quick Start

```bash
# Grok optimized (similar to ChatGPT)
abyss . --gpt -o context.md

# With wit (Grok appreciates concise context)
abyss . --gpt --compress-level standard -o context.md
```

### Usage with Grok

```bash
# Generate compressed context for Grok
abyss . --max-tokens 100000 --compress-level light --format markdown -o context.md
```

Then in Grok:
```
Here's my codebase:

[paste context]

Roast my code architecture (constructively).
```

---

## Cursor

Cursor is an AI-first code editor with native context support.

### Generate Cursor Context

```bash
# Generate Cursor-compatible JSON
abyss . --cursor -o context.json

# With query focus
abyss . --cursor --query "authentication flow" -o auth-context.json

# For PR review
abyss . --cursor --diff main --max-tokens 50000 -o pr-context.json
```

### Use in Cursor

1. Generate context: `abyss . --cursor -o context.json`
2. Open Cursor
3. Use "Add Context" (`Cmd+Shift+P` → "Add Context from File")
4. Select the generated JSON file
5. Ask questions about your codebase

### Cursor Rules Integration

Create a `.cursorrules` file using abyss-generated context:

```bash
# Generate codebase overview
abyss . --compress-level aggressive --max-tokens 5000 --format plain -o overview.txt

# Add to .cursorrules
echo "# Codebase Overview\n" > .cursorrules
cat overview.txt >> .cursorrules
```

---

## All-in-One LLM Setup

Create shell functions to quickly generate context for any LLM:

### Bash/Zsh Setup

Add to `~/.bashrc` or `~/.zshrc`:

```bash
# Generate context for any LLM
llm-ctx() {
  local platform="${1:-claude}"
  local query="$2"
  local output="/tmp/llm-context.md"
  
  case "$platform" in
    gpt|openai|chatgpt)
      abyss . --gpt --format markdown -o "$output"
      ;;
    claude|anthropic)
      abyss . --claude --format xml -o "${output%.md}.xml"
      output="${output%.md}.xml"
      ;;
    gemini|google)
      abyss . --gemini --format markdown -o "$output"
      ;;
    grok|xai)
      abyss . --gpt --compress-level light --format markdown -o "$output"
      ;;
    cursor)
      abyss . --cursor -o "${output%.md}.json"
      output="${output%.md}.json"
      ;;
    *)
      echo "Unknown platform: $platform"
      echo "Supported: gpt, claude, gemini, grok, cursor"
      return 1
      ;;
  esac
  
  if [ -n "$query" ]; then
    abyss . --query "$query" --format markdown -o "$output"
  fi
  
  cat "$output" | pbcopy
  echo "Context for $platform copied to clipboard (~$(wc -c < "$output" | awk '{print int($1/4)}') tokens)"
  echo "File: $output"
}

# Quick aliases
alias ctx-gpt='llm-ctx gpt'
alias ctx-claude='llm-ctx claude'
alias ctx-gemini='llm-ctx gemini'
alias ctx-grok='llm-ctx grok'
alias ctx-cursor='llm-ctx cursor'

# Query-driven
ctx-ask() {
  local platform="${1:-claude}"
  local query="$2"
  abyss . --query "$query" --max-tokens 50000 --format markdown -o /tmp/query-context.md
  cat /tmp/query-context.md | pbcopy
  echo "Query context for '$query' copied to clipboard"
}
```

### Usage

```bash
# Platform-specific
ctx-gpt                         # ChatGPT optimized
ctx-claude                      # Claude optimized
ctx-gemini                      # Gemini (full context)
ctx-grok                        # Grok optimized
ctx-cursor                      # Cursor JSON

# With query
llm-ctx claude "authentication" # Claude + auth focus
ctx-ask gpt "error handling"    # GPT + query
```

---

## GitHub Actions

Automatically generate LLM context for pull requests to enable AI-assisted code review.

### Basic Usage

```yaml
# .github/workflows/pr-context.yml
name: Generate PR Context

on:
  pull_request:
    types: [opened, synchronize]

jobs:
  generate-context:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
        with:
          fetch-depth: 0
          
      - name: Generate Context
        uses: ./.github/actions/abyss
        with:
          diff: ${{ github.base_ref }}
          max-tokens: '50000'
          compression: 'light'
```

### All Options

| Input | Description | Default |
|-------|-------------|---------|
| `path` | Directory to scan | `.` |
| `output` | Output file path | `abyss-context.xml` |
| `format` | Output format (xml, json, markdown, plain) | `xml` |
| `max-tokens` | Maximum tokens in output | `100000` |
| `compression` | Compression level (none, light, standard, aggressive) | `none` |
| `diff` | Generate context for changes relative to this ref | - |
| `query` | Query-driven context generation | - |
| `show-impact` | Show impact analysis | `false` |

### Outputs

| Output | Description |
|--------|-------------|
| `output-file` | Path to generated context file |
| `token-count` | Estimated token count |
| `file-count` | Number of files included |

### Advanced: PR Comment with Context

```yaml
- name: Generate Context
  id: abyss
  uses: ./.github/actions/abyss
  with:
    diff: ${{ github.base_ref }}
    
- name: Comment on PR
  uses: actions/github-script@v7
  with:
    script: |
      github.rest.issues.createComment({
        issue_number: context.issue.number,
        owner: context.repo.owner,
        repo: context.repo.repo,
        body: `Context generated: ${{ steps.abyss.outputs.file-count }} files, ~${{ steps.abyss.outputs.token-count }} tokens`
      });
```

---

## VSCode

### Manual Usage (Recommended for now)

1. Install abyss: `cargo install abyss`
2. Generate context from terminal:
   ```bash
   abyss . -o context.xml --max-tokens 50000
   ```
3. Open the context file in VSCode
4. Copy and paste into your LLM chat

### Keyboard Shortcut

Add to `keybindings.json`:

```json
{
  "key": "ctrl+shift+a",
  "command": "workbench.action.terminal.sendSequence",
  "args": {
    "text": "abyss . --format markdown -o /tmp/context.md && code /tmp/context.md\n"
  }
}
```

### Tasks Integration

Add to `.vscode/tasks.json`:

```json
{
  "version": "2.0.0",
  "tasks": [
    {
      "label": "Generate LLM Context",
      "type": "shell",
      "command": "abyss",
      "args": [
        ".",
        "--format", "markdown",
        "--max-tokens", "50000",
        "-o", "${workspaceFolder}/.abyss-context.md"
      ],
      "problemMatcher": [],
      "group": "build"
    },
    {
      "label": "Generate PR Context",
      "type": "shell", 
      "command": "abyss",
      "args": [
        ".",
        "--diff", "main",
        "--format", "markdown",
        "-o", "${workspaceFolder}/.pr-context.md"
      ],
      "problemMatcher": []
    }
  ]
}
```

Run with `Cmd+Shift+B` (Build Task) or `Cmd+Shift+P` → "Run Task".

---

## Shell Integration

### Bash/Zsh Aliases

Add to `~/.bashrc` or `~/.zshrc`:

```bash
# Quick context generation
alias ctx='abyss . --format markdown -o /tmp/context.md && cat /tmp/context.md | pbcopy && echo "Context copied to clipboard"'

# PR context
alias prctx='abyss . --diff main --format markdown -o /tmp/pr-context.md && cat /tmp/pr-context.md | pbcopy && echo "PR context copied"'

# Query-driven context
ctxq() {
  abyss . --query "$1" --format markdown -o /tmp/context.md
  cat /tmp/context.md | pbcopy
  echo "Context for '$1' copied to clipboard"
}

# Impact analysis
impact() {
  abyss . --diff "${1:-HEAD~1}" --show-impact
}
```

Usage:

```bash
ctx                          # Full context to clipboard
prctx                        # PR changes context
ctxq "how does auth work"    # Query-focused context
impact main                  # Impact analysis vs main
```

### Fish Shell

Add to `~/.config/fish/config.fish`:

```fish
function ctx
  abyss . --format markdown -o /tmp/context.md
  cat /tmp/context.md | pbcopy
  echo "Context copied to clipboard"
end

function ctxq
  abyss . --query "$argv" --format markdown -o /tmp/context.md
  cat /tmp/context.md | pbcopy
  echo "Context for '$argv' copied"
end
```

---

## Pre-commit Hook

Automatically update context when committing.

### Setup

1. Create `.git/hooks/pre-commit`:

```bash
#!/bin/bash
# Generate context for staged files

STAGED_FILES=$(git diff --cached --name-only --diff-filter=ACM)

if [ -n "$STAGED_FILES" ]; then
  echo "Updating .abyss-context.md..."
  abyss . --diff HEAD --format markdown -o .abyss-context.md
  git add .abyss-context.md
fi
```

2. Make executable:

```bash
chmod +x .git/hooks/pre-commit
```

### Using with husky

In `package.json`:

```json
{
  "husky": {
    "hooks": {
      "pre-commit": "abyss . --diff HEAD --format markdown -o .abyss-context.md && git add .abyss-context.md"
    }
  }
}
```

---

## CI/CD Examples

### GitLab CI

```yaml
# .gitlab-ci.yml
generate-context:
  stage: test
  image: rust:latest
  script:
    - cargo install abyss
    - abyss . --diff $CI_MERGE_REQUEST_TARGET_BRANCH_NAME -o context.xml
  artifacts:
    paths:
      - context.xml
    expire_in: 1 week
  only:
    - merge_requests
```

### Jenkins

```groovy
// Jenkinsfile
pipeline {
  agent any
  stages {
    stage('Generate Context') {
      steps {
        sh 'cargo install abyss'
        sh 'abyss . --diff origin/main -o context.xml'
        archiveArtifacts artifacts: 'context.xml'
      }
    }
  }
}
```

### CircleCI

```yaml
# .circleci/config.yml
version: 2.1
jobs:
  generate-context:
    docker:
      - image: rust:latest
    steps:
      - checkout
      - run: cargo install abyss
      - run: abyss . -o context.xml
      - store_artifacts:
          path: context.xml
```

---

## Configuration Tips

### Project-specific Settings

Create `.abyssignore` in your project root:

```
# Ignore test files for production context
*.test.ts
*.spec.js
__tests__/

# Ignore generated files
*.generated.ts
dist/
build/
```

### Environment Variables

```bash
# Set default max tokens
export ABYSS_MAX_TOKENS=100000

# Use in commands
abyss .  # Will use 100000 tokens by default
```

---

## Troubleshooting

### Common Issues

**Context too large:**
```bash
# Use compression
abyss . --compress-level aggressive

# Limit tokens
abyss . --max-tokens 30000

# Focus on specific query
abyss . --query "authentication"
```

**Missing files:**
```bash
# Check what's being included
abyss . --dry-run

# Force include patterns
abyss . --include "*.config.ts"
```

**Slow generation:**
```bash
# Use fast token estimation
abyss . --no-tokens

# Limit scope
abyss . --diff main
```

---

## Getting Help

- [GitHub Issues](https://github.com/yourorg/abyss/issues)
- [Documentation](https://github.com/yourorg/abyss#readme)
- Run `abyss --help` for all options
