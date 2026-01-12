//! Config file generation.

use crate::manifest::Manifest;
use crate::schema::{ConfigFormat, SchemaError, SchemaProvider, ToolSchema};
use std::collections::HashMap;
use std::fs;
use std::path::Path;

/// Result of generating a tool config.
#[derive(Debug)]
pub struct GeneratedConfig {
    /// Tool name.
    pub tool: String,
    /// Path where config was written.
    pub path: std::path::PathBuf,
    /// Format used.
    pub format: ConfigFormat,
}

/// Preview of what would be generated (for diff mode).
#[derive(Debug)]
pub struct ConfigPreview {
    /// Tool name.
    pub tool: String,
    /// Path where config would be written.
    pub path: std::path::PathBuf,
    /// Content that would be written.
    pub content: String,
    /// Existing content (if file exists).
    pub existing: Option<String>,
}

/// Errors that can occur during generation.
#[derive(Debug, thiserror::Error)]
pub enum GenerateError {
    #[error("failed to fetch schema for '{0}': {1}")]
    SchemaFetch(String, SchemaError),
    #[error("tool '{tool}' config is invalid:\n{errors}")]
    ValidationFailed { tool: String, errors: String },
    #[error("failed to create directory for '{0}': {1}")]
    CreateDir(String, std::io::Error),
    #[error("failed to write config for '{0}': {1}")]
    WriteConfig(String, std::io::Error),
    #[error("failed to serialize config for '{0}': {1}")]
    Serialize(String, String),
}

/// Generate config files for all tools in the manifest.
pub fn generate_configs(
    manifest: &Manifest,
    provider: &dyn SchemaProvider,
    base_dir: &Path,
) -> Result<Vec<GeneratedConfig>, GenerateError> {
    let mut results = Vec::new();

    // Build variables map including project name
    let mut vars: HashMap<String, String> = manifest
        .variables
        .iter()
        .filter_map(|(k, _)| manifest.get_variable(k).map(|val| (k.clone(), val)))
        .collect();
    vars.insert("name".to_string(), manifest.project.name.clone());
    if let Some(version) = &manifest.project.version {
        vars.insert("version".to_string(), version.clone());
    }

    for (tool_name, tool_config) in &manifest.tool_configs {
        let result = generate_tool_config(tool_name, tool_config, &vars, provider, base_dir)?;
        results.push(result);
    }

    Ok(results)
}

/// Preview what configs would be generated (for diff mode).
pub fn preview_configs(
    manifest: &Manifest,
    provider: &dyn SchemaProvider,
    base_dir: &Path,
) -> Result<Vec<ConfigPreview>, GenerateError> {
    let mut previews = Vec::new();

    // Build variables map including project name
    let mut vars: HashMap<String, String> = manifest
        .variables
        .iter()
        .filter_map(|(k, _)| manifest.get_variable(k).map(|val| (k.clone(), val)))
        .collect();
    vars.insert("name".to_string(), manifest.project.name.clone());
    if let Some(version) = &manifest.project.version {
        vars.insert("version".to_string(), version.clone());
    }

    for (tool_name, tool_config) in &manifest.tool_configs {
        let preview = preview_tool_config(tool_name, tool_config, &vars, provider, base_dir)?;
        previews.push(preview);
    }

    Ok(previews)
}

/// Preview config for a single tool.
fn preview_tool_config(
    tool_name: &str,
    config: &toml::Value,
    vars: &HashMap<String, String>,
    provider: &dyn SchemaProvider,
    base_dir: &Path,
) -> Result<ConfigPreview, GenerateError> {
    // Fetch schema
    let schema = provider
        .fetch(tool_name)
        .map_err(|e| GenerateError::SchemaFetch(tool_name.to_string(), e))?;

    // Convert config to JSON for validation and variable expansion
    let config_json = toml_to_json(config);
    let expanded = expand_variables(&config_json, vars);

    // Validate against schema
    validate_config(tool_name, &expanded, &schema)?;

    // Serialize without writing
    let config_path = base_dir.join(&schema.config_path);
    let content = serialize_config(tool_name, &expanded, schema.format)?;

    // Read existing content if present
    let existing = fs::read_to_string(&config_path).ok();

    Ok(ConfigPreview {
        tool: tool_name.to_string(),
        path: config_path,
        content,
        existing,
    })
}

/// Generate config for a single tool.
fn generate_tool_config(
    tool_name: &str,
    config: &toml::Value,
    vars: &HashMap<String, String>,
    provider: &dyn SchemaProvider,
    base_dir: &Path,
) -> Result<GeneratedConfig, GenerateError> {
    // Fetch schema
    let schema = provider
        .fetch(tool_name)
        .map_err(|e| GenerateError::SchemaFetch(tool_name.to_string(), e))?;

    // Convert config to JSON for validation and variable expansion
    let config_json = toml_to_json(config);
    let expanded = expand_variables(&config_json, vars);

    // Validate against schema
    validate_config(tool_name, &expanded, &schema)?;

    // Write config file
    let config_path = base_dir.join(&schema.config_path);
    write_config(tool_name, &config_path, &expanded, schema.format)?;

    Ok(GeneratedConfig {
        tool: tool_name.to_string(),
        path: config_path,
        format: schema.format,
    })
}

