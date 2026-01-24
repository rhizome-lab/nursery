# myenv Design

myenv is a **configuration manager** for the rhi ecosystem. It generates per-tool config files from a central `myenv.toml` manifest.

## Core Principles

1. **One source of truth** — `myenv.toml` defines all tool configs in one place
2. **Invisible manifest** — Tools never read `myenv.toml` directly, only their generated native configs
3. **Tools stay dumb** — Tools just read their native config files, no special runtime behavior
4. **Validation before generation** — Catch config errors before writing anything
5. **Templating for DRY** — Share variables across tools, with optional Lua upgrade path

## Architecture

```
┌─────────────────┐
│  myenv.toml   │  ← Central manifest
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│     myenv     │  ← Validates & generates
│    generate     │
└────────┬────────┘
         │
         ▼
┌─────────────────────────────────────┐
│  .siphon/config.toml                │
│  .dew/config.toml                   │  ← Per-tool native configs
│  .spore/config.toml                 │
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

[spore]
entry = "scripts/main.lua"
integrations = ["llm", "moss"]
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

### `myenv new <name> [--seed <template>]`

Scaffold a new project from a seed template.

### `myenv generate`

Generate per-tool config files from `myenv.toml`.

1. Parse manifest
2. Expand variables/templates
3. For each tool section:
   - Fetch schema via `<tool> --schema`
   - Validate config against schema
   - Write to `config_path` in correct format

### `myenv validate`

Validate manifest without generating files. Useful for CI.

### `myenv seeds`

List available seed templates.

## Seeds

Project templates stored in `~/.config/myenv/seeds/` or built-in.

```
my-seed/
  seed.toml       # metadata + variables
  template/       # files to scaffold
    myenv.toml
    src/
```

See [Seeds documentation](/seeds) for details.

## Variable Resolution

Variables are resolved with precedence (highest first):

1. CLI `--var key=value`
2. Global config `~/.config/myenv/config.toml`
3. Seed defaults
4. Inferred (git config, environment)

## Templating

Simple `{{variable}}` substitution by default.

Future: Lua scripting for complex logic (via spore runtime).

## Non-Goals

myenv does **not**:

- Run tools — That's spore's job
- Manage tool execution order — That's spore's job
- Install or version tools — Use your package manager
- Replace tool CLIs — Tools work standalone, myenv is optional

## Crate Structure

- `rhizome-myenv-core` — Manifest parsing, validation, generation
- `rhizome-myenv-cli` — CLI binary (`myenv`)
- `rhizome-myenv-seed` — Template scaffolding
