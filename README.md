# ctx

**Repeatable context for LLM workflows**

`ctx` solves context reproducibility for LLM-assisted development:
- Curate exactly what the model sees
- Preview token usage before sending
- Share context setups via `ctx.toml`
- Deterministic rendering (same pack = same output)

## Quick Start

```bash
# Install
cargo install --path crates/ctx-cli

# Quick context: file + related files to clipboard
ctx @ src/auth.rs

# Or build a pack for repeatable context
ctx create my-feature --tokens 50000
ctx add my-feature file:src/auth.rs
ctx add my-feature 'glob:tests/**/*.rs'
ctx add my-feature 'git:diff --base=main'

# Add with related files automatically
ctx add my-feature file:src/auth.rs --with-related

# Check for missing dependencies
ctx lint my-feature --fix

# Preview
ctx preview my-feature --tokens

# Copy to clipboard
ctx cp my-feature

# Or use the interactive TUI
ctx ui
```

## MCP Integration

Connect to Claude Code:
```bash
claude mcp add ctx -- ctx mcp --stdio
```

Then ask: "List my ctx packs" or "Preview the auth pack"

## Project-Local Packs

Share pack definitions via version control:
```bash
ctx init                  # Create ctx.toml
ctx save my-feature       # Export to ctx.toml
ctx sync                  # Import from ctx.toml
```

## Documentation

- **[User Guide & Reference](./CTX_REFERENCE.md)** - Full CLI reference, pack patterns, best practices
- **[Architecture](./ARCHITECTURE.md)** - Technical design and implementation details
- **[Changelog](./CHANGELOG.md)** - Version history

## Development

```bash
cargo build --release
cargo test
./tests/integration_test.sh
```

See [ARCHITECTURE.md](./ARCHITECTURE.md) for detailed development info.

## License

MIT OR Apache-2.0
