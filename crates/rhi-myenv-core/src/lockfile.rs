//! Lockfile parsing and generation for `myenv.lock`.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::Path;

/// A parsed lockfile.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Lockfile {
    /// Locked tool entries.
    #[serde(flatten)]
    pub tools: BTreeMap<String, LockedTool>,
}

/// A locked tool with resolved packages per ecosystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedTool {
    /// Canonical source (e.g., "github:BurntSushi/ripgrep").
    pub source: String,
    /// Original version constraint from nursery.toml.
    pub constraint: String,
    /// Resolved packages per ecosystem.
    #[serde(flatten)]
    pub ecosystems: BTreeMap<String, LockedPackage>,
}

/// A locked package for a specific ecosystem.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LockedPackage {
    /// Package name in this ecosystem.
    pub package: String,
    /// Resolved version.
    pub version: String,
    /// Hash if available (nix, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hash: Option<String>,
    /// Archive URL for historical versions (ALA, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub archive: Option<String>,
    /// Nixpkgs revision for nix.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nixpkgs: Option<String>,
}

/// Errors that can occur with lockfiles.
#[derive(Debug, thiserror::Error)]
pub enum LockfileError {
    #[error("failed to read lockfile: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to parse lockfile: {0}")]
    Parse(#[from] toml::de::Error),
    #[error("failed to serialize lockfile: {0}")]
    Serialize(#[from] toml::ser::Error),
}

impl Lockfile {
    /// Load a lockfile from a path.
    pub fn from_path(path: impl AsRef<Path>) -> Result<Self, LockfileError> {
        let contents = std::fs::read_to_string(path)?;
        Self::parse(&contents)
    }

    /// Parse a lockfile from a TOML string.
    pub fn parse(s: &str) -> Result<Self, LockfileError> {
        Ok(toml::from_str(s)?)
    }

    /// Load from path, or return empty lockfile if not found.
    pub fn load_or_default(path: impl AsRef<Path>) -> Self {
        Self::from_path(path).unwrap_or_default()
    }

    /// Serialize to TOML string.
    pub fn to_string(&self) -> Result<String, LockfileError> {
        Ok(toml::to_string_pretty(self)?)
    }

    /// Write to a file.
    pub fn write(&self, path: impl AsRef<Path>) -> Result<(), LockfileError> {
        let contents = self.to_string()?;
        std::fs::write(path, contents)?;
        Ok(())
    }

    /// Get the locked package for a tool in a specific ecosystem.
    pub fn get(&self, tool: &str, ecosystem: &str) -> Option<&LockedPackage> {
        self.tools.get(tool)?.ecosystems.get(ecosystem)
    }

    /// Check if a tool is locked.
    pub fn has_tool(&self, tool: &str) -> bool {
        self.tools.contains_key(tool)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_lockfile() {
        let toml = r#"
            [ripgrep]
            source = "github:BurntSushi/ripgrep"
            constraint = ">=14"

            [ripgrep.pacman]
            package = "ripgrep"
            version = "14.1.0-1"
            archive = "https://archive.archlinux.org/packages/r/ripgrep/ripgrep-14.1.0-1-x86_64.pkg.tar.zst"

            [ripgrep.nix]
            package = "ripgrep"
            version = "14.1.0"
            hash = "sha256-abc123"
            nixpkgs = "github:NixOS/nixpkgs/abc123"
        "#;

        let lockfile = Lockfile::parse(toml).unwrap();
        assert!(lockfile.has_tool("ripgrep"));

        let pacman = lockfile.get("ripgrep", "pacman").unwrap();
        assert_eq!(pacman.package, "ripgrep");
        assert_eq!(pacman.version, "14.1.0-1");
        assert!(pacman.archive.is_some());

        let nix = lockfile.get("ripgrep", "nix").unwrap();
        assert_eq!(nix.version, "14.1.0");
        assert!(nix.hash.is_some());
        assert!(nix.nixpkgs.is_some());
    }

    #[test]
    fn roundtrip_lockfile() {
        let mut lockfile = Lockfile::default();

        let mut ecosystems = BTreeMap::new();
        ecosystems.insert(
            "apt".to_string(),
            LockedPackage {
                package: "ripgrep".to_string(),
                version: "14.0.0".to_string(),
                hash: None,
                archive: None,
                nixpkgs: None,
            },
        );

        lockfile.tools.insert(
            "ripgrep".to_string(),
            LockedTool {
                source: "github:BurntSushi/ripgrep".to_string(),
                constraint: ">=14".to_string(),
                ecosystems,
            },
        );

        let serialized = lockfile.to_string().unwrap();
        let parsed = Lockfile::parse(&serialized).unwrap();

        assert!(parsed.has_tool("ripgrep"));
        let apt = parsed.get("ripgrep", "apt").unwrap();
        assert_eq!(apt.version, "14.0.0");
    }
}
