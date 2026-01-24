//! Config sync command implementations.

use myenv_core::{CliSchemaProvider, Manifest, merge_to_manifest, pull_configs};
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

pub fn pull(path: &PathBuf, tools: Vec<String>, dry_run: bool) -> ExitCode {
    let provider = CliSchemaProvider;
    let base_dir = path.parent().unwrap_or(std::path::Path::new("."));

    // Determine which tools to pull
    let tool_names: Vec<String> = if tools.is_empty() {
        // Try to read existing manifest to get tool list
        match Manifest::from_path(path) {
            Ok(m) => m.tool_configs.keys().cloned().collect(),
            Err(_) => {
                eprintln!("error: no tools specified and no existing manifest");
                eprintln!("hint: specify tools to pull, e.g., 'myenv config pull siphon dew'");
                return ExitCode::FAILURE;
            }
        }
    } else {
        tools
    };

    if tool_names.is_empty() {
        println!("no tools to pull");
        return ExitCode::SUCCESS;
    }

    // Pull configs
    let pulled = match pull_configs(&tool_names, &provider, base_dir) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    for config in &pulled {
        println!("pulled: {} <- {}", config.tool, config.path.display());
    }

    // Merge into manifest
    let existing = fs::read_to_string(path).ok();
    let merged = match merge_to_manifest(&pulled, existing.as_deref()) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    if dry_run {
        println!("\n--- nursery.toml (dry run) ---");
        println!("{merged}");
        return ExitCode::SUCCESS;
    }

    // Write manifest
    if let Err(e) = fs::write(path, &merged) {
        eprintln!("error: failed to write manifest: {e}");
        return ExitCode::FAILURE;
    }

    println!("updated: {}", path.display());
    ExitCode::SUCCESS
}
