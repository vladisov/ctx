# ctx Quick Reference

**ctx** creates repeatable context packs for LLM workflows. Use this guide to create appropriate packs for any repository.

## Core Concepts

| Concept | Description |
|---------|-------------|
| **Pack** | Named bundle of sources with a token budget (default 128k) |
| **Artifact** | Single source item (file, glob pattern, git diff, text) |
| **Render** | Combines all artifacts into deterministic output |
| **Snapshot** | Immutable versioned payload for reproducibility |

## Source Types

```bash
file:path/to/file.rs       # Single file
glob:src/**/*.rs           # Pattern matching multiple files
glob:docs/**/*.md          # Documentation files
git:diff --base=main       # Git diff against branch
git:diff --base=HEAD~3     # Git diff against commit
text:Your inline text      # Inline text/instructions
md_dir:docs/               # Directory of markdown files
```

## CLI Quick Reference

```bash
# Pack management
ctx pack create <name>              # Create pack (128k budget)
ctx pack create <name> --tokens 50000  # Custom budget
ctx pack list                       # List all packs
ctx pack show <name>                # Show pack details
ctx pack delete <name>              # Delete pack

# Artifacts
ctx pack add <pack> <source>        # Add artifact
ctx pack add <pack> 'glob:src/**/*.rs'  # Quote globs!
ctx pack remove <pack> <artifact-id>    # Remove artifact

# Preview & Export
ctx pack preview <pack>             # Show stats
ctx pack preview <pack> --tokens    # Show per-artifact tokens
ctx pack preview <pack> --show-payload  # Show full content
ctx pack snapshot <pack> --label "v1"   # Create snapshot

# Interactive
ctx tui                             # Terminal UI
```

## Common Pack Patterns

### 1. Style Guide Pack
For coding standards and conventions:
```bash
ctx pack create style-guide
ctx pack add style-guide file:CONTRIBUTING.md
ctx pack add style-guide file:.editorconfig
ctx pack add style-guide 'text:Conventions: use async/await, prefer explicit error handling'
```

### 2. Architecture Pack
For understanding codebase structure:
```bash
ctx pack create architecture
ctx pack add architecture file:README.md
ctx pack add architecture 'glob:src/lib.rs'        # Entry points
ctx pack add architecture 'glob:**/mod.rs'         # Module structure
ctx pack add architecture file:Cargo.toml          # Dependencies
```

### 3. Feature Context Pack
For working on a specific feature:
```bash
ctx pack create feature-auth
ctx pack add feature-auth 'glob:src/auth/**/*.rs'
ctx pack add feature-auth 'glob:tests/auth_*.rs'
ctx pack add feature-auth 'git:diff --base=main'
```

### 4. API Documentation Pack
```bash
ctx pack create api-docs
ctx pack add api-docs 'glob:docs/api/**/*.md'
ctx pack add api-docs file:openapi.yaml
```

### 5. Test Context Pack
```bash
ctx pack create tests
ctx pack add tests 'glob:tests/**/*.rs'
ctx pack add tests 'glob:src/**/tests.rs'
ctx pack add tests file:pytest.ini              # or test config
```

### 6. Current Changes Pack
For code review or understanding recent work:
```bash
ctx pack create changes
ctx pack add changes 'git:diff --base=main'
ctx pack add changes 'git:diff --base=HEAD~5'
```

## Pack Creation Strategy

### By Repository Type

**Rust Project:**
```bash
ctx pack create rust-context
ctx pack add rust-context file:Cargo.toml
ctx pack add rust-context 'glob:src/lib.rs'
ctx pack add rust-context 'glob:src/main.rs'
ctx pack add rust-context 'glob:src/**/mod.rs'
```

**TypeScript/Node Project:**
```bash
ctx pack create ts-context
ctx pack add ts-context file:package.json
ctx pack add ts-context file:tsconfig.json
ctx pack add ts-context 'glob:src/index.ts'
ctx pack add ts-context 'glob:src/**/*.ts' --exclude '**/*.test.ts'
```

**Python Project:**
```bash
ctx pack create py-context
ctx pack add py-context file:pyproject.toml
ctx pack add py-context file:requirements.txt
ctx pack add py-context 'glob:src/**/*.py'
ctx pack add py-context 'glob:**/__init__.py'
```

**Go Project:**
```bash
ctx pack create go-context
ctx pack add go-context file:go.mod
ctx pack add go-context 'glob:cmd/**/*.go'
ctx pack add go-context 'glob:internal/**/*.go'
```

### By Task Type

**Bug Fix:** Include failing tests + relevant source
```bash
ctx pack create bugfix
ctx pack add bugfix 'glob:tests/test_failing.py'
ctx pack add bugfix 'glob:src/module/*.py'
ctx pack add bugfix 'git:diff --base=main'
```

**New Feature:** Include similar features + interfaces
```bash
ctx pack create new-feature
ctx pack add new-feature 'glob:src/features/similar/**/*'
ctx pack add new-feature 'glob:src/interfaces/*.ts'
ctx pack add new-feature file:docs/ARCHITECTURE.md
```

**Refactoring:** Include full module + tests
```bash
ctx pack create refactor
ctx pack add refactor 'glob:src/legacy/**/*'
ctx pack add refactor 'glob:tests/legacy/**/*'
ctx pack add refactor 'text:Goal: modernize to use async patterns'
```

**Code Review:** Include changes + context
```bash
ctx pack create review
ctx pack add review 'git:diff --base=main'
ctx pack add review file:CONTRIBUTING.md
```

## Best Practices

1. **Start small** - Begin with essential files, add more as needed
2. **Use descriptive names** - `auth-service` not `pack1`
3. **Set appropriate budgets** - Large codebases need higher limits
4. **Preview before use** - `ctx pack preview <name> --tokens`
5. **Quote glob patterns** - Shell expands unquoted globs
6. **Exclude generated files** - `node_modules`, `target`, `dist`
7. **Include context** - Add READMEs and docs for understanding
8. **Separate concerns** - One pack per feature/module/task

## Token Budget Guidelines

| Use Case | Suggested Budget |
|----------|-----------------|
| Quick question | 10,000 - 20,000 |
| Single file focus | 20,000 - 50,000 |
| Feature work | 50,000 - 100,000 |
| Architecture review | 100,000 - 150,000 |
| Large codebase | 150,000 - 200,000 |

## MCP Integration

Connect ctx to Claude Code:
```bash
# Add ctx as MCP server (stdio transport - recommended)
claude mcp add ctx -- ctx mcp --stdio

# Verify connection
claude mcp list
```

Then ask Claude: "List my ctx packs" or "Preview the X pack"

**HTTP transport** (for MCP Inspector or other tools):
```bash
ctx mcp --port 17373
```

## File Locations

- **Config:** `~/.ctx/config.toml`
- **Database:** `~/.local/share/com.ctx.ctx/state.db`
- **Blobs:** `~/.local/share/com.ctx.ctx/blobs/`

## Security Notes

- Secrets are automatically redacted (API keys, tokens, etc.)
- Sensitive files blocked by default (`.env`, `*.pem`, etc.)
- Always preview packs before sharing
