# CLAUDE.md

Behavioral rules for Claude Code in this repository.

## Overview

Nursery is the orchestration layer for the Rhizome ecosystem. It provides project scaffolding, manifest-based configuration, and tool coordination.

### Key Concepts

- **`rhizome.toml`**: The manifest file that defines how Rhizome tools compose for a specific project
- **Seeds**: Starter templates for common project types (archaeology, creation, lab)
- **Orchestration**: Coordinating multiple tools (winnow, sap, lotus, etc.) based on the manifest

### The Manifest

```toml
# rhizome.toml
[project]
name = "my-project"
version = "0.1.0"

[winnow]
source = "./dump/game.exe"
strategy = "gms2"
assets = "./assets/raw"

[sap]
pipeline = "src/pipelines/assets.sap"

[lotus]
target = "web-wasm"
port = 8080
```

The manifest is the single source of truth for how tools interact in a project.

### Seeds

Pre-configured starter templates:
- **seed-archaeology**: For lifting legacy games (winnow → sap → lotus)
- **seed-creation**: For new lotus projects
- **seed-lab**: Full ecosystem sandbox

## Core Rule

**Note things down immediately:**
- Bugs/issues → fix or add to TODO.md
- Design decisions → docs/ or code comments
- Future work → TODO.md
- Key insights → this file

**Triggers:** User corrects you, 2+ failed attempts, "aha" moment, framework quirk discovered → document before proceeding.

**Do the work properly.** When asked to analyze X, actually read X - don't synthesize from conversation.

## Design Principles

**Manifest-driven.** The `rhizome.toml` file is the single source of truth. Don't require CLI flags for things that belong in the manifest.

**Lazy discovery.** Only inspect tools that are referenced in the manifest. Don't require all tools to be installed.

**Fail informatively.** When a tool is missing or misconfigured, show exactly what's wrong and how to fix it.

**No magic.** The manifest should be readable by humans. Avoid implicit behavior that can't be understood from reading the file.

## Negative Constraints

Do not:
- Announce actions ("I will now...") - just do them
- Leave work uncommitted
- Create special cases - design to avoid them
- Add implicit dependencies between tools
- Require tools that aren't referenced in the manifest
- Hide configuration in environment variables

## Crate Structure

All crates use the `rhizome-nursery-` prefix:
- `rhizome-nursery-core` - Manifest parsing and validation
- `rhizome-nursery-cli` - CLI binary (named `nursery`)
- `rhizome-nursery-seed` - Template scaffolding
