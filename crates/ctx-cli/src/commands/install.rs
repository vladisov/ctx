use crate::cli::InstallTarget;
use anyhow::Result;
use std::fs;
use std::path::{Path, PathBuf};

pub async fn handle(targets: Vec<InstallTarget>) -> Result<()> {
    // Shared SKILL.md template
    let skill_content = r#"---
description: Manage context packs and relevant file curation for AI Context.
---
# Context Management (ctx)

The `ctx` tool allows you to create, manage, and render "packs" of context (files, diffs, docs) to ensure the AI has exactly the information it needs without exceeding token limits.

## Common Workflows

### 1. Creating a Context Pack for a Task
When starting a new complex task (e.g., "Refactor Authentication"), create a pack to hold relevant files.

```bash
# Create a new pack with a token budget (default 128k)
ctx create auth-refactor

# Add key files
ctx add auth-refactor file:src/auth.rs
ctx add auth-refactor file:src/user.rs

# Add related files using globs (Use QUOTES for globs!)
ctx add auth-refactor 'glob:src/auth/**/*.rs'

# Add recent changes to understand current state
ctx add auth-refactor 'git:diff --base=main'
```

### 2. Getting Suggestions
If you are unsure what files are related to the modified file
```bash
# See what files are related to the modified file
ctx suggest src/auth.rs
```

### 3. Using the Context
To "see" the context, you can render it.

```bash
# Preview what is in the pack and token usage
ctx preview auth-refactor

# Render the full content (useful if you need to read it)
ctx packs load --pack auth-refactor
# OR via CLI
ctx preview auth-refactor --payload
```

## Best Practices
- **Always Quote Globs**: `'glob:**/*.rs'` to prevent shell expansion.
- **Use Priorities**: If a file is critical, give it high priority: `ctx add mypack file:important.rs --priority 100`.
- **Check Budget**: Use `ctx preview` to ensure you are within limits.

### 4. Checking Completeness
Ensure your pack isn't missing important files that are imported by the code you added.

```bash
# Check for missing dependencies
ctx lint auth-refactor

# Automatically add missing files
ctx lint auth-refactor --fix
```

### 5. Persistence (Sharing Packs)
To share packs with your team or persist them across sessions:

```bash
# Create ctx.toml in the current directory if it doesn't exist
ctx init

# Save your pack to ctx.toml
ctx save auth-refactor

# Import packs from ctx.toml (useful when starting work)
ctx sync
```
"#;

    for target in targets {
        match target {
            InstallTarget::Claude => {
                install_to_dir("Claude Code", "~/.claude/skills/ctx", skill_content)?
            }
            InstallTarget::Opencode => {
                install_to_dir("OpenCode", "~/.config/opencode/skills/ctx", skill_content)?
            }
            InstallTarget::Antigravity => install_to_dir(
                "Antigravity",
                "~/.gemini/antigravity/skills/ctx",
                skill_content,
            )?,
        }
    }
    Ok(())
}

fn install_to_dir(name: &str, path_str: &str, content: &str) -> Result<()> {
    println!("📦 Installing for {}...", name);

    // Expand ~ manually since std::process does not do it
    let path_str = if path_str.starts_with("~/") {
        let home =
            dirs::home_dir().ok_or_else(|| anyhow::anyhow!("Could not find home directory"))?;
        let mut p = home.to_string_lossy().to_string();
        p.push_str(&path_str[1..]);
        p
    } else {
        path_str.to_string()
    };

    let path = PathBuf::from(path_str);

    if !path.exists() {
        fs::create_dir_all(&path)?;
    }

    let target_file = path.join("SKILL.md");
    fs::write(&target_file, content)?;

    println!("   ✅ Created skill at: {:?}", target_file);
    Ok(())
}
