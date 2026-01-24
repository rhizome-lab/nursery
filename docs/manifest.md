# Manifest Reference

The `myenv.toml` manifest is the single source of truth for tool configuration. Running `myenv generate` creates per-tool config files from this central manifest.

## Project Section

```toml
[project]
name = "my-project"
version = "0.1.0"
```

Required fields:
- `name` — Project identifier
- `version` — Semantic version

## Variables Section

Define shared values that can be used across tool configs:

```toml
[variables]
assets = "./assets"
build_dir = "./build"
debug = true
```

Use variables with `{{variable_name}}` syntax:

```toml
[siphon]
output = "{{assets}}/raw"

[lotus]
output = "{{build_dir}}/web"
```

## Tool Sections

Each tool gets its own section. myenv validates these against the tool's schema and writes them to the tool's config file.

```toml
[siphon]
source = "./dump/game.exe"
strategy = "gms2"
assets = "{{assets}}/raw"

[dew]
pipeline = "src/pipelines/assets.dew"
input = "{{assets}}/raw"
output = "{{assets}}/processed"

[lotus]
target = "web-wasm"
assets = "{{assets}}/processed"
port = 8080
```

Running `myenv generate` creates:
- `.siphon/config.toml`
- `.dew/config.toml`
- `.lotus/config.toml`

The exact paths and formats are determined by each tool's `--schema` response.

## Tool Integration

Tools tell myenv where their config lives via `<tool> --schema`:

```json
{
  "config_path": ".siphon/config.toml",
  "format": "toml",
  "schema": { ... }
}
```

See [Tool Integration Guide](/tool-integration) for implementation details.

## Example: Full Manifest

```toml
[project]
name = "my-game-port"
version = "0.1.0"

[variables]
assets = "./assets"
source_exe = "./dump/original.exe"

[siphon]
source = "{{source_exe}}"
strategy = "gms2"
assets = "{{assets}}/raw"

[dew]
pipeline = "src/pipelines/main.dew"
input = "{{assets}}/raw"
output = "{{assets}}/processed"

[resin]
output = "{{assets}}/generated"
seed = 12345

[lotus]
target = "web-wasm"
assets = "{{assets}}/processed"
port = 8080
```
