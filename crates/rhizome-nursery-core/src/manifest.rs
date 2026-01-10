//! Manifest parsing for `nursery.toml`.

use serde::Deserialize;
use std::collections::BTreeMap;
use std::path::Path;

/// A parsed manifest.
#[derive(Debug, Clone)]
pub struct Manifest {
    /// Project metadata.
    pub project: Project,
    /// Shared variables for templating.
    pub variables: BTreeMap<String, toml::Value>,
    /// Tool configurations, keyed by tool name.
    pub tools: BTreeMap<String, toml::Value>,
}

/// Project metadata from the `[project]` section.
#[derive(Debug, Clone, Deserialize)]
pub struct Project {
    /// Project name.
    pub name: String,
    /// Project version.
    pub version: String,
}

/// Errors that can occur when loading a manifest.
#[derive(Debug, thiserror::Error)]
pub enum ManifestError {
    #[error("failed to read manifest: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse manifest: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("missing required [project] section")]
    MissingProject,
}

impl Manifest {
    /// Load a manifest from a file path.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, ManifestError> {
        let contents = std::fs::read_to_string(path)?;
        Self::from_str(&contents)
    }

    /// Parse a manifest from a TOML string.
    pub fn from_str(s: &str) -> Result<Self, ManifestError> {
        let mut table: toml::Table = toml::from_str(s)?;

        // Extract and parse the project section
        let project_value = table
            .remove("project")
            .ok_or(ManifestError::MissingProject)?;
        let project: Project = project_value.try_into()?;

        // Extract variables section (optional)
        let variables = table
            .remove("variables")
            .and_then(|v| v.as_table().cloned())
            .map(|t| t.into_iter().map(|(k, v)| (k, v)).collect())
            .unwrap_or_default();

        // Everything else is a tool section
        let tools = table.into_iter().collect();

        Ok(Self {
            project,
            variables,
            tools,
        })
    }

    /// Get a variable value as a string.
    pub fn get_variable(&self, name: &str) -> Option<String> {
        self.variables.get(name).and_then(|v| match v {
            toml::Value::String(s) => Some(s.clone()),
            toml::Value::Integer(i) => Some(i.to_string()),
            toml::Value::Float(f) => Some(f.to_string()),
            toml::Value::Boolean(b) => Some(b.to_string()),
            _ => None,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_minimal_manifest() {
        let toml = r#"
            [project]
            name = "test"
            version = "0.1.0"
        "#;

        let manifest = Manifest::from_str(toml).unwrap();
        assert_eq!(manifest.project.name, "test");
        assert_eq!(manifest.project.version, "0.1.0");
        assert!(manifest.variables.is_empty());
        assert!(manifest.tools.is_empty());
    }

    #[test]
    fn parse_manifest_with_variables() {
        let toml = r#"
            [project]
            name = "test"
            version = "0.1.0"

            [variables]
            assets = "./assets"
            debug = true
            count = 42
        "#;

        let manifest = Manifest::from_str(toml).unwrap();
        assert_eq!(manifest.get_variable("assets"), Some("./assets".to_string()));
        assert_eq!(manifest.get_variable("debug"), Some("true".to_string()));
        assert_eq!(manifest.get_variable("count"), Some("42".to_string()));
    }

    #[test]
    fn parse_manifest_with_tools() {
        let toml = r#"
            [project]
            name = "test"
            version = "0.1.0"

            [siphon]
            source = "./game.exe"
            strategy = "gms2"

            [dew]
            pipeline = "assets.dew"
        "#;

        let manifest = Manifest::from_str(toml).unwrap();
        assert_eq!(manifest.tools.len(), 2);
        assert!(manifest.tools.contains_key("siphon"));
        assert!(manifest.tools.contains_key("dew"));
    }

    #[test]
    fn missing_project_section() {
        let toml = r#"
            [siphon]
            source = "./game.exe"
        "#;

        let err = Manifest::from_str(toml).unwrap_err();
        assert!(matches!(err, ManifestError::MissingProject));
    }
}
