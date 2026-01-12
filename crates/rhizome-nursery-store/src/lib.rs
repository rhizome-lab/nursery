//! Content-addressed package store.
//!
//! Stores packages at `~/.nursery/store/<hash>/` and activates binaries
//! via symlinks to `~/.nursery/bin/`.

use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::fs::{self, File};
use std::io::{self, BufReader, Read};
use std::path::{Path, PathBuf};

/// The nursery store for content-addressed packages.
#[derive(Debug)]
pub struct Store {
    /// Store directory (~/.local/share/nursery/store)
    store_dir: PathBuf,
    /// Bin directory (~/.local/share/nursery/bin)
    bin_dir: PathBuf,
}

/// A package in the store.
#[derive(Debug, Clone)]
pub struct StoredPackage {
    /// Content hash (sha256)
    pub hash: String,
    /// Path in store
    pub path: PathBuf,
    /// Binary names provided
    pub binaries: Vec<String>,
}

/// Errors from store operations.
#[derive(Debug, thiserror::Error)]
pub enum StoreError {
    #[error("failed to create directory: {0}")]
    CreateDir(#[source] io::Error),
    #[error("failed to read file: {0}")]
    ReadFile(#[source] io::Error),
    #[error("failed to write file: {0}")]
    WriteFile(#[source] io::Error),
    #[error("failed to download: {0}")]
    Download(String),
    #[error("failed to unpack archive: {0}")]
    Unpack(String),
    #[error("hash mismatch: expected {expected}, got {actual}")]
    HashMismatch { expected: String, actual: String },
    #[error("package not found: {0}")]
    NotFound(String),
}

impl Store {
    /// Create a store with default XDG paths.
    /// - Store: ~/.local/share/nursery/store (XDG_DATA_HOME)
    /// - Bin: ~/.local/share/nursery/bin
    pub fn new() -> Result<Self, StoreError> {
        let data_dir = dirs::data_dir()
            .map(|d| d.join("nursery"))
            .unwrap_or_else(|| {
                dirs::home_dir()
                    .map(|h| h.join(".local/share/nursery"))
                    .unwrap_or_else(|| PathBuf::from(".nursery"))
            });

        let store_dir = data_dir.join("store");
        let bin_dir = data_dir.join("bin");

        fs::create_dir_all(&store_dir).map_err(StoreError::CreateDir)?;
        fs::create_dir_all(&bin_dir).map_err(StoreError::CreateDir)?;

        Ok(Self { store_dir, bin_dir })
    }

    /// Create a store with a custom root directory.
    pub fn with_root(root: PathBuf) -> Result<Self, StoreError> {
        let store_dir = root.join("store");
        let bin_dir = root.join("bin");

        fs::create_dir_all(&store_dir).map_err(StoreError::CreateDir)?;
        fs::create_dir_all(&bin_dir).map_err(StoreError::CreateDir)?;

        Ok(Self { store_dir, bin_dir })
    }

    /// Get the bin directory path.
    pub fn bin_dir(&self) -> &Path {
        &self.bin_dir
    }

    /// Check if a package with given hash exists.
    pub fn has(&self, hash: &str) -> bool {
        self.store_dir.join(hash).exists()
    }

    /// Get a stored package by hash.
    pub fn get(&self, hash: &str) -> Option<StoredPackage> {
        let path = self.store_dir.join(hash);
        if !path.exists() {
            return None;
        }

        let binaries = self.find_binaries(&path);
        Some(StoredPackage {
            hash: hash.to_string(),
            path,
            binaries,
        })
    }

    /// Add a local file/directory to the store.
    pub fn add_path(&self, source: &Path) -> Result<StoredPackage, StoreError> {
        let hash = hash_path(source)?;
        let dest = self.store_dir.join(&hash);

        if !dest.exists() {
            if source.is_dir() {
                copy_dir_all(source, &dest)?;
            } else {
                fs::create_dir_all(&dest).map_err(StoreError::CreateDir)?;
                let file_name = source.file_name().unwrap_or_default();
                fs::copy(source, dest.join(file_name)).map_err(StoreError::WriteFile)?;
            }
        }

        let binaries = self.find_binaries(&dest);
        Ok(StoredPackage {
            hash,
            path: dest,
            binaries,
        })
    }

    /// Add from bytes (e.g., downloaded content).
    pub fn add_bytes(
        &self,
        bytes: &[u8],
        expected_hash: Option<&str>,
    ) -> Result<StoredPackage, StoreError> {
        let hash = hash_bytes(bytes);

        if let Some(expected) = expected_hash
            && hash != expected
        {
            return Err(StoreError::HashMismatch {
                expected: expected.to_string(),
                actual: hash,
            });
        }

        let dest = self.store_dir.join(&hash);

        if !dest.exists() {
            fs::create_dir_all(&dest).map_err(StoreError::CreateDir)?;

            // Try to unpack as archive, otherwise store as raw binary
            if !try_unpack_archive(bytes, &dest)? {
                // Single binary - write with executable permissions
                let bin_path = dest.join("bin");
                fs::write(&bin_path, bytes).map_err(StoreError::WriteFile)?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    fs::set_permissions(&bin_path, fs::Permissions::from_mode(0o755))
                        .map_err(StoreError::WriteFile)?;
                }
            }
        }

        let binaries = self.find_binaries(&dest);
        Ok(StoredPackage {
            hash,
            path: dest,
            binaries,
        })
    }

    /// Activate a package's binaries by symlinking to bin_dir.
    pub fn activate(&self, pkg: &StoredPackage) -> Result<Vec<PathBuf>, StoreError> {
        let mut activated = Vec::new();

        for bin_name in &pkg.binaries {
            let bin_path = self.find_binary_path(&pkg.path, bin_name);
            if let Some(source) = bin_path {
                let link = self.bin_dir.join(bin_name);

                // Remove existing symlink if present
                if link.exists() || link.symlink_metadata().is_ok() {
                    fs::remove_file(&link).map_err(StoreError::WriteFile)?;
                }

                #[cfg(unix)]
                std::os::unix::fs::symlink(&source, &link).map_err(StoreError::WriteFile)?;
                #[cfg(windows)]
                std::os::windows::fs::symlink_file(&source, &link)
                    .map_err(StoreError::WriteFile)?;

                activated.push(link);
            }
        }

        Ok(activated)
    }

    /// Deactivate a package's binaries.
    pub fn deactivate(&self, pkg: &StoredPackage) -> Result<(), StoreError> {
        for bin_name in &pkg.binaries {
            let link = self.bin_dir.join(bin_name);
            if link.symlink_metadata().is_ok() {
                fs::remove_file(&link).map_err(StoreError::WriteFile)?;
            }
        }
        Ok(())
    }

    /// List all packages in the store.
    pub fn list(&self) -> Result<Vec<StoredPackage>, StoreError> {
        let mut packages = Vec::new();

        if !self.store_dir.exists() {
            return Ok(packages);
        }

        for entry in fs::read_dir(&self.store_dir).map_err(StoreError::ReadFile)? {
            let entry = entry.map_err(StoreError::ReadFile)?;
            let path = entry.path();

            if path.is_dir()
                && let Some(hash) = path.file_name().and_then(|n| n.to_str())
            {
                let binaries = self.find_binaries(&path);
                packages.push(StoredPackage {
                    hash: hash.to_string(),
                    path,
                    binaries,
                });
            }
        }

        Ok(packages)
    }

    /// Garbage collect: remove packages not in the keep set.
    pub fn gc(&self, keep_hashes: &HashSet<String>) -> Result<Vec<String>, StoreError> {
        let mut removed = Vec::new();

        for pkg in self.list()? {
            if !keep_hashes.contains(&pkg.hash) {
                self.deactivate(&pkg)?;
                fs::remove_dir_all(&pkg.path).map_err(StoreError::WriteFile)?;
                removed.push(pkg.hash);
            }
        }

        Ok(removed)
    }

    /// Find executable binaries in a package directory.
    fn find_binaries(&self, pkg_path: &Path) -> Vec<String> {
        let mut binaries = Vec::new();

        // Check common binary locations
        let search_paths = [
            pkg_path.to_path_buf(),
            pkg_path.join("bin"),
            pkg_path.join("usr/bin"),
            pkg_path.join("usr/local/bin"),
        ];

        for search_path in search_paths {
            if let Ok(entries) = fs::read_dir(&search_path) {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if is_executable(&path)
                        && let Some(name) = path.file_name().and_then(|n| n.to_str())
                        && !binaries.contains(&name.to_string())
                    {
                        binaries.push(name.to_string());
                    }
                }
            }
        }

        binaries
    }

    /// Find the actual path to a binary within a package.
    fn find_binary_path(&self, pkg_path: &Path, bin_name: &str) -> Option<PathBuf> {
        let search_paths = [
            pkg_path.join(bin_name),
            pkg_path.join("bin").join(bin_name),
            pkg_path.join("usr/bin").join(bin_name),
            pkg_path.join("usr/local/bin").join(bin_name),
        ];

        search_paths.into_iter().find(|p| p.exists())
    }
}

impl Default for Store {
    fn default() -> Self {
        Self::new().expect("failed to create default store")
    }
}

/// Hash a file or directory.
fn hash_path(path: &Path) -> Result<String, StoreError> {
    let mut hasher = Sha256::new();

    if path.is_file() {
        let file = File::open(path).map_err(StoreError::ReadFile)?;
        let mut reader = BufReader::new(file);
        let mut buffer = [0u8; 8192];

        loop {
            let n = reader.read(&mut buffer).map_err(StoreError::ReadFile)?;
            if n == 0 {
                break;
            }
            hasher.update(&buffer[..n]);
        }
    } else if path.is_dir() {
        hash_dir(&mut hasher, path)?;
    }

    Ok(format!("{:x}", hasher.finalize()))
}

/// Hash directory contents recursively.
fn hash_dir(hasher: &mut Sha256, dir: &Path) -> Result<(), StoreError> {
    let mut entries: Vec<_> = fs::read_dir(dir)
        .map_err(StoreError::ReadFile)?
        .filter_map(|e| e.ok())
        .collect();

    // Sort for deterministic hashing
    entries.sort_by_key(|e| e.path());

    for entry in entries {
        let path = entry.path();
        let name = path.file_name().unwrap_or_default().to_string_lossy();

        hasher.update(name.as_bytes());

        if path.is_file() {
            let contents = fs::read(&path).map_err(StoreError::ReadFile)?;
            hasher.update(&contents);
        } else if path.is_dir() {
            hash_dir(hasher, &path)?;
        }
    }

    Ok(())
}

/// Hash bytes.
fn hash_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

/// Copy a directory recursively.
fn copy_dir_all(src: &Path, dst: &Path) -> Result<(), StoreError> {
    fs::create_dir_all(dst).map_err(StoreError::CreateDir)?;

    for entry in fs::read_dir(src).map_err(StoreError::ReadFile)? {
        let entry = entry.map_err(StoreError::ReadFile)?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_all(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path).map_err(StoreError::WriteFile)?;
        }
    }

