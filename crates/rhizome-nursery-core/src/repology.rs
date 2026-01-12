//! Repology API client for cross-platform package name resolution.
//!
//! Uses the [Repology API](https://repology.org/api) to look up package names
//! across different package manager ecosystems.

use crate::Ecosystem;
use serde::Deserialize;
use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

/// A Repology API client with local caching.
pub struct RepologyClient {
    cache_dir: PathBuf,
    cache_ttl: Duration,
}

/// A package entry from Repology.
#[derive(Debug, Clone, Deserialize)]
pub struct RepologyPackage {
    /// Repository name (e.g., "arch", "debian_12").
    pub repo: String,
    /// Visible package name (what to install).
    #[serde(default)]
    pub visiblename: Option<String>,
    /// Binary name (what executable to look for).
    #[serde(default)]
    pub binname: Option<String>,
    /// Source package name (for reference).
    #[serde(default)]
    #[allow(dead_code)]
    pub srcname: Option<String>,
    /// Version string.
    #[serde(default)]
    pub version: Option<String>,
    /// Version status (newest, outdated, etc.).
    #[serde(default)]
    pub status: Option<String>,
    /// Summary/description.
    #[serde(default)]
    #[allow(dead_code)]
    pub summary: Option<String>,
}

/// Resolved tool information from Repology.
#[derive(Debug, Clone, Default)]
pub struct ToolInfo {
    /// Package name per ecosystem.
    pub packages: BTreeMap<Ecosystem, PackageInfo>,
    /// Binary name to look for (if different from tool name).
    pub binname: Option<String>,
}

/// Package info for a specific ecosystem.
#[derive(Debug, Clone)]
pub struct PackageInfo {
    /// Package name in this ecosystem.
    pub name: String,
    /// Latest version available.
    pub version: String,
}

