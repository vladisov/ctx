# ctx User Guide & Reference

**ctx** creates repeatable context packs for LLM workflows. This guide covers installation, usage, and reference material.

## Getting Started

### Installation

```bash
# From source
cargo install --path crates/ctx-cli

# Verify
ctx --version
```

### Quick Example

```bash
# Create a pack with 5000 token budget
ctx pack create demo --tokens 5000

# Add some content
ctx pack add demo file:README.md
ctx pack add demo 'glob:src/**/*.rs'
ctx pack add demo 'text:Focus on error handling'

# Preview before using
ctx pack preview demo --tokens

# Get the full payload
ctx pack preview demo --show-payload
```

## Core Concepts

| Concept | Description |
|---------|-------------|
| **Pack** | Named bundle of sources with a token budget (default 128k) |
| **Artifact** | Single source item (file, glob pattern, git diff, text) |
| **Render** | Combines all artifacts into deterministic output |

## Source Types

### Files (`file:`)
```bash
ctx pack add demo file:Cargo.toml           # Single file
ctx pack add demo Cargo.toml                # Implicit scheme
ctx pack add demo Cargo.toml --start 10 --end 50  # Line range
ctx pack add demo Cargo.toml --priority 100       # Higher = kept first
```

### Text (`text:`)
```bash
ctx pack add demo 'text:Focus on error handling' --priority 500
```

### Glob Patterns (`glob:`)
```bash
ctx pack add demo 'glob:src/**/*.rs'        # All Rust files
ctx pack add demo 'glob:docs/**/*.md'       # All markdown in docs
ctx pack add demo 'glob:data/*.json'        # JSON files in data/
```

### Git Diffs (`git:`)
```bash
ctx pack add demo git:diff                  # Working tree vs HEAD
ctx pack add demo 'git:diff --base=main'    # Diff against main
ctx pack add demo 'git:diff --base=HEAD~3'  # Last 3 commits
```

### Markdown Directories (`md_dir:`)
```bash
ctx pack add demo md_dir:./docs --recursive
```

## CLI Quick Reference

```bash
# Pack management
ctx pack create <name>                 # Create pack (128k budget)
ctx pack create <name> --tokens 50000  # Custom budget
ctx pack list                          # List all packs
ctx pack show <name>                   # Show pack details
ctx pack delete <name>                 # Delete pack

# Artifacts
ctx pack add <pack> <source>           # Add artifact
ctx pack add <pack> 'glob:src/**/*.rs' # Quote globs!
ctx pack remove <pack> <artifact-id>   # Remove artifact

# Preview & Export
ctx pack preview <pack>                # Show stats
ctx pack preview <pack> --tokens       # Per-artifact tokens
ctx pack preview <pack> --show-payload # Full content

# Project-local packs
ctx init                               # Create ctx.toml
ctx pack sync                          # Sync from ctx.toml
ctx pack save <pack>                   # Save to ctx.toml

# Interactive
ctx ui                                 # Terminal UI
```

## Project-Local Packs (ctx.toml)

Save pack definitions to version control:

```bash
ctx init                    # Create ctx.toml
ctx pack save my-pack       # Export pack to ctx.toml
ctx pack sync               # Import packs from ctx.toml
```

Example `ctx.toml`:
```toml
[config]
default_budget = 50000

[packs.style-guide]
budget = 25000
artifacts = [
    { source = "file:CONTRIBUTING.md", priority = 10 },
    { source = "text:Use async/await patterns", priority = 100 },
]

[packs.feature-auth]
artifacts = [
    { source = "glob:src/auth/**/*.rs", priority = 0 },
    { source = "git:diff --base=main", priority = 5 },
]
```

Packs are auto-namespaced by project directory (e.g., `my-project:style-guide`).

## Common Pack Patterns

### Style Guide Pack
```bash
ctx pack create style-guide
ctx pack add style-guide file:CONTRIBUTING.md
ctx pack add style-guide file:.editorconfig
ctx pack add style-guide 'text:Conventions: use async/await, prefer explicit error handling'
```

### Architecture Pack
```bash
ctx pack create architecture
ctx pack add architecture file:README.md
ctx pack add architecture 'glob:src/lib.rs'
ctx pack add architecture 'glob:**/mod.rs'
ctx pack add architecture file:Cargo.toml
```

### Feature Context Pack
```bash
ctx pack create feature-auth
ctx pack add feature-auth 'glob:src/auth/**/*.rs'
ctx pack add feature-auth 'glob:tests/auth_*.rs'
ctx pack add feature-auth 'git:diff --base=main'
```

### Code Review Pack
```bash
ctx pack create review
ctx pack add review 'git:diff --base=main'
ctx pack add review file:CONTRIBUTING.md
```

## Pack Strategies by Project Type

**Rust:**
```bash
ctx pack add <pack> file:Cargo.toml
ctx pack add <pack> 'glob:src/lib.rs'
ctx pack add <pack> 'glob:src/**/mod.rs'
```

**TypeScript:**
```bash
ctx pack add <pack> file:package.json
ctx pack add <pack> file:tsconfig.json
ctx pack add <pack> 'glob:src/**/*.ts'
```

**Python:**
```bash
ctx pack add <pack> file:pyproject.toml
ctx pack add <pack> 'glob:src/**/*.py'
```

**Go:**
```bash
ctx pack add <pack> file:go.mod
ctx pack add <pack> 'glob:cmd/**/*.go'
ctx pack add <pack> 'glob:internal/**/*.go'
```

## Best Practices

1. **Start small** - Begin with essential files, add more as needed
2. **Use descriptive names** - `auth-service` not `pack1`
3. **Set appropriate budgets** - Large codebases need higher limits
4. **Preview before use** - `ctx pack preview <name> --tokens`
5. **Quote glob patterns** - Shell expands unquoted globs
6. **Exclude generated files** - `node_modules`, `target`, `dist`

## Token Budget Guidelines

| Use Case | Suggested Budget |
|----------|-----------------|
| Quick question | 10,000 - 20,000 |
| Single file focus | 20,000 - 50,000 |
| Feature work | 50,000 - 100,000 |
| Architecture review | 100,000 - 150,000 |
| Large codebase | 150,000 - 200,000 |

## MCP Integration

### Quick Setup (Recommended)
```bash
# Add ctx as MCP server using stdio transport
claude mcp add ctx -- ctx mcp --stdio

# Verify
claude mcp list
```

Then ask Claude: "List my ctx packs" or "Preview the auth pack"

### HTTP Transport
For MCP Inspector or other tools:
```bash
ctx mcp --port 17373
claude mcp add --transport http ctx http://127.0.0.1:17373
```

### Available MCP Tools
- `ctx_packs_list` - List all packs
- `ctx_packs_get` - Get pack details
- `ctx_packs_preview` - Preview rendered content
- `ctx_packs_create` - Create new packs
- `ctx_packs_add_artifact` - Add artifacts to packs
- `ctx_packs_delete` - Delete packs

### Troubleshooting
```bash
# Test HTTP server
curl -X POST http://127.0.0.1:17373 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","id":1,"method":"ping","params":{}}'
# Should return: {"jsonrpc":"2.0","id":1,"result":{}}
```

## File Locations

- **Config:** `~/.ctx/config.toml`
- **Database:** `~/.local/share/com.ctx.ctx/state.db`
- **Blobs:** `~/.local/share/com.ctx.ctx/blobs/`

## Security

- **Redaction**: Secrets automatically redacted (API keys, tokens, private keys)
- **Denylist**: Sensitive files blocked by default (`.env`, `*.pem`, etc.)
- **Preview**: Always review packs before sharing
