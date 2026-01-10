//! Nursery core: manifest parsing, validation, and config generation.
//!
//! Nursery is a configuration manager. It generates per-tool config files
//! from a central `nursery.toml` manifest.

mod generate;
mod manifest;
mod schema;

pub use generate::{generate_configs, GenerateError, GeneratedConfig};
pub use manifest::{Manifest, ManifestError, Project};
pub use schema::{CliSchemaProvider, ConfigFormat, SchemaError, SchemaProvider, ToolSchema};
