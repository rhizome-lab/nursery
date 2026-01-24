# CLAUDE.md

Behavioral rules for Claude Code in this repository.

## Overview

myenv is a **configuration manager** for the rhi ecosystem. It generates per-tool config files from a central `myenv.toml` manifest.

### What myenv Does

- **Generate** — `myenv.toml` → per-tool native configs
- **Validate** — Check configs against tool schemas before writing
- **Template** — Variable substitution, shared logic across tools
- **Scaffold** — Create new projects from seed templates

### What myenv Does NOT Do

- **Run tools** — That's spore's job
- **Manage tool execution order** — That's spore's job
- **Install tools** — Use your package manager

### Key Concepts

- **`myenv.toml`**: Central manifest defining all tool configs (invisible to tools at runtime)
- **`<tool> --schema`**: Convention for tools to expose their config schema
- **Seeds**: Starter templates for common project types
- **Variables**: Shared values across tool configs

### The Invisible Manifest

`myenv.toml` is the **source of truth** but is **invisible at runtime**. Tools never read it directly - they only read their generated native configs.

```
myenv.toml  →  myenv generate  →  .spore/config.toml
                               →  .siphon/config.toml
                               →  .dew/config.toml
```

This keeps tools simple and decoupled from myenv.

### The Manifest

```toml
# myenv.toml
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

Running `myenv generate` creates:
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

**Config generation, not orchestration.** myenv generates configs, spore runs tools.

**Tools stay dumb.** No special myenv conventions at runtime. Tools just read their config files.

**One source of truth.** `myenv.toml` is the single place to configure all tools.

**Validate before write.** Catch errors before generating configs.

**No magic.** The manifest should be readable by humans.

## Commit Convention

Use conventional commits: `type(scope): message`

Types:
- `feat` - New feature
- `fix` - Bug fix
- `refactor` - Code change that neither fixes a bug nor adds a feature
- `docs` - Documentation only
- `chore` - Maintenance (deps, CI, etc.)
- `test` - Adding or updating tests

Scope is optional but recommended for multi-crate repos.

## Negative Constraints

Do not:
- Announce actions ("I will now...") - just do them
- Leave work uncommitted
- Create special cases - design to avoid them
- Add tool execution to myenv - that's spore
- Require tools at myenv runtime (only `--schema` is needed)
- Use path dependencies in Cargo.toml - causes clippy to stash changes across repos
- Use `--no-verify` - fix the issue or fix the hook
- Assume tools are missing - check if `nix develop` is available for the right environment

## Crate Structure

All crates use the `rhi-myenv-` prefix:
- `rhi-myenv-core` - Manifest parsing, validation, config generation
- `rhi-myenv-cli` - CLI binary (named `myenv`)
- `rhi-myenv-seed` - Template scaffolding
- `rhi-myenv-store` - Content-addressed package store
