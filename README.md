# Nursery

Rhizome ecosystem configuration manager.

Part of the [Rhizome](https://rhizome-lab.github.io) ecosystem.

## Overview

Nursery generates per-tool config files from a central `nursery.toml` manifest. Tools never read `nursery.toml` directly - they only read their generated native configs.

```
nursery.toml  →  nursery generate  →  .siphon/config.toml
                                  →  .dew/config.toml
                                  →  .spore/config.toml
```

## The Manifest

```toml
# nursery.toml
[project]
name = "my-project"
version = "0.1.0"

[variables]
assets = "./assets"

[siphon]
source = "./dump/game.exe"
strategy = "gms2"
output = "{{assets}}/raw"

[dew]
pipeline = "src/pipelines/assets.dew"
input = "{{assets}}/raw"
output = "{{assets}}/processed"

[spore]
entry = "scripts/main.lua"
integrations = ["llm", "moss"]
```

## Seeds

Starter templates for common workflows:

- **creation** - New project from scratch
- **archaeology** - Lifting legacy games
- **lab** - Full ecosystem sandbox

## License

MIT
