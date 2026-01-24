//! Built-in seed templates.

use crate::{Seed, SeedSource};
use std::collections::HashMap;

/// Get all built-in seeds.
pub fn builtins() -> Vec<Seed> {
    vec![
        Seed {
            name: "creation".to_string(),
            description: "New project from scratch".to_string(),
            variables: default_variables(),
            source: SeedSource::Builtin(CREATION_FILES),
        },
        Seed {
            name: "archaeology".to_string(),
            description: "Lift a legacy game".to_string(),
            variables: default_variables(),
            source: SeedSource::Builtin(ARCHAEOLOGY_FILES),
        },
        Seed {
            name: "lab".to_string(),
            description: "Full ecosystem sandbox".to_string(),
            variables: default_variables(),
            source: SeedSource::Builtin(LAB_FILES),
        },
    ]
}

fn default_variables() -> HashMap<String, Option<String>> {
    let mut vars = HashMap::new();
    // name is required (no default)
    vars.insert("name".to_string(), None);
    // version has a default
    vars.insert("version".to_string(), Some("0.1.0".to_string()));
    vars
}

static CREATION_FILES: &[(&str, &str)] = &[
    (
        "nursery.toml",
        r#"[project]
name = "{{name}}"
version = "{{version}}"

[lotus]
target = "web-wasm"
port = 8080
"#,
    ),
    (".gitignore", "/target\n"),
];

static ARCHAEOLOGY_FILES: &[(&str, &str)] = &[
    (
        "nursery.toml",
        r#"[project]
name = "{{name}}"
version = "{{version}}"

[siphon]
source = "./dump/game.exe"
strategy = "gms2"
assets = "./assets/raw"

[dew]
pipeline = "src/pipelines/assets.dew"

[lotus]
target = "web-wasm"
port = 8080
"#,
    ),
    ("dump/.gitkeep", ""),
    ("assets/raw/.gitkeep", ""),
    ("src/pipelines/assets.dew", "; Asset processing pipeline\n"),
    (".gitignore", "/target\n"),
];

static LAB_FILES: &[(&str, &str)] = &[
    (
        "nursery.toml",
        r#"[project]
name = "{{name}}"
version = "{{version}}"

[siphon]
source = "./dump/game.exe"
strategy = "gms2"
assets = "./assets/raw"

[dew]
pipeline = "src/pipelines/assets.dew"

[resin]
assets = "./assets/generated"

[lotus]
target = "web-wasm"
port = 8080
"#,
    ),
    ("dump/.gitkeep", ""),
    ("assets/raw/.gitkeep", ""),
    ("assets/generated/.gitkeep", ""),
    ("src/pipelines/assets.dew", "; Asset processing pipeline\n"),
    (".gitignore", "/target\n"),
];