    Ok(())
}

/// Try to unpack an archive. Returns true if it was an archive.
fn try_unpack_archive(bytes: &[u8], dest: &Path) -> Result<bool, StoreError> {
    // Try tar.gz
    if bytes.len() > 2 && bytes[0] == 0x1f && bytes[1] == 0x8b {
        let decoder = flate2::read::GzDecoder::new(bytes);
        let mut archive = tar::Archive::new(decoder);
        archive
            .unpack(dest)
            .map_err(|e| StoreError::Unpack(e.to_string()))?;
        return Ok(true);
    }

    // Try tar.xz
    if bytes.len() > 6 && &bytes[0..6] == b"\xfd7zXZ\x00" {
        let decoder = xz2::read::XzDecoder::new(bytes);
        let mut archive = tar::Archive::new(decoder);
        archive
            .unpack(dest)
            .map_err(|e| StoreError::Unpack(e.to_string()))?;
        return Ok(true);
    }

    // Try zip
    if bytes.len() > 4 && &bytes[0..4] == b"PK\x03\x04" {
        let cursor = std::io::Cursor::new(bytes);
        let mut archive =
            zip::ZipArchive::new(cursor).map_err(|e| StoreError::Unpack(e.to_string()))?;
        archive
            .extract(dest)
            .map_err(|e| StoreError::Unpack(e.to_string()))?;
        return Ok(true);
    }

    Ok(false)
}

