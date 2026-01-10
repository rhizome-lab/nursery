//! Pull tool configs back into manifest format.

use crate::schema::{ConfigFormat, SchemaError, SchemaProvider};
use std::fs;
use std::path::Path;

/// Result of pulling a tool config.
#[derive(Debug)]
pub struct PulledConfig {
    /// Tool name.
    pub tool: String,
    /// Path config was read from.
    pub path: std::path::PathBuf,
    /// Parsed config as TOML value.
    pub config: toml::Value,
}

/// Errors that can occur during pull.
#[derive(Debug, thiserror::Error)]
pub enum PullError {
    #[error("failed to fetch schema for '{0}': {1}")]
    SchemaFetch(String, SchemaError),
    #[error("config file not found for '{tool}': {path}")]
    ConfigNotFound { tool: String, path: String },
    #[error("failed to read config for '{0}': {1}")]
    ReadConfig(String, std::io::Error),
    #[error("failed to parse config for '{0}': {1}")]
    ParseConfig(String, String),
}

/// Pull configs for all tools.
pub fn pull_configs(
    tools: &[String],
    provider: &dyn SchemaProvider,
    base_dir: &Path,
) -> Result<Vec<PulledConfig>, PullError> {
    let mut results = Vec::new();

    for tool_name in tools {
        let result = pull_tool_config(tool_name, provider, base_dir)?;
        results.push(result);
    }

    Ok(results)
}

/// Pull config for a single tool.
fn pull_tool_config(
    tool_name: &str,
    provider: &dyn SchemaProvider,
    base_dir: &Path,
) -> Result<PulledConfig, PullError> {
    // Fetch schema to know where config lives
    let schema = provider
        .fetch(tool_name)
        .map_err(|e| PullError::SchemaFetch(tool_name.to_string(), e))?;

    let config_path = base_dir.join(&schema.config_path);

    if !config_path.exists() {
        return Err(PullError::ConfigNotFound {
            tool: tool_name.to_string(),
            path: config_path.display().to_string(),
        });
    }

    let contents = fs::read_to_string(&config_path)
        .map_err(|e| PullError::ReadConfig(tool_name.to_string(), e))?;

    let config = parse_config(&contents, schema.format, tool_name)?;

    Ok(PulledConfig {
        tool: tool_name.to_string(),
        path: config_path,
        config,
    })
}

/// Parse config from string based on format.
fn parse_config(contents: &str, format: ConfigFormat, tool_name: &str) -> Result<toml::Value, PullError> {
    match format {
        ConfigFormat::Toml => {
            toml::from_str(contents)
                .map_err(|e| PullError::ParseConfig(tool_name.to_string(), e.to_string()))
        }
        ConfigFormat::Json => {
            let json: serde_json::Value = serde_json::from_str(contents)
                .map_err(|e| PullError::ParseConfig(tool_name.to_string(), e.to_string()))?;
            Ok(json_to_toml(&json))
        }
        ConfigFormat::Yaml => {
            let yaml: serde_json::Value = serde_yaml::from_str(contents)
                .map_err(|e| PullError::ParseConfig(tool_name.to_string(), e.to_string()))?;
            Ok(json_to_toml(&yaml))
        }
    }
}

/// Convert JSON value to TOML value.
fn json_to_toml(value: &serde_json::Value) -> toml::Value {
    match value {
        serde_json::Value::Null => toml::Value::String("null".to_string()),
        serde_json::Value::Bool(b) => toml::Value::Boolean(*b),
        serde_json::Value::Number(n) => {
            if let Some(i) = n.as_i64() {
                toml::Value::Integer(i)
            } else if let Some(f) = n.as_f64() {
                toml::Value::Float(f)
            } else {
                toml::Value::String(n.to_string())
            }
        }
        serde_json::Value::String(s) => toml::Value::String(s.clone()),
        serde_json::Value::Array(arr) => {
            toml::Value::Array(arr.iter().map(json_to_toml).collect())
        }
        serde_json::Value::Object(obj) => {
            let table: toml::map::Map<String, toml::Value> = obj
                .iter()
                .map(|(k, v)| (k.clone(), json_to_toml(v)))
                .collect();
            toml::Value::Table(table)
        }
    }
}

/// Merge pulled configs into a manifest TOML structure.
pub fn merge_to_manifest(
    pulled: &[PulledConfig],
    existing: Option<&str>,
) -> Result<String, PullError> {
    let mut table: toml::Table = if let Some(s) = existing {
        toml::from_str(s).unwrap_or_default()
    } else {
        toml::Table::new()
    };

    // Ensure project section exists
    if !table.contains_key("project") {
        let mut project = toml::Table::new();
        project.insert("name".to_string(), toml::Value::String("my-project".to_string()));
        project.insert("version".to_string(), toml::Value::String("0.1.0".to_string()));
        table.insert("project".to_string(), toml::Value::Table(project));
    }

    // Update tool sections
    for config in pulled {
        table.insert(config.tool.clone(), config.config.clone());
    }

    Ok(toml::to_string_pretty(&table).unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn json_to_toml_basic() {
        let json = serde_json::json!({
            "name": "test",
            "count": 42,
            "enabled": true
        });

        let toml_val = json_to_toml(&json);
        let table = toml_val.as_table().unwrap();

        assert_eq!(table["name"].as_str(), Some("test"));
        assert_eq!(table["count"].as_integer(), Some(42));
        assert_eq!(table["enabled"].as_bool(), Some(true));
    }

    #[test]
    fn merge_creates_project() {
        let pulled = vec![PulledConfig {
            tool: "mytool".to_string(),
            path: ".mytool/config.toml".into(),
            config: toml::toml! {
                source = "./input"
            }.into(),
        }];

        let result = merge_to_manifest(&pulled, None).unwrap();
        assert!(result.contains("[project]"));
        assert!(result.contains("[mytool]"));
        assert!(result.contains("source = \"./input\""));
    }

    #[test]
    fn merge_preserves_existing() {
        let existing = r#"
[project]
name = "existing"
version = "1.0.0"

[variables]
foo = "bar"
"#;

        let pulled = vec![PulledConfig {
            tool: "mytool".to_string(),
            path: ".mytool/config.toml".into(),
            config: toml::toml! {
                source = "./input"
            }.into(),
        }];

        let result = merge_to_manifest(&pulled, Some(existing)).unwrap();
        assert!(result.contains("name = \"existing\""));
        assert!(result.contains("foo = \"bar\""));
        assert!(result.contains("[mytool]"));
    }
}
