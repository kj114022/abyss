# Abyss for VS Code

Generate LLM-optimized context for your code using the [Abyss](https://github.com/kj/abyss) engine.

## Features

- **Generate Context**: Right-click or run command to scan your workspace.
- **Smart Compression**: Automatically compresses code structure for token efficiency.
- **Copy to Clipboard**: Quick export for pasting into ChatGPT/Claude.

## Requirements

You must have the `abyss` CLI installed:

```bash
npm install -g abyss-cli
```

## Extension Settings

* `abyss.format`: Output format (markdown, xml, etc.). Default: `markdown`.
* `abyss.maxTokens`: Max tokens budget. Default: `128000`.

## Known Issues

None.

## Release Notes

### 2.0.1
Initial release matching Abyss v2.0.1 core.
