# ctx

**Curate what your LLM sees.** Define reusable context packs and load them directly into Claude Code, Cursor, or any MCP-compatible tool.

```bash
# Create a context pack for a feature
ctx create auth-refactor
ctx add auth-refactor file:src/auth.rs --priority 10
ctx add auth-refactor 'glob:src/middleware/*.rs'
ctx add auth-refactor 'git:diff --base=main'
ctx add auth-refactor 'text:We are migrating from JWT to session tokens'

# Connect to Claude Code via MCP
claude mcp add ctx -- ctx mcp --stdio

# Now Claude can load your curated context:
# "Load the auth-refactor pack"
```

---

## Why ctx?

LLMs can discover files, but they can't read your mind. ctx lets you:

- **Pre-curate context** for specific tasks (refactors, reviews, features)
- **Control token budgets** with priorities - important files first
- **Share context setups** via `ctx.toml` in version control
- **Auto-redact secrets** before they reach the LLM
- **Find related files** via git history and import analysis

---

## Install

**Homebrew** (macOS/Linux):
```bash
brew install ctx-dev/tap/ctx
```

**Pre-built binaries**:
```bash
# Download from GitHub releases
curl -fsSL https://github.com/ctx-dev/ctx/releases/latest/download/ctx-$(uname -m)-$(uname -s | tr '[:upper:]' '[:lower:]').tar.gz | tar xz
sudo mv ctx /usr/local/bin/
```

**Cargo** (from source):
```bash
cargo install ctx-cli
```

**VS Code Extension**:
Search "ctx" in the VS Code marketplace, or:
```bash
code --install-extension ctx-dev.vscode-ctx
```

---

## Quick Start

### One-off context (fastest)
```bash
ctx @ src/auth.rs              # File + related files → clipboard
ctx @ src/auth.rs --output     # Print to stdout instead
```

### Reusable packs (for repeated tasks)
```bash
# Create a pack
ctx create auth-feature

# Add sources
ctx add auth-feature file:src/auth.rs
ctx add auth-feature file:src/middleware.rs --priority 10
ctx add auth-feature 'glob:tests/auth/**/*.rs'
ctx add auth-feature 'git:diff --base=main'
ctx add auth-feature 'text:Focus on the JWT validation logic'

# Auto-add related files
ctx add auth-feature file:src/auth.rs --with-related

# Preview token usage
ctx preview auth-feature --tokens

# Copy to clipboard
ctx cp auth-feature
```

### Share with your team
```bash
ctx init                    # Create ctx.toml
ctx save auth-feature       # Export pack to ctx.toml
git add ctx.toml && git commit -m "Add context packs"

# Teammates run:
ctx sync                    # Import packs from ctx.toml
```

---

## Features

| Feature | Description |
|---------|-------------|
| **Smart suggestions** | Auto-discover related files via git history + import analysis |
| **Token budgets** | Set limits, see what fits, prioritize what matters |
| **Secret redaction** | AWS keys, tokens, and secrets auto-redacted |
| **Git-aware** | Include diffs, respect .gitignore |
| **Project-local** | Share packs via `ctx.toml` in version control |

### Source Types

```bash
ctx add pack file:src/main.rs              # Single file
ctx add pack file:src/main.rs --start 10 --end 50   # Line range
ctx add pack 'glob:src/**/*.rs'            # Glob pattern
ctx add pack 'git:diff --base=main'        # Git diff
ctx add pack 'url:https://docs.example.com/api'     # Web page
ctx add pack 'text:Remember to use async/await'     # Inline text
```

---

## VS Code Extension

The VS Code extension provides:
- **Sidebar** with pack browser
- **Right-click → Add to Pack** on any file
- **Cmd+Shift+C S** to show related files
- **Preview panel** with token counts
- **ctx.toml** sync notifications

Install from the marketplace or build locally:
```bash
cd vscode-ctx
npm install && npm run compile
code --install-extension ctx-dev.vscode-ctx-0.1.0.vsix
```

---

## MCP Integration (Claude Code)

Connect ctx to Claude Code for AI-native pack management:

```bash
# Add ctx as an MCP server
claude mcp add ctx -- ctx mcp --stdio

# Now ask Claude:
# "List my ctx packs"
# "Preview the auth pack"
# "Add src/utils.rs to the auth pack"
# "Load the auth pack" (injects context directly)
```

For other MCP clients (HTTP transport):
```bash
ctx mcp --port 17373
# Connect to http://127.0.0.1:17373
```

---

## REST API

ctx exposes a REST API for custom integrations:

```bash
ctx mcp --port 17373

# Endpoints:
GET  /api/packs                    # List packs
POST /api/packs                    # Create pack
GET  /api/packs/:name              # Get pack
GET  /api/packs/:name/render       # Get rendered content
POST /api/packs/:name/artifacts    # Add artifact
GET  /api/suggest?file=path        # Get file suggestions
```

Use this for ChatGPT Actions, custom scripts, or CI/CD pipelines.

---

## Commands Reference

| Command | Description |
|---------|-------------|
| `ctx @` | Quick context: file + related → clipboard |
| `ctx create` | Create a new pack |
| `ctx add` | Add source to pack |
| `ctx rm` | Remove artifact from pack |
| `ctx ls` | List all packs |
| `ctx show` | Show pack details |
| `ctx preview` | Preview with token counts |
| `ctx cp` | Copy rendered pack to clipboard |
| `ctx delete` | Delete a pack |
| `ctx lint` | Find missing dependencies |
| `ctx suggest` | Get related file suggestions |
| `ctx init` | Create ctx.toml |
| `ctx sync` | Import from ctx.toml |
| `ctx save` | Export to ctx.toml |
| `ctx mcp` | Start MCP/REST server |
| `ctx ui` | Interactive terminal UI |
| `ctx completions` | Generate shell completions |

See **[CTX_REFERENCE.md](./CTX_REFERENCE.md)** for full documentation.

---

## Configuration

**Global config**: `~/.config/ctx/config.toml`
```toml
budget_tokens = 128000

[denylist]
patterns = ["**/.env*", "**/secrets/**"]

[mcp]
port = 17373
```

**Project config**: `ctx.toml`
```toml
[config]
default_budget = 50000

[packs.auth-feature]
budget = 80000
artifacts = [
    { source = "file:src/auth.rs", priority = 10 },
    { source = "glob:tests/auth/**/*.rs" },
    { source = "text:Focus on JWT validation" },
]
```

---

## Development

```bash
git clone https://github.com/ctx-dev/ctx
cd ctx

# Build
cargo build --release

# Test
cargo test --workspace
./tests/integration_test.sh

# VS Code extension
cd vscode-ctx
npm install
npm run compile
```

See **[ARCHITECTURE.md](./ARCHITECTURE.md)** for technical details.

---

## License

MIT OR Apache-2.0
