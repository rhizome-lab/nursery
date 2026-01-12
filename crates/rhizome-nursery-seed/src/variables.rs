//! Variable resolution from multiple sources.

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use serde::Deserialize;

/// Source of variable values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VariableSource {
    /// From CLI --var flag.
    Cli,
    /// From global config file.
    Config,
    /// From seed defaults.
    SeedDefault,
    /// Inferred from environment.
    Inferred,
}

/// Resolves variables from multiple sources with precedence.
#[derive(Debug, Default)]
pub struct VariableResolver {
    /// CLI overrides (highest priority).
    cli: HashMap<String, String>,
    /// Global config values.
    config: HashMap<String, String>,
    /// Seed defaults.
    seed_defaults: HashMap<String, String>,
    /// Inferred values (lowest priority).
    inferred: HashMap<String, String>,
}

/// Global config file structure.
#[derive(Debug, Default, Deserialize)]
struct GlobalConfig {
    #[serde(default)]
    variables: HashMap<String, String>,
}

impl VariableResolver {
    /// Create a new resolver.
    pub fn new() -> Self {
        Self::default()
    }

    /// Add CLI variable overrides.
    pub fn with_cli(mut self, vars: HashMap<String, String>) -> Self {
        self.cli = vars;
        self
    }

    /// Load global config from ~/.config/nursery/config.toml.
    pub fn with_global_config(mut self) -> Self {
        if let Some(config_dir) = dirs::config_dir() {
            let config_path = config_dir.join("nursery").join("config.toml");
            if let Ok(config) = load_config(&config_path) {
                self.config = config.variables;
            }
        }
        self
    }

    /// Load global config from a specific path.
    pub fn with_config_file(mut self, path: &Path) -> Self {
        if let Ok(config) = load_config(path) {
            self.config = config.variables;
        }
        self
    }

    /// Add seed default values.
    pub fn with_seed_defaults(mut self, defaults: HashMap<String, Option<String>>) -> Self {
        self.seed_defaults = defaults
            .into_iter()
            .filter_map(|(k, v)| v.map(|val| (k, val)))
            .collect();
        self
    }

    /// Add inferred values from git and environment.
    #[cfg(feature = "infer")]
    pub fn with_inferred(mut self) -> Self {
        self.inferred = infer_variables();
        self
    }

    /// Add inferred values (no-op when feature disabled).
    #[cfg(not(feature = "infer"))]
    pub fn with_inferred(self) -> Self {
        self
    }

    /// Resolve a variable value.
    pub fn get(&self, name: &str) -> Option<(&str, VariableSource)> {
        if let Some(v) = self.cli.get(name) {
            return Some((v.as_str(), VariableSource::Cli));
        }
        if let Some(v) = self.config.get(name) {
            return Some((v.as_str(), VariableSource::Config));
        }
        if let Some(v) = self.seed_defaults.get(name) {
            return Some((v.as_str(), VariableSource::SeedDefault));
        }
        if let Some(v) = self.inferred.get(name) {
            return Some((v.as_str(), VariableSource::Inferred));
        }
        None
    }

    /// Resolve all variables, returning the final values.
    pub fn resolve_all(&self, required: &[String]) -> Result<HashMap<String, String>, String> {
        let mut result = HashMap::new();

        // Collect all known variable names
        let mut all_names: Vec<_> = self
            .cli
            .keys()
            .chain(self.config.keys())
            .chain(self.seed_defaults.keys())
            .chain(self.inferred.keys())
            .chain(required.iter())
            .cloned()
            .collect();
        all_names.sort();
        all_names.dedup();

        for name in all_names {
            if let Some((value, _)) = self.get(&name) {
                result.insert(name, value.to_string());
            } else if required.contains(&name) {
                return Err(name);
            }
        }

        Ok(result)
    }

    /// Get all resolved variables with their sources.
    pub fn all_with_sources(&self) -> Vec<(String, String, VariableSource)> {
        let mut all_names: Vec<_> = self
            .cli
            .keys()
            .chain(self.config.keys())
            .chain(self.seed_defaults.keys())
            .chain(self.inferred.keys())
            .cloned()
            .collect();
        all_names.sort();
        all_names.dedup();

        all_names
            .into_iter()
            .filter_map(|name| {
                self.get(&name)
                    .map(|(value, source)| (name, value.to_string(), source))
            })
            .collect()
    }
}

fn load_config(path: &Path) -> Result<GlobalConfig, ()> {
    let contents = fs::read_to_string(path).map_err(|_| ())?;
    toml::from_str(&contents).map_err(|_| ())
}

/// Infer variables from git config and environment.
#[cfg(feature = "infer")]
fn infer_variables() -> HashMap<String, String> {
    let mut vars = HashMap::new();

    // Try git config for author info
    if let Ok(output) = std::process::Command::new("git")
        .args(["config", "--get", "user.name"])
        .output()
        && output.status.success()
        && let Ok(name) = String::from_utf8(output.stdout)
    {
        vars.insert("author".to_string(), name.trim().to_string());
    }

    if let Ok(output) = std::process::Command::new("git")
        .args(["config", "--get", "user.email"])
        .output()
        && output.status.success()
        && let Ok(email) = String::from_utf8(output.stdout)
    {
        vars.insert("email".to_string(), email.trim().to_string());
    }

    // Environment variables
    if let Ok(user) = std::env::var("USER") {
        vars.entry("author".to_string()).or_insert(user);
    }

    vars
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_takes_precedence() {
        let mut cli = HashMap::new();
        cli.insert("name".to_string(), "from-cli".to_string());

        let mut defaults = HashMap::new();
        defaults.insert("name".to_string(), Some("from-default".to_string()));

        let resolver = VariableResolver::new()
            .with_cli(cli)
            .with_seed_defaults(defaults);

        let (value, source) = resolver.get("name").unwrap();
        assert_eq!(value, "from-cli");
        assert_eq!(source, VariableSource::Cli);
    }

    #[test]
    fn falls_back_to_defaults() {
        let mut defaults = HashMap::new();
        defaults.insert("version".to_string(), Some("1.0.0".to_string()));

        let resolver = VariableResolver::new().with_seed_defaults(defaults);

        let (value, source) = resolver.get("version").unwrap();
        assert_eq!(value, "1.0.0");
        assert_eq!(source, VariableSource::SeedDefault);
    }

    #[test]
    fn missing_variable() {
        let resolver = VariableResolver::new();
        assert!(resolver.get("nonexistent").is_none());
    }

    #[test]
    fn resolve_all_missing_required() {
        let resolver = VariableResolver::new();
        let required = vec!["name".to_string()];
        let err = resolver.resolve_all(&required).unwrap_err();
        assert_eq!(err, "name");
    }
}
