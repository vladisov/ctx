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
# Instant context: file + related files to clipboard
ctx @ src/auth.rs

# Or create a pack with 5000 token budget
ctx create demo --tokens 5000

# Add some content
ctx add demo file:README.md
ctx add demo 'glob:src/**/*.rs'
ctx add demo 'text:Focus on error handling'

# Preview before using
ctx preview demo --tokens

# Copy to clipboard
ctx cp demo
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
ctx add demo file:Cargo.toml           # Single file
ctx add demo Cargo.toml                # Implicit scheme
ctx add demo Cargo.toml --start 10 --end 50  # Line range
ctx add demo Cargo.toml --priority 100       # Higher = kept first
```

### Text (`text:`)
```bash
ctx add demo 'text:Focus on error handling' --priority 500
```

### Glob Patterns (`glob:`)
```bash
ctx add demo 'glob:src/**/*.rs'        # All Rust files
ctx add demo 'glob:docs/**/*.md'       # All markdown in docs
ctx add demo 'glob:data/*.json'        # JSON files in data/
```

### Git Diffs (`git:`)
```bash
ctx add demo git:diff                  # Working tree vs HEAD
ctx add demo 'git:diff --base=main'    # Diff against main
ctx add demo 'git:diff --base=HEAD~3'  # Last 3 commits
```

### Markdown Directories (`md_dir:`)
```bash
ctx add demo md_dir:./docs --recursive
```

## CLI Quick Reference

```bash
# Quick workflow
ctx @ <file>                      # File + related to clipboard
ctx @ <file> -o                   # Output to stdout instead
ctx @ <file> -n 10                # Include up to 10 related files

# Pack management
ctx create <name>                 # Create pack (128k budget)
ctx create <name> --tokens 50000  # Custom budget
ctx ls                            # List all packs
ctx show <name>                   # Show pack details
ctx delete <name>                 # Delete pack

# Artifacts
ctx add <pack> <source>           # Add artifact
ctx add <pack> <source> -r        # Add with related files
ctx add <pack> 'glob:src/**/*.rs' # Quote globs!
ctx rm <pack> <artifact-id>       # Remove artifact

# Smart context
ctx suggest <file>                # Find related files
ctx lint <pack>                   # Check for missing deps
ctx lint <pack> --fix             # Auto-add missing deps

# Preview & Export
ctx preview <pack>                # Show stats
ctx preview <pack> --tokens       # Per-artifact tokens
ctx preview <pack> --payload      # Full content
ctx cp <pack>                     # Copy to clipboard

# Project-local packs
ctx init                          # Create ctx.toml
ctx sync                          # Sync from ctx.toml
ctx save <pack>                   # Save to ctx.toml

# Interactive
ctx ui                            # Terminal UI
```

## Project-Local Packs (ctx.toml)

Save pack definitions to version control:

```bash
ctx init                    # Create ctx.toml
ctx save my-pack            # Export pack to ctx.toml
ctx sync                    # Import packs from ctx.toml
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

## Smart Context Selection

ctx can automatically suggest related files based on:
- **Git co-change**: Files frequently modified together in commits
- **Import graph**: Files that import each other (Rust, TypeScript, Python)

### Quick Context
```bash
# File + related files to clipboard in one command
ctx @ src/auth.rs

# Output to stdout instead
ctx @ src/auth.rs --output

# Control number of related files (default: 5)
ctx @ src/auth.rs -n 10
```

### Find Related Files
```bash
ctx suggest src/auth.rs
# Output:
# 1. src/auth/middleware.rs (85%)
# 2. src/auth/tokens.rs (72%)
# 3. tests/auth_test.rs (65%)
```

### Add Files with Related
```bash
# Add a file and automatically include related files
ctx add my-pack file:src/auth.rs --with-related

# Control how many related files to add (default: 5)
ctx add my-pack file:src/auth.rs -r --related-max 10
```

### Check Pack Completeness
```bash
# Find files imported by pack contents but not included
ctx lint my-pack
# Output:
#   Missing dependencies (3):
#     src/utils/hash.rs (imported by 2 file(s))
#     src/config.rs (imported by 1 file(s))

# Auto-fix by adding missing files
ctx lint my-pack --fix
```

## Common Pack Patterns

### Style Guide Pack
```bash
ctx create style-guide
ctx add style-guide file:CONTRIBUTING.md
ctx add style-guide file:.editorconfig
ctx add style-guide 'text:Conventions: use async/await, prefer explicit error handling'
```

### Architecture Pack
```bash
ctx create architecture
ctx add architecture file:README.md
ctx add architecture 'glob:src/lib.rs'
ctx add architecture 'glob:**/mod.rs'
ctx add architecture file:Cargo.toml
```

### Feature Context Pack
```bash
ctx create feature-auth
ctx add feature-auth 'glob:src/auth/**/*.rs'
ctx add feature-auth 'glob:tests/auth_*.rs'
ctx add feature-auth 'git:diff --base=main'
```

### Code Review Pack
```bash
ctx create review
ctx add review 'git:diff --base=main'
ctx add review file:CONTRIBUTING.md
```

## Pack Strategies by Project Type

**Rust:**
```bash
ctx add <pack> file:Cargo.toml
ctx add <pack> 'glob:src/lib.rs'
ctx add <pack> 'glob:src/**/mod.rs'
```

**TypeScript:**
```bash
ctx add <pack> file:package.json
ctx add <pack> file:tsconfig.json
ctx add <pack> 'glob:src/**/*.ts'
```

**Python:**
```bash
ctx add <pack> file:pyproject.toml
ctx add <pack> 'glob:src/**/*.py'
```

**Go:**
```bash
ctx add <pack> file:go.mod
ctx add <pack> 'glob:cmd/**/*.go'
ctx add <pack> 'glob:internal/**/*.go'
```

## Best Practices

1. **Start small** - Begin with essential files, add more as needed
2. **Use descriptive names** - `auth-service` not `pack1`
3. **Set appropriate budgets** - Large codebases need higher limits
4. **Preview before use** - `ctx preview <name> --tokens`
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
