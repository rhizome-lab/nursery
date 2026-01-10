# Nursery Design

Nursery is a **configuration manager** for the Rhizome ecosystem. It generates per-tool config files from a central `nursery.toml` manifest.

## Core Principles

1. **One source of truth** — `nursery.toml` defines all tool configs in one place
2. **Tools stay dumb** — Tools just read their native config files, no special runtime behavior
3. **Validation before generation** — Catch config errors before writing anything
4. **Templating for DRY** — Share variables across tools, with optional Lua upgrade path

## Architecture

```
┌─────────────────┐
│  nursery.toml   │  ← Central manifest
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│     nursery     │  ← Validates & generates
│    generate     │
└────────┬────────┘
         │
         ▼
┌─────────────────────────────────────┐
│  .siphon/config.toml                │
│  .dew/config.toml                   │  ← Per-tool native configs
│  .lotus/config.toml                 │
└─────────────────────────────────────┘
         │
         ▼
┌─────────────────┐
│      spore      │  ← Runs tools (separate project)
└─────────────────┘
```

## Manifest Format

```toml
[project]
name = "my-game"
version = "0.1.0"

[variables]
assets_dir = "./assets"

[siphon]
source = "./dump/game.exe"
strategy = "gms2"
assets = "{{assets_dir}}/raw"

[dew]
pipeline = "src/pipelines/main.dew"
input = "{{assets_dir}}/raw"
output = "{{assets_dir}}/processed"

[lotus]
target = "web-wasm"
assets = "{{assets_dir}}/processed"
port = 8080
```

## Tool Schema Convention

Tools expose their config schema via `<tool> --schema`:

```json
{
  "config_path": ".siphon/config.toml",
  "format": "toml",
  "schema": {
    "type": "object",
    "properties": {
      "source": { "type": "string" },
      "strategy": { "type": "string" },
      "assets": { "type": "string" }
    },
    "required": ["source", "strategy"]
  }
}
```

Fields:
- `config_path` — Where the tool reads its config
- `format` — `toml`, `json`, or `yaml`
- `schema` — JSON Schema for validation

## CLI Commands

### `nursery new <name> [--seed <template>]`

Scaffold a new project from a seed template.

### `nursery generate`

Generate per-tool config files from `nursery.toml`.

1. Parse manifest
2. Expand variables/templates
3. For each tool section:
   - Fetch schema via `<tool> --schema`
   - Validate config against schema
   - Write to `config_path` in correct format

### `nursery validate`

Validate manifest without generating files. Useful for CI.

### `nursery seeds`

List available seed templates.

## Seeds

Project templates stored in `~/.config/nursery/seeds/` or built-in.

```
my-seed/
  seed.toml       # metadata + variables
  template/       # files to scaffold
    nursery.toml
    src/
```

See [Seeds documentation](/seeds) for details.

## Variable Resolution

Variables are resolved with precedence (highest first):

1. CLI `--var key=value`
2. Global config `~/.config/nursery/config.toml`
3. Seed defaults
4. Inferred (git config, environment)

## Templating

Simple `{{variable}}` substitution by default.

Future: Lua scripting for complex logic (via spore runtime).

## Non-Goals

Nursery does **not**:

- Run tools — That's spore's job
- Manage dependencies between tools — That's spore's job
- Install or version tools — Use your package manager
- Replace tool CLIs — Tools work standalone, nursery is optional

## Crate Structure

- `rhizome-nursery-core` — Manifest parsing, validation, generation
- `rhizome-nursery-cli` — CLI binary (`nursery`)
- `rhizome-nursery-seed` — Template scaffolding
