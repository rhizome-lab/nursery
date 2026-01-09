# Manifest Reference

The `rhizome.toml` file defines how Rhizome tools compose for a project.

## Project Section

```toml
[project]
name = "my-project"
version = "0.1.0"
```

Required fields:
- `name` - Project identifier
- `version` - Semantic version

## Tool Sections

Each Rhizome tool can have its own section. Only include sections for tools you're using.

### Winnow

```toml
[winnow]
source = "./dump/game.exe"    # Path to legacy binary
strategy = "gms2"             # Extraction strategy
assets = "./assets/raw"       # Output directory for extracted assets
```

### Dew

```toml
[dew]
pipeline = "src/pipelines/assets.dew"  # Pipeline definition file
```

### Lotus

```toml
[lotus]
target = "web-wasm"    # Build target (web-wasm, native)
port = 8080            # Dev server port
```

### Resin

```toml
[resin]
assets = "./assets"    # Asset generation output
```

## Dependency Resolution

Nursery automatically determines tool execution order based on declared inputs and outputs. For example, if `lotus.assets` points to `winnow.assets`, Winnow runs first.