/// Errors from the Repology client.
#[derive(Debug, thiserror::Error)]
pub enum RepologyError {
    #[error("HTTP request failed: {0}")]
    Http(String),
    #[error("failed to parse response: {0}")]
    Parse(#[from] serde_json::Error),
    #[error("cache error: {0}")]
    Cache(#[from] std::io::Error),
    #[error("project not found: {0}")]
    NotFound(String),
}

impl RepologyClient {
    /// Create a new client with default cache settings.
    pub fn new() -> Self {
        let cache_dir = dirs::cache_dir()
            .unwrap_or_else(|| PathBuf::from(".cache"))
            .join("nursery")
            .join("repology");

        Self {
            cache_dir,
            cache_ttl: Duration::from_secs(24 * 60 * 60), // 24 hours
        }
    }

    /// Create a client with a custom cache directory.
    pub fn with_cache_dir(cache_dir: PathBuf) -> Self {
        Self {
            cache_dir,
            cache_ttl: Duration::from_secs(24 * 60 * 60),
        }
    }

    /// Look up a project by name.
    pub fn lookup(&self, project: &str) -> Result<ToolInfo, RepologyError> {
        // Check cache first
        if let Some(cached) = self.read_cache(project)? {
            return Ok(cached);
        }

        // Fetch from API
        let packages = self.fetch_project(project)?;

        // Convert to ToolInfo
        let info = self.process_packages(packages);

        // Cache the result
        self.write_cache(project, &info)?;

        Ok(info)
    }

    /// Fetch project data from the Repology API.
    fn fetch_project(&self, project: &str) -> Result<Vec<RepologyPackage>, RepologyError> {
        let url = format!("https://repology.org/api/v1/project/{}", project);

        let response = ureq::get(&url)
            .set(
                "User-Agent",
                "nursery/0.1 (https://github.com/rhizome-lab/nursery)",
            )
            .call()
            .map_err(|e| RepologyError::Http(e.to_string()))?;

        if response.status() == 404 {
            return Err(RepologyError::NotFound(project.to_string()));
        }

        let body = response
            .into_string()
            .map_err(|e| RepologyError::Http(e.to_string()))?;

        let packages: Vec<RepologyPackage> = serde_json::from_str(&body)?;

        Ok(packages)
    }

    /// Process Repology packages into a ToolInfo.
    fn process_packages(&self, packages: Vec<RepologyPackage>) -> ToolInfo {
        let mut info = ToolInfo::default();

        // Build a map of repo -> best package
        let mut repo_packages: HashMap<&str, &RepologyPackage> = HashMap::new();

        for pkg in &packages {
            let pkg_name = pkg.visiblename.as_deref().unwrap_or("");

            // Skip documentation, completion, development, and other auxiliary packages
            if pkg_name.ends_with("-doc")
                || pkg_name.ends_with("-docs")
                || pkg_name.ends_with("-git")
                || pkg_name.ends_with("-bin")
                || pkg_name.ends_with("-dev")
                || pkg_name.ends_with("-devel")
                || pkg_name.ends_with("-dbg")
                || pkg_name.contains("-completion")
                || pkg_name.contains("-debug")
            {
                continue;
            }

            if let Some(existing) = repo_packages.get(pkg.repo.as_str()) {
                // Prefer packages with status "newest"
                let existing_newest = existing.status.as_deref() == Some("newest");
                let pkg_newest = pkg.status.as_deref() == Some("newest");

                if existing_newest && !pkg_newest {
                    continue; // Keep existing
                }

                // If same status, prefer shorter name (usually the main package)
                let existing_name_len = existing
                    .visiblename
                    .as_ref()
                    .map(|n| n.len())
                    .unwrap_or(usize::MAX);
                let pkg_name_len = pkg_name.len();

                if !pkg_newest && pkg_name_len >= existing_name_len {
                    continue; // Keep existing
                }
            }

            repo_packages.insert(&pkg.repo, pkg);
        }

        // Map repos to our ecosystems
        for (repo, pkg) in repo_packages {
            if let Some(ecosystem) = repo_to_ecosystem(repo) {
                let name = pkg
                    .visiblename
                    .clone()
                    .unwrap_or_else(|| pkg.binname.clone().unwrap_or_default());

                if !name.is_empty() {
                    info.packages.insert(
                        ecosystem,
                        PackageInfo {
                            name,
                            version: pkg.version.clone().unwrap_or_default(),
                        },
                    );
                }

                // Capture binname from a good package (not completion/doc)
                if info.binname.is_none()
                    && let Some(binname) = &pkg.binname
                    && !binname.ends_with("-completion")
                    && !binname.ends_with("-doc")
                    && !binname.contains("completion")
                {
                    info.binname = Some(binname.clone());
                }
            }
        }

        info
    }

    /// Read from cache if valid.
    fn read_cache(&self, project: &str) -> Result<Option<ToolInfo>, RepologyError> {
        let cache_path = self.cache_path(project);

        if !cache_path.exists() {
            return Ok(None);
        }

        // Check TTL
        let metadata = std::fs::metadata(&cache_path)?;
        let modified = metadata.modified()?;
        let age = SystemTime::now()
            .duration_since(modified)
            .unwrap_or(Duration::MAX);

        if age > self.cache_ttl {
            return Ok(None);
        }

        // Read and parse
        let contents = std::fs::read_to_string(&cache_path)?;
        let cached: CachedToolInfo = serde_json::from_str(&contents)?;

        Ok(Some(cached.into()))
    }

    /// Write to cache.
    fn write_cache(&self, project: &str, info: &ToolInfo) -> Result<(), RepologyError> {
        std::fs::create_dir_all(&self.cache_dir)?;

        let cache_path = self.cache_path(project);
        let cached = CachedToolInfo::from(info.clone());
        let contents = serde_json::to_string_pretty(&cached)?;

        std::fs::write(cache_path, contents)?;
        Ok(())
    }

    /// Get the cache file path for a project.
    fn cache_path(&self, project: &str) -> PathBuf {
        self.cache_dir.join(format!("{}.json", project))
    }

    /// Clear all cached data.
    pub fn clear_cache(&self) -> Result<(), RepologyError> {
        if self.cache_dir.exists() {
            std::fs::remove_dir_all(&self.cache_dir)?;
        }
        Ok(())
    }
}

impl Default for RepologyClient {
    fn default() -> Self {
        Self::new()
    }
}

/// Cached representation of ToolInfo (serializable).
#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CachedToolInfo {
    packages: BTreeMap<String, CachedPackageInfo>,
    binname: Option<String>,
}

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct CachedPackageInfo {
    name: String,
    version: String,
}

impl From<ToolInfo> for CachedToolInfo {
    fn from(info: ToolInfo) -> Self {
        Self {
            packages: info
                .packages
                .into_iter()
                .map(|(eco, pkg)| {
                    (
                        eco.id().to_string(),
                        CachedPackageInfo {
                            name: pkg.name,
                            version: pkg.version,
                        },
                    )
                })
                .collect(),
            binname: info.binname,
        }
    }
}

impl From<CachedToolInfo> for ToolInfo {
    fn from(cached: CachedToolInfo) -> Self {
        Self {
            packages: cached
                .packages
                .into_iter()
                .filter_map(|(eco_str, pkg)| {
                    Ecosystem::from_id(&eco_str).map(|eco| {
                        (
                            eco,
                            PackageInfo {
                                name: pkg.name,
                                version: pkg.version,
                            },
                        )
                    })
                })
                .collect(),
            binname: cached.binname,
        }
    }
}

/// Map a Repology repo name to our Ecosystem enum.
fn repo_to_ecosystem(repo: &str) -> Option<Ecosystem> {
    // Repology uses various repo names - we map the primary ones
    match repo {
        // Arch Linux
        "arch" | "aur" => Some(Ecosystem::Pacman),

        // Debian/Ubuntu
        s if s.starts_with("debian") || s.starts_with("ubuntu") => Some(Ecosystem::Apt),

        // Fedora/RHEL
        s if s.starts_with("fedora") || s.starts_with("centos") || s.starts_with("epel") => {
            Some(Ecosystem::Dnf)
        }

        // Alpine
        s if s.starts_with("alpine") => Some(Ecosystem::Apk),

        // Homebrew
        "homebrew" | "homebrew_casks" => Some(Ecosystem::Brew),

        // Nix
        s if s.starts_with("nix") => Some(Ecosystem::Nix),

        // Scoop
        "scoop" => Some(Ecosystem::Scoop),

        // WinGet
        "winget" => Some(Ecosystem::Winget),

        // Cargo (crates.io)
        "crates_io" => Some(Ecosystem::Cargo),

        _ => None,
    }
}

/// Get the preferred Repology repo for an ecosystem.
#[allow(dead_code)]
pub fn ecosystem_to_repo(ecosystem: Ecosystem) -> &'static str {
    match ecosystem {
        Ecosystem::Pacman => "arch",
        Ecosystem::Apt => "debian_12",
        Ecosystem::Dnf => "fedora_40",
        Ecosystem::Apk => "alpine_edge",
        Ecosystem::Brew => "homebrew",
        Ecosystem::Nix => "nix_unstable",
        Ecosystem::Scoop => "scoop",
        Ecosystem::Winget => "winget",
        Ecosystem::Cargo => "crates_io",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repo_mapping() {
        assert_eq!(repo_to_ecosystem("arch"), Some(Ecosystem::Pacman));
        assert_eq!(repo_to_ecosystem("debian_12"), Some(Ecosystem::Apt));
        assert_eq!(repo_to_ecosystem("ubuntu_24_04"), Some(Ecosystem::Apt));
        assert_eq!(repo_to_ecosystem("fedora_40"), Some(Ecosystem::Dnf));
        assert_eq!(repo_to_ecosystem("alpine_edge"), Some(Ecosystem::Apk));
        assert_eq!(repo_to_ecosystem("homebrew"), Some(Ecosystem::Brew));
        assert_eq!(repo_to_ecosystem("nix_unstable"), Some(Ecosystem::Nix));
        assert_eq!(repo_to_ecosystem("scoop"), Some(Ecosystem::Scoop));
        assert_eq!(repo_to_ecosystem("winget"), Some(Ecosystem::Winget));
        assert_eq!(repo_to_ecosystem("crates_io"), Some(Ecosystem::Cargo));
        assert_eq!(repo_to_ecosystem("unknown_repo"), None);
    }

    #[test]
    fn ecosystem_to_repo_mapping() {
        assert_eq!(ecosystem_to_repo(Ecosystem::Pacman), "arch");
        assert_eq!(ecosystem_to_repo(Ecosystem::Apt), "debian_12");
    }

    #[test]
    fn cache_roundtrip() {
        let mut info = ToolInfo::default();
        info.packages.insert(
            Ecosystem::Pacman,
            PackageInfo {
                name: "ripgrep".to_string(),
                version: "14.1.0".to_string(),
            },
        );
        info.binname = Some("rg".to_string());

        let cached = CachedToolInfo::from(info.clone());
        let roundtrip: ToolInfo = cached.into();

        assert_eq!(roundtrip.binname, Some("rg".to_string()));
        assert!(roundtrip.packages.contains_key(&Ecosystem::Pacman));
    }
}
