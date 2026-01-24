//! Project scaffolding from seed templates.

mod builtin;
mod resolve;
mod variables;

use std::collections::HashMap;
use std::fs;
use std::path::Path;

pub use resolve::{SeedResolver, SeedSource};
pub use variables::{VariableResolver, VariableSource};

/// A seed template for scaffolding new projects.
#[derive(Debug, Clone)]
pub struct Seed {
    /// Seed name.
    pub name: String,
    /// Short description.
    pub description: String,
    /// Variable definitions (name -> default value, None if required).
    pub variables: HashMap<String, Option<String>>,
    /// Where this seed came from.
    source: SeedSource,
}

#[derive(Debug, thiserror::Error)]
pub enum SeedError {
    #[error("unknown seed: {0}")]
    UnknownSeed(String),
    #[error("directory already exists: {0}")]
    AlreadyExists(String),
    #[error("failed to create directory: {0}")]
    CreateDir(#[source] std::io::Error),
    #[error("failed to write file: {0}")]
    WriteFile(#[source] std::io::Error),
    #[error("failed to read seed: {0}")]
    ReadSeed(#[source] std::io::Error),
    #[error("failed to parse seed.toml: {0}")]
    ParseSeed(#[source] toml::de::Error),
    #[error("missing required variable: {0}")]
    MissingVariable(String),
}

impl Seed {
    /// Scaffold a new project.
    ///
    /// If `raw` is true, no variable substitution is performed.
    pub fn scaffold(
        &self,
        dest: &Path,
        vars: &HashMap<String, String>,
        raw: bool,
    ) -> Result<(), SeedError> {
        if dest.exists() {
            return Err(SeedError::AlreadyExists(dest.display().to_string()));
        }

        // Check for missing required variables
        if !raw {
            for (name, default) in &self.variables {
                if default.is_none() && !vars.contains_key(name) {
                    return Err(SeedError::MissingVariable(name.clone()));
                }
            }
        }

        fs::create_dir_all(dest).map_err(SeedError::CreateDir)?;

        match &self.source {
            SeedSource::Builtin(files) => {
                for (path, contents) in *files {
                    let expanded = if raw {
                        contents.to_string()
                    } else {
                        substitute(contents, vars)
                    };
                    write_file(dest, path, &expanded)?;
                }
            }
            SeedSource::Directory(seed_dir) => {
                let template_dir = seed_dir.join("template");
                copy_dir(&template_dir, dest, vars, raw)?;
            }
        }

        Ok(())
    }
}

fn write_file(dest: &Path, relative: &str, contents: &str) -> Result<(), SeedError> {
    let file_path = dest.join(relative);
    if let Some(parent) = file_path.parent() {
        fs::create_dir_all(parent).map_err(SeedError::CreateDir)?;
    }
    fs::write(&file_path, contents).map_err(SeedError::WriteFile)
}

fn copy_dir(
    src: &Path,
    dest: &Path,
    vars: &HashMap<String, String>,
    raw: bool,
) -> Result<(), SeedError> {
    if !src.exists() {
        return Ok(());
    }

    for entry in fs::read_dir(src).map_err(SeedError::ReadSeed)? {
        let entry = entry.map_err(SeedError::ReadSeed)?;
        let src_path = entry.path();
        let file_name = entry.file_name();
        let dest_path = dest.join(&file_name);

        if src_path.is_dir() {
            fs::create_dir_all(&dest_path).map_err(SeedError::CreateDir)?;
            copy_dir(&src_path, &dest_path, vars, raw)?;
        } else {
            let contents = fs::read_to_string(&src_path).map_err(SeedError::ReadSeed)?;
            let expanded = if raw {
                contents
            } else {
                substitute(&contents, vars)
            };
            fs::write(&dest_path, expanded).map_err(SeedError::WriteFile)?;
        }
    }

    Ok(())
}

/// Simple variable substitution: replaces `{{key}}` with value.
pub fn substitute(template: &str, vars: &HashMap<String, String>) -> String {
    let mut result = template.to_string();
    for (key, value) in vars {
        result = result.replace(&format!("{{{{{key}}}}}"), value);
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn substitute_vars() {
        let mut vars = HashMap::new();
        vars.insert("name".to_string(), "my-project".to_string());
        vars.insert("version".to_string(), "1.0.0".to_string());

        let result = substitute("name = \"{{name}}\"\nversion = \"{{version}}\"", &vars);
        assert_eq!(result, "name = \"my-project\"\nversion = \"1.0.0\"");
    }
}
