# Integrations Guide

This guide covers how to integrate abyss with popular development tools and CI/CD pipelines.

## Quick Links

- [GitHub Actions](#github-actions)
- [Cursor](#cursor)
- [VSCode](#vscode)
- [Shell Integration](#shell-integration)
- [Pre-commit Hook](#pre-commit-hook)

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

## Cursor

Cursor is an AI-first code editor. Generate context compatible with Cursor's context API.

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