/// Check if a file is executable.
#[cfg(unix)]
fn is_executable(path: &Path) -> bool {
    use std::os::unix::fs::PermissionsExt;
    path.is_file()
        && fs::metadata(path)
            .map(|m| m.permissions().mode() & 0o111 != 0)
            .unwrap_or(false)
}

#[cfg(not(unix))]
fn is_executable(path: &Path) -> bool {
    path.is_file()
        && path
            .extension()
            .map(|e| e == "exe" || e == "cmd" || e == "bat")
            .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn store_and_retrieve() {
        let temp = TempDir::new().unwrap();
        let store = Store::with_root(temp.path().to_path_buf()).unwrap();

        // Create a test binary
        let bin_content = b"#!/bin/sh\necho hello";
        let pkg = store.add_bytes(bin_content, None).unwrap();

        assert!(store.has(&pkg.hash));
        assert!(store.get(&pkg.hash).is_some());
    }

    #[test]
    fn hash_verification() {
        let temp = TempDir::new().unwrap();
        let store = Store::with_root(temp.path().to_path_buf()).unwrap();

        let content = b"test content";
        let hash = hash_bytes(content);

        // Correct hash
        let pkg = store.add_bytes(content, Some(&hash)).unwrap();
        assert_eq!(pkg.hash, hash);

        // Wrong hash
        let result = store.add_bytes(content, Some("wrong"));
        assert!(matches!(result, Err(StoreError::HashMismatch { .. })));
    }

    #[test]
    fn activation() {
        let temp = TempDir::new().unwrap();
        let store = Store::with_root(temp.path().to_path_buf()).unwrap();

        // Create a mock binary
        let pkg_dir = temp.path().join("pkg");
        fs::create_dir_all(&pkg_dir).unwrap();
        let bin_path = pkg_dir.join("mytool");
        let mut file = File::create(&bin_path).unwrap();
        file.write_all(b"#!/bin/sh\necho hello").unwrap();
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&bin_path, fs::Permissions::from_mode(0o755)).unwrap();
        }

        let pkg = store.add_path(&pkg_dir).unwrap();
        let activated = store.activate(&pkg).unwrap();

        assert!(!activated.is_empty());
        assert!(store.bin_dir().join("mytool").exists());
    }

    #[test]
    fn garbage_collection() {
        let temp = TempDir::new().unwrap();
        let store = Store::with_root(temp.path().to_path_buf()).unwrap();

        let pkg1 = store.add_bytes(b"package1", None).unwrap();
        let pkg2 = store.add_bytes(b"package2", None).unwrap();

        let mut keep = HashSet::new();
        keep.insert(pkg1.hash.clone());

        let removed = store.gc(&keep).unwrap();

        assert!(store.has(&pkg1.hash));
        assert!(!store.has(&pkg2.hash));
        assert_eq!(removed.len(), 1);
        assert_eq!(removed[0], pkg2.hash);
    }
}
