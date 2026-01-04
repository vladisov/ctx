# ctx User Guide: From Install to Snapshot

This guide covers the complete workflow for using `ctx`: installing, creating packs, adding content, and previewing prompts.

## 1. Installation

Install `ctx` globally from the project root:

```bash
cargo install --path crates/ctx-cli
```

Verify it works:
```bash
ctx --version
```

## 2. Creating a Pack

A "Pack" is a group of files with a token budget.

```bash
# Create a pack named "demo" with a 5000 token limit
ctx pack create demo --tokens 5000
```

## 3. Adding Content

You can add files, text, or folders using different schemas.

### Single Files (`file:`)
The default schema. Supports line ranges.
```bash
# Explicit scheme
ctx pack add demo file:Cargo.toml

# Implicit scheme (auto-detects file)
ctx pack add demo Cargo.toml

# With line ranges (for huge files)
ctx pack add demo Cargo.toml --start 10 --end 50

# With Priority (higher = kept when over budget)
ctx pack add demo Cargo.toml --priority 100
```

### Raw Text (`text:`)
Useful for adding instructions, notes, or scratches.
```bash
ctx pack add demo text:"IMPORTANT: This is a release build" --priority 500
```

### Documentation Directories (`md_dir:`)
Convenience for recursively adding all Markdown files in a folder.
```bash
ctx pack add demo md_dir:./docs --recursive
```

### Pattern Matching (`glob:`)
Add multiple files matching a pattern.
```bash
# Add all Rust files in src/
ctx pack add demo glob:"src/**/*.rs" --priority 10

# Add all JSON files
ctx pack add demo glob:"data/*.json"
```

### Git Diffs (`git:`)
Add git diffs for code review context.
```bash
# Diff working tree vs HEAD
ctx pack add demo git:diff

# Diff between branches
ctx pack add demo 'git:diff --base=main --head=feature-branch'
```

## 4. Previewing Context

Before sending to an LLM, check what `ctx` will generate.

```bash
# See what fits in the 5000 token budget
ctx pack preview demo --tokens

# Check for redacted secrets
ctx pack preview demo --redactions
```

## 5. Generating the Prompt (Payload)

Get the final text to paste into ChatGPT/Claude.

```bash
ctx pack preview demo --show-payload
```

## 6. Saving State (Snapshot)

Save a perfect copy of this context for later reproducibility.

```bash
ctx pack snapshot demo --label "v1-release"
```
