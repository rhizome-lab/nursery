//! User configuration from ~/.config/nursery/config.toml

use serde::Deserialize;
use std::path::PathBuf;

/// User configuration.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
pub struct UserConfig {
    /// Tool installation preferences.
    pub tools: ToolsConfig,
}

/// Tool installation preferences.
#[derive(Debug, Clone, Deserialize)]
#[serde(default)]
pub struct ToolsConfig {
    /// Default source for tool installation.
    pub source: ToolSource,
}

impl Default for ToolsConfig {
    fn default() -> Self {
        Self {
            source: ToolSource::PreferSystem,
        }
    }
}

/// Where to install tools from.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ToolSource {
    /// Always use system package manager.
    System,
    /// Always use local store.
    Store,
    /// Prefer system, fall back to store.
    #[default]
    PreferSystem,
    /// Prefer store, fall back to system.
    PreferStore,
}

impl UserConfig {
    /// Load user config from default path (~/.config/nursery/config.toml).
    pub fn load() -> Self {
        Self::from_path(Self::default_path()).unwrap_or_default()
    }

    /// Load user config from a specific path.
    pub fn from_path(path: Option<PathBuf>) -> Option<Self> {
        let path = path?;
        let contents = std::fs::read_to_string(&path).ok()?;
        toml::from_str(&contents).ok()
    }

    /// Get the default config path.
    pub fn default_path() -> Option<PathBuf> {
        dirs::config_dir().map(|d| d.join("nursery").join("config.toml"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_config() {
        let toml = r#"
            [tools]
            source = "prefer-store"
        "#;

        let config: UserConfig = toml::from_str(toml).unwrap();
        assert_eq!(config.tools.source, ToolSource::PreferStore);
    }

    #[test]
    fn default_config() {
        let config = UserConfig::default();
        assert_eq!(config.tools.source, ToolSource::PreferSystem);
    }

    #[test]
    fn parse_all_sources() {
        for (s, expected) in [
            ("system", ToolSource::System),
            ("store", ToolSource::Store),
            ("prefer-system", ToolSource::PreferSystem),
            ("prefer-store", ToolSource::PreferStore),
        ] {
            let toml = format!("[tools]\nsource = \"{s}\"");
            let config: UserConfig = toml::from_str(&toml).unwrap();
            assert_eq!(config.tools.source, expected);
        }
    }
}