/// Validate config against schema.
fn validate_config(
    tool_name: &str,
    config: &serde_json::Value,
    schema: &ToolSchema,
) -> Result<(), GenerateError> {
    let validator =
        jsonschema::validator_for(&schema.schema).map_err(|e| GenerateError::ValidationFailed {
            tool: tool_name.to_string(),
            errors: format!("invalid schema: {e}"),
        })?;

    let errors: Vec<_> = validator.iter_errors(config).collect();
    if !errors.is_empty() {
        let error_messages: Vec<_> = errors
            .iter()
            .map(|e| format!("  - {}: {}", e.instance_path, e))
            .collect();
        return Err(GenerateError::ValidationFailed {
            tool: tool_name.to_string(),
            errors: error_messages.join("\n"),
        });
    }

    Ok(())
}

/// Serialize config to string in the specified format.
fn serialize_config(
    tool_name: &str,
    config: &serde_json::Value,
    format: ConfigFormat,
) -> Result<String, GenerateError> {
    match format {
        ConfigFormat::Toml => {
            let toml_value = json_to_toml(config);
            toml::to_string_pretty(&toml_value)
                .map_err(|e| GenerateError::Serialize(tool_name.to_string(), e.to_string()))
        }
        ConfigFormat::Json => serde_json::to_string_pretty(config)
            .map_err(|e| GenerateError::Serialize(tool_name.to_string(), e.to_string())),
        ConfigFormat::Yaml => serde_yaml::to_string(config)
            .map_err(|e| GenerateError::Serialize(tool_name.to_string(), e.to_string())),
    }
}

/// Write config to file in the specified format.
fn write_config(
    tool_name: &str,
    path: &Path,
    config: &serde_json::Value,
    format: ConfigFormat,
) -> Result<(), GenerateError> {
    // Create parent directories
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| GenerateError::CreateDir(tool_name.to_string(), e))?;
    }

    let contents = serialize_config(tool_name, config, format)?;

    fs::write(path, contents).map_err(|e| GenerateError::WriteConfig(tool_name.to_string(), e))
}

/// Convert TOML value to JSON value.
fn toml_to_json(value: &toml::Value) -> serde_json::Value {
    match value {
        toml::Value::String(s) => serde_json::Value::String(s.clone()),
        toml::Value::Integer(i) => serde_json::Value::Number((*i).into()),
        toml::Value::Float(f) => serde_json::Number::from_f64(*f)
            .map(serde_json::Value::Number)
            .unwrap_or(serde_json::Value::Null),
        toml::Value::Boolean(b) => serde_json::Value::Bool(*b),
        toml::Value::Datetime(dt) => serde_json::Value::String(dt.to_string()),
        toml::Value::Array(arr) => serde_json::Value::Array(arr.iter().map(toml_to_json).collect()),
        toml::Value::Table(table) => {
            let map = table
                .iter()
                .map(|(k, v)| (k.clone(), toml_to_json(v)))
                .collect();
            serde_json::Value::Object(map)
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
        serde_json::Value::Array(arr) => toml::Value::Array(arr.iter().map(json_to_toml).collect()),
        serde_json::Value::Object(obj) => {
            let table = obj
                .iter()
                .map(|(k, v)| (k.clone(), json_to_toml(v)))
                .collect();
            toml::Value::Table(table)
        }
    }
}

/// Expand {{variable}} placeholders in all string values.
fn expand_variables(
    value: &serde_json::Value,
    vars: &HashMap<String, String>,
) -> serde_json::Value {
    match value {
        serde_json::Value::String(s) => {
            let mut result = s.clone();
            for (key, val) in vars {
                result = result.replace(&format!("{{{{{key}}}}}"), val);
            }
            serde_json::Value::String(result)
        }
        serde_json::Value::Array(arr) => {
            serde_json::Value::Array(arr.iter().map(|v| expand_variables(v, vars)).collect())
        }
        serde_json::Value::Object(obj) => {
            let map = obj
                .iter()
                .map(|(k, v)| (k.clone(), expand_variables(v, vars)))
                .collect();
            serde_json::Value::Object(map)
        }
        other => other.clone(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expand_simple_variable() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "my-project".to_string());

        let input = serde_json::json!({"title": "{{name}}"});
        let output = expand_variables(&input, &vars);

        assert_eq!(output["title"], "my-project");
    }

    #[test]
    fn expand_nested_variables() {
        let mut vars = HashMap::new();
        vars.insert("dir".to_string(), "./assets".to_string());

        let input = serde_json::json!({
            "paths": ["{{dir}}/a", "{{dir}}/b"]
        });
        let output = expand_variables(&input, &vars);

        assert_eq!(output["paths"][0], "./assets/a");
        assert_eq!(output["paths"][1], "./assets/b");
    }

    #[test]
    fn toml_json_roundtrip() {
        let toml_str = r#"
            name = "test"
            count = 42
            enabled = true
            tags = ["a", "b"]
        "#;
        let toml_value: toml::Value = toml::from_str(toml_str).unwrap();
        let json = toml_to_json(&toml_value);
        let back = json_to_toml(&json);

        assert_eq!(toml_value, back);
    }
}
