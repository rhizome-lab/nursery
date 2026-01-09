# Nursery

Rhizome ecosystem orchestrator.

Part of the [Rhizome](https://rhizome-lab.github.io) ecosystem.

## Overview

Nursery is the glue that holds the Rhizome ecosystem together. Instead of memorizing arguments for multiple tools, you write one `rhizome.toml` manifest file that describes how they compose.

## The Manifest

```toml
# rhizome.toml
[project]
name = "feral-remake"
version = "0.1.0"

[winnow]
source = "./dump/game.exe"
strategy = "gms2"
assets = "./assets/raw"

[dew]
pipeline = "src/pipelines/assets.dew"

[lotus]
target = "web-wasm"
port = 8080
```

## Seeds

Starter templates for common workflows:

- **seed-archaeology** - Lifting legacy games
- **seed-creation** - New Lotus projects
- **seed-lab** - Full ecosystem sandbox

## License

MIT
