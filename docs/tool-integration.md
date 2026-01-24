# Tool Integration Guide

This guide explains how to make your tool work with myenv.

## Overview

myenv generates per-tool config files from a central `myenv.toml`. Tools integrate via one convention:

**`<tool> --schema`** — Returns config metadata and JSON Schema

That's it. Tools read their own config files normally. No special runtime behavior needed.

## Schema Convention

When invoked with `--schema`, your tool prints JSON describing:

- `config_path` — Where the tool expects its config file
- `format` — Config format (`toml`, `json`, or `yaml`)
- `schema` — JSON Schema for validation

```bash
$ mytool --schema
{
  "config_path": ".mytool/config.toml",
  "format": "toml",
  "schema": {
    "$schema": "https://json-schema.org/draft/2020-12/schema",
    "type": "object",
    "properties": {
      "source": {
        "type": "string",
        "description": "Path to input file"
      },
      "output": {
        "type": "string",
        "description": "Path to output directory"
      },
      "verbose": {
        "type": "boolean",
        "default": false
      }
    },
    "required": ["source", "output"]
  }
}
```

## Rust Examples

### Deriving from existing config struct

If you already have a config struct, just add `JsonSchema`:

```rust
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct Config {
    /// Path to input file
    source: String,
    /// Path to output directory
    output: String,
    /// Enable verbose output
    #[serde(default)]
    verbose: bool,
}
```

Then expose it via `--schema`:

```rust
fn main() {
    let args: Vec<String> = std::env::args().collect();

    if args.get(1).map(|s| s.as_str()) == Some("--schema") {
        let schema_response = serde_json::json!({
            "config_path": ".mytool/config.toml",
            "format": "toml",
            "schema": schemars::schema_for!(Config)
        });
        println!("{}", serde_json::to_string_pretty(&schema_response).unwrap());
        return;
    }

    // Normal execution - just read your config file
    let config: Config = toml::from_str(
        &std::fs::read_to_string(".mytool/config.toml").expect("config not found")
    ).expect("invalid config");

    // Do work...
}
```

### With clap

```rust
use clap::{Parser, Subcommand};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

#[derive(Parser)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand)]
enum Command {
    /// Output config schema for myenv
    Schema,
}

#[derive(Debug, Deserialize, Serialize, JsonSchema)]
struct Config {
    source: String,
    output: String,
    #[serde(default)]
    verbose: bool,
}

fn main() {
    let cli = Cli::parse();

    if let Some(Command::Schema) = cli.command {
        let response = serde_json::json!({
            "config_path": ".mytool/config.toml",
            "format": "toml",
            "schema": schemars::schema_for!(Config)
        });
        println!("{}", serde_json::to_string_pretty(&response).unwrap());
        return;
    }

    let config: Config = toml::from_str(
        &std::fs::read_to_string(".mytool/config.toml").unwrap()
    ).unwrap();

    // Do work...
}
```

## Python Example

```python
import json
import sys
import tomllib  # Python 3.11+

SCHEMA_RESPONSE = {
    "config_path": ".mytool/config.toml",
    "format": "toml",
    "schema": {
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type": "object",
        "properties": {
            "source": {"type": "string", "description": "Path to input file"},
            "output": {"type": "string", "description": "Path to output directory"},
            "verbose": {"type": "boolean", "default": False},
        },
        "required": ["source", "output"],
    },
}

if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "--schema":
        print(json.dumps(SCHEMA_RESPONSE, indent=2))
        sys.exit(0)

    # Normal execution - read config file
    with open(".mytool/config.toml", "rb") as f:
        config = tomllib.load(f)

    # Do work...
```

## Go Example

```go
package main

import (
    "encoding/json"
    "fmt"
    "os"

    "github.com/BurntSushi/toml"
)

type Config struct {
    Source  string `toml:"source"`
    Output  string `toml:"output"`
    Verbose bool   `toml:"verbose"`
}

var schemaResponse = map[string]any{
    "config_path": ".mytool/config.toml",
    "format":      "toml",
    "schema": map[string]any{
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "type":    "object",
        "properties": map[string]any{
            "source":  map[string]any{"type": "string"},
            "output":  map[string]any{"type": "string"},
            "verbose": map[string]any{"type": "boolean", "default": false},
        },
        "required": []string{"source", "output"},
    },
}

func main() {
    if len(os.Args) > 1 && os.Args[1] == "--schema" {
        enc := json.NewEncoder(os.Stdout)
        enc.SetIndent("", "  ")
        enc.Encode(schemaResponse)
        return
    }

    // Normal execution - read config file
    var config Config
    if _, err := toml.DecodeFile(".mytool/config.toml", &config); err != nil {
        fmt.Fprintln(os.Stderr, err)
        os.Exit(1)
    }

    // Do work...
}
```

## Manifest Example

With your tool integrated, users add it to `myenv.toml`:

```toml
[project]
name = "my-project"
version = "0.1.0"

[mytool]
source = "./input.bin"
output = "./out"
verbose = true
```

Then run:

```bash
myenv generate
```

myenv will:
1. Fetch schema via `mytool --schema`
2. Validate the `[mytool]` section
3. Write `.mytool/config.toml` in the correct format

Your tool just reads its config file as usual.

## Templating

myenv supports variable substitution in configs:

```toml
[project]
name = "my-project"

[mytool]
source = "./{{name}}/input.bin"
output = "./out/{{name}}"
```

Variables are expanded before writing the tool config.

## Testing

```bash
# Check schema is valid
mytool --schema | jq .

# Generate configs
myenv generate

# Verify output
cat .mytool/config.toml

# Run tool normally
mytool
```
