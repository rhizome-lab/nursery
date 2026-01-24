# Introduction

myenv is the orchestration layer for the rhi ecosystem.

## The Problem

The rhi ecosystem contains many tools:
- **Siphon** - Legacy software lifting
- **Dew** - Minimal expression language
- **Lotus** - Persistent world runtime
- **Resin** - Procedural media generation
- **Moss** - Code intelligence
- **Canopy** - Universal UI

Each tool is useful on its own, but combining them requires understanding their interfaces and dependencies. This cognitive load increases with each new tool.

## The Solution

Instead of memorizing arguments for multiple tools, you write one `myenv.toml` manifest:

```toml
[project]
name = "my-project"
version = "0.1.0"

[siphon]
source = "./dump/game.exe"
strategy = "gms2"
assets = "./assets/raw"

[lotus]
target = "web-wasm"
port = 8080
```

myenv reads this manifest and coordinates the tools. You don't need to remember which tool runs first or what flags to pass.

## Seeds

To reduce the "where do I start?" friction, myenv provides **Seeds**—pre-configured starter templates:

- **seed-archaeology** - For lifting legacy games (siphon → lotus)
- **seed-creation** - For new Lotus projects from scratch
- **seed-lab** - Full ecosystem sandbox with all tools configured

## Next Steps

- [Getting Started](/getting-started) - Install and create your first project
- [Manifest Reference](/manifest) - Full `myenv.toml` specification
- [Seeds](/seeds) - Available starter templates
