//! Seed resolution from multiple sources.

use crate::builtin::builtins;
use crate::{Seed, SeedError};
use serde::Deserialize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Where a seed's files come from.
#[derive(Debug, Clone)]
pub enum SeedSource {
    /// Built-in seed with static file contents.
    Builtin(&'static [(&'static str, &'static str)]),
    /// File-based seed in a directory.
    Directory(PathBuf),
}

/// Resolves seeds from multiple locations.
#[derive(Debug)]
pub struct SeedResolver {
    /// User seeds directory (~/.config/nursery/seeds).
    user_dir: Option<PathBuf>,
}

/// Parsed seed.toml manifest.
#[derive(Debug, Deserialize)]
struct SeedManifest {
    name: String,
    #[serde(default)]
    description: String,
    #[serde(default)]
    variables: HashMap<String, VariableDef>,
}

/// Variable definition in seed.toml.
#[derive(Debug, Deserialize)]
#[serde(untagged)]
enum VariableDef {
    /// Just a default value.
    Default(String),
    /// Full definition with optional default.
    Full { default: Option<String> },
}

impl SeedResolver {
    /// Create a resolver with the default user seeds directory.
    pub fn new() -> Self {
        let user_dir = dirs::config_dir().map(|d| d.join("nursery").join("seeds"));
        Self { user_dir }
    }

    /// Create a resolver with a custom user seeds directory.
    pub fn with_user_dir(user_dir: Option<PathBuf>) -> Self {
        Self { user_dir }
    }

    /// Get a seed by name.
    pub fn get(&self, name: &str) -> Result<Seed, SeedError> {
        // Check user seeds first (higher priority)
        if let Some(seed) = self.get_user_seed(name)? {
            return Ok(seed);
        }

        // Fall back to builtins
        builtins()
            .into_iter()
            .find(|s| s.name == name)
            .ok_or_else(|| SeedError::UnknownSeed(name.to_string()))
    }

    /// List all available seeds.
    pub fn list(&self) -> Result<Vec<Seed>, SeedError> {
        let mut seeds = builtins();

        // Add user seeds, potentially overriding builtins
        if let Some(ref user_dir) = self.user_dir
            && user_dir.exists()
        {
            for entry in fs::read_dir(user_dir).map_err(SeedError::ReadSeed)? {
                let entry = entry.map_err(SeedError::ReadSeed)?;
                let path = entry.path();

                if path.is_dir()
                    && let Some(seed) = self.load_seed_dir(&path)?
                {
                    // Remove any builtin with the same name
                    seeds.retain(|s| s.name != seed.name);
                    seeds.push(seed);
                }
            }
        }

        seeds.sort_by(|a, b| a.name.cmp(&b.name));
        Ok(seeds)
    }

    fn get_user_seed(&self, name: &str) -> Result<Option<Seed>, SeedError> {
        if let Some(ref user_dir) = self.user_dir {
            let seed_dir = user_dir.join(name);
            if seed_dir.exists() {
                return self.load_seed_dir(&seed_dir);
            }
        }
        Ok(None)
    }

    fn load_seed_dir(&self, path: &Path) -> Result<Option<Seed>, SeedError> {
        let manifest_path = path.join("seed.toml");
        if !manifest_path.exists() {
            return Ok(None);
        }

        let contents = fs::read_to_string(&manifest_path).map_err(SeedError::ReadSeed)?;
        let manifest: SeedManifest = toml::from_str(&contents).map_err(SeedError::ParseSeed)?;

        let variables = manifest
            .variables
            .into_iter()
            .map(|(k, v)| {
                let default = match v {
                    VariableDef::Default(s) => Some(s),
                    VariableDef::Full { default } => default,
                };
                (k, default)
            })
            .collect();

        Ok(Some(Seed {
            name: manifest.name,
            description: manifest.description,
            variables,
            source: SeedSource::Directory(path.to_path_buf()),
        }))
    }
}

impl Default for SeedResolver {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_builtin_seed() {
        let resolver = SeedResolver::with_user_dir(None);
        let seed = resolver.get("creation").unwrap();
        assert_eq!(seed.name, "creation");
    }

    #[test]
    fn unknown_seed() {
        let resolver = SeedResolver::with_user_dir(None);
        let err = resolver.get("nonexistent").unwrap_err();
        assert!(matches!(err, SeedError::UnknownSeed(_)));
    }

    #[test]
    fn list_builtins() {
        let resolver = SeedResolver::with_user_dir(None);
        let seeds = resolver.list().unwrap();
        assert_eq!(seeds.len(), 3);
    }
}
