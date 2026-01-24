# myenv

rhi ecosystem configuration manager.

Part of the [rhi](https://rhi.zone) ecosystem.

## Overview

myenv generates per-tool config files from a central `myenv.toml` manifest. Tools never read `myenv.toml` directly - they only read their generated native configs.

```
myenv.toml  →  myenv generate  →  .siphon/config.toml
                               →  .dew/config.toml
                               →  .spore/config.toml
```

## The Manifest

```toml
# myenv.toml
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
