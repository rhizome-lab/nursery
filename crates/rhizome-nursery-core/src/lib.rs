//! Nursery core: manifest parsing, validation, and config generation.
//!
//! Nursery is a configuration manager. It generates per-tool config files
//! from a central `nursery.toml` manifest.

mod generate;
mod manifest;
mod pull;
mod schema;

pub use generate::{generate_configs, preview_configs, ConfigPreview, GenerateError, GeneratedConfig};
pub use manifest::{Manifest, ManifestError, Project};
pub use pull::{merge_to_manifest, pull_configs, PullError, PulledConfig};
pub use schema::{CliSchemaProvider, ConfigFormat, SchemaError, SchemaProvider, ToolSchema};
