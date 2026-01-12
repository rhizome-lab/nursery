//! Schema discovery from tools.
//!
//! Tools provide their configuration schema via the `--schema` flag convention.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::Command;

/// Tool schema response from `<tool> --schema`.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ToolSchema {
    /// Path where the tool expects its config file.
    pub config_path: PathBuf,
    /// Config file format.
    pub format: ConfigFormat,
    /// JSON Schema for validation.
    pub schema: serde_json::Value,
}

/// Supported config file formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ConfigFormat {
    Toml,
    Json,
    Yaml,
}

/// Errors that can occur when fetching a schema.
#[derive(Debug, thiserror::Error)]
pub enum SchemaError {
    #[error("tool '{0}' not found in PATH")]
    ToolNotFound(String),
    #[error("tool '{tool}' exited with {code}: {stderr}")]
    ToolFailed {
        tool: String,
        code: i32,
        stderr: String,
    },
    #[error("tool '{0}' did not output valid schema response: {1}")]
    InvalidResponse(String, serde_json::Error),
    #[error("failed to execute tool '{0}': {1}")]
    Exec(String, std::io::Error),
}

/// Provides schemas for tools.
pub trait SchemaProvider {
    /// Fetch the schema for a tool by name.
    fn fetch(&self, tool: &str) -> Result<ToolSchema, SchemaError>;
}

/// Fetches schemas by invoking `<tool> --schema`.
#[derive(Debug, Default, Clone)]
pub struct CliSchemaProvider;

impl SchemaProvider for CliSchemaProvider {
    fn fetch(&self, tool: &str) -> Result<ToolSchema, SchemaError> {
        let output = Command::new(tool).arg("--schema").output().map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                SchemaError::ToolNotFound(tool.to_string())
            } else {
                SchemaError::Exec(tool.to_string(), e)
            }
        })?;

        if !output.status.success() {
            return Err(SchemaError::ToolFailed {
                tool: tool.to_string(),
                code: output.status.code().unwrap_or(-1),
                stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
            });
        }

        serde_json::from_slice(&output.stdout)
            .map_err(|e| SchemaError::InvalidResponse(tool.to_string(), e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_schema_response() {
        let json = r#"{
            "config_path": ".mytool/config.toml",
            "format": "toml",
            "schema": {
                "type": "object",
                "properties": {
                    "source": { "type": "string" }
                }
            }
        }"#;

        let schema: ToolSchema = serde_json::from_str(json).unwrap();
        assert_eq!(schema.config_path, PathBuf::from(".mytool/config.toml"));
        assert_eq!(schema.format, ConfigFormat::Toml);
    }

    #[test]
    fn parse_json_format() {
        let json = r#"{
            "config_path": "config.json",
            "format": "json",
            "schema": {}
        }"#;

        let schema: ToolSchema = serde_json::from_str(json).unwrap();
        assert_eq!(schema.format, ConfigFormat::Json);
    }

    #[test]
    fn parse_yaml_format() {
        let json = r#"{
            "config_path": "config.yaml",
            "format": "yaml",
            "schema": {}
        }"#;

        let schema: ToolSchema = serde_json::from_str(json).unwrap();
        assert_eq!(schema.format, ConfigFormat::Yaml);
    }
}
