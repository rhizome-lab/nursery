//! Nursery core: manifest parsing, validation, and config generation.
//!
//! Nursery is a configuration manager. It generates per-tool config files
//! from a central `nursery.toml` manifest.

mod config;
mod ecosystem;
mod generate;
mod lockfile;
mod manifest;
mod pull;
mod repology;
mod schema;

pub use config::{ToolSource, ToolsConfig, UserConfig};
pub use ecosystem::{Ecosystem, detect_ecosystems, detect_primary_ecosystem, is_installed};
pub use generate::{
    ConfigPreview, GenerateError, GeneratedConfig, generate_configs, preview_configs,
};
pub use lockfile::{LockedPackage, LockedTool, Lockfile, LockfileError};
pub use manifest::{Manifest, ManifestError, Project, ToolDep};
pub use pull::{PullError, PulledConfig, merge_to_manifest, pull_configs};
pub use repology::{PackageInfo, RepologyClient, RepologyError, ToolInfo};
pub use schema::{CliSchemaProvider, ConfigFormat, SchemaError, SchemaProvider, ToolSchema};
