# CLAUDE.md

Behavioral rules for Claude Code in this repository.

## Overview

Nursery is a **configuration manager** for the Rhizome ecosystem. It generates per-tool config files from a central `nursery.toml` manifest.

### What Nursery Does

- **Generate** — `nursery.toml` → per-tool native configs
- **Validate** — Check configs against tool schemas before writing
- **Template** — Variable substitution, shared logic across tools
- **Scaffold** — Create new projects from seed templates

### What Nursery Does NOT Do

- **Run tools** — That's spore's job
- **Manage tool execution order** — That's spore's job
- **Install tools** — Use your package manager

### Key Concepts

- **`nursery.toml`**: Central manifest defining all tool configs (invisible to tools at runtime)
- **`<tool> --schema`**: Convention for tools to expose their config schema
- **Seeds**: Starter templates for common project types
- **Variables**: Shared values across tool configs

### The Invisible Manifest

`nursery.toml` is the **source of truth** but is **invisible at runtime**. Tools never read it directly - they only read their generated native configs.

```
nursery.toml  →  nursery generate  →  .spore/config.toml
                                  →  .siphon/config.toml
                                  →  .dew/config.toml
```

This keeps tools simple and decoupled from nursery.

### The Manifest

```toml
# nursery.toml
[project]
name = "my-project"
version = "0.1.0"

[variables]
assets = "./assets"

[siphon]
source = "./dump/game.exe"
output = "{{assets}}/raw"

[dew]
pipeline = "main.dew"
input = "{{assets}}/raw"
output = "{{assets}}/processed"
```

Running `nursery generate` creates:
- `.siphon/config.toml`
- `.dew/config.toml`

Tools read their native configs. No special runtime behavior.

### Tool Schema Convention

Tools expose config metadata via `--schema`:

```json
{
  "config_path": ".siphon/config.toml",
  "format": "toml",
  "schema": { ... JSON Schema ... }
}
```

## Core Rule

**Note things down immediately:**
- Bugs/issues → fix or add to TODO.md
- Design decisions → docs/ or code comments
- Future work → TODO.md
- Key insights → this file

**Triggers:** User corrects you, 2+ failed attempts, "aha" moment, framework quirk discovered → document before proceeding.

**Do the work properly.** When asked to analyze X, actually read X - don't synthesize from conversation.

## Behavioral Patterns

From ecosystem-wide session analysis:

- **Question scope early:** Before implementing, ask whether it belongs in this crate/module
- **Check consistency:** Look at how similar things are done elsewhere in the codebase
- **Implement fully:** No silent arbitrary caps, incomplete pagination, or unexposed trait methods
- **Name for purpose:** Avoid names that describe one consumer
- **Verify before stating:** Don't assert API behavior or codebase facts without checking

## Design Principles

**Config generation, not orchestration.** Nursery generates configs, spore runs tools.

**Tools stay dumb.** No special nursery conventions at runtime. Tools just read their config files.

**One source of truth.** `nursery.toml` is the single place to configure all tools.

**Validate before write.** Catch errors before generating configs.

**No magic.** The manifest should be readable by humans.

## Negative Constraints

Do not:
- Announce actions ("I will now...") - just do them
- Leave work uncommitted
- Create special cases - design to avoid them
- Add tool execution to nursery - that's spore
- Require tools at nursery runtime (only `--schema` is needed)

## Crate Structure

All crates use the `rhizome-nursery-` prefix:
- `rhizome-nursery-core` - Manifest parsing, validation, config generation
- `rhizome-nursery-cli` - CLI binary (named `nursery`)
- `rhizome-nursery-seed` - Template scaffolding
