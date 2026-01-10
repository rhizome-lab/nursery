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
    /// Tool dependencies from `[tools]` section.
    pub tool_deps: BTreeMap<String, ToolDep>,
    /// Ecosystems to include in lockfile (optional).
    pub ecosystems: Option<Vec<String>>,
    /// Tool configurations (e.g., `[siphon]`, `[dew]`).
    pub tool_configs: BTreeMap<String, toml::Value>,
}

/// A tool dependency specification.
#[derive(Debug, Clone)]
pub struct ToolDep {
    /// Version constraint (e.g., ">=14", "*", "=1.7").
    pub version: String,
    /// Whether this tool is optional.
    pub optional: bool,
}

impl ToolDep {
    /// Parse from a TOML value (either string or table).
    fn from_toml(value: &toml::Value) -> Option<Self> {
        match value {
            // Simple form: ripgrep = ">=14"
            toml::Value::String(version) => Some(Self {
                version: version.clone(),
                optional: false,
            }),
            // Table form: ripgrep = { version = ">=14", optional = true }
            toml::Value::Table(t) => {
                let version = t.get("version")?.as_str()?.to_string();
                let optional = t
                    .get("optional")
                    .and_then(|v| v.as_bool())
                    .unwrap_or(false);
                Some(Self { version, optional })
            }
            _ => None,
        }
    }
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
            .map(|t| t.into_iter().collect())
            .unwrap_or_default();

        // Extract tools section (dependencies, optional)
        let (tool_deps, ecosystems) = if let Some(tools_value) = table.remove("tools") {
            if let Some(tools_table) = tools_value.as_table() {
                let ecosystems = tools_table
                    .get("ecosystems")
                    .and_then(|v| v.as_array())
                    .map(|arr| {
                        arr.iter()
                            .filter_map(|v| v.as_str().map(String::from))
                            .collect()
                    });

                let deps = tools_table
                    .iter()
                    .filter(|(k, _)| *k != "ecosystems")
                    .filter_map(|(k, v)| ToolDep::from_toml(v).map(|dep| (k.clone(), dep)))
                    .collect();

                (deps, ecosystems)
            } else {
                (BTreeMap::new(), None)
            }
        } else {
            (BTreeMap::new(), None)
        };

        // Everything else is a tool config section
        let tool_configs = table.into_iter().collect();

        Ok(Self {
            project,
            variables,
            tool_deps,
            ecosystems,
            tool_configs,
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
        assert!(manifest.tool_deps.is_empty());
        assert!(manifest.tool_configs.is_empty());
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
    fn parse_manifest_with_tool_configs() {
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
        assert_eq!(manifest.tool_configs.len(), 2);
        assert!(manifest.tool_configs.contains_key("siphon"));
        assert!(manifest.tool_configs.contains_key("dew"));
    }

    #[test]
    fn parse_manifest_with_tool_deps() {
        let toml = r#"
            [project]
            name = "test"
            version = "0.1.0"

            [tools]
            ripgrep = ">=14"
            fd = "*"
            jq = { version = "=1.7", optional = true }
        "#;

        let manifest = Manifest::from_str(toml).unwrap();
        assert_eq!(manifest.tool_deps.len(), 3);

        let rg = &manifest.tool_deps["ripgrep"];
        assert_eq!(rg.version, ">=14");
        assert!(!rg.optional);

        let fd = &manifest.tool_deps["fd"];
        assert_eq!(fd.version, "*");

        let jq = &manifest.tool_deps["jq"];
        assert_eq!(jq.version, "=1.7");
        assert!(jq.optional);
    }

    #[test]
    fn parse_manifest_with_ecosystems() {
        let toml = r#"
            [project]
            name = "test"
            version = "0.1.0"

            [tools]
            ecosystems = ["pacman", "nix"]
            ripgrep = ">=14"
        "#;

        let manifest = Manifest::from_str(toml).unwrap();
        assert_eq!(manifest.ecosystems, Some(vec!["pacman".to_string(), "nix".to_string()]));
        assert_eq!(manifest.tool_deps.len(), 1);
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
