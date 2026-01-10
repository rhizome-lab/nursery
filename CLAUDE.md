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
- **Manage dependencies** — That's spore's job
- **Install tools** — Use your package manager

### Key Concepts

- **`nursery.toml`**: Central manifest defining all tool configs
- **`<tool> --schema`**: Convention for tools to expose their config schema
- **Seeds**: Starter templates for common project types
- **Variables**: Shared values across tool configs

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
