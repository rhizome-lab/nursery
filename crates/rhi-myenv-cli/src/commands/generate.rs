//! Generate command implementation.

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use rhi_myenv_core::{
    CliSchemaProvider, GenerateResult, Manifest, SchemaProvider, generate_configs, preview_configs,
};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc;
use std::time::Duration;

pub fn run(path: &PathBuf, check_only: bool, diff_mode: bool) -> ExitCode {
    let manifest = match Manifest::from_path(path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    if manifest.tool_configs.is_empty() {
        println!("no tools configured");
        return ExitCode::SUCCESS;
    }

    let provider = CliSchemaProvider;
    let base_dir = path.parent().unwrap_or(std::path::Path::new("."));

    if check_only {
        // Just validate, don't write
        for tool_name in manifest.tool_configs.keys() {
            match provider.fetch(tool_name) {
                Ok(schema) => {
                    println!("validated: {tool_name} -> {}", schema.config_path.display());
                }
                Err(e) => {
                    eprintln!("error: {tool_name}: {e}");
                    return ExitCode::FAILURE;
                }
            }
        }
        println!("all tools validated");
        return ExitCode::SUCCESS;
    }

    if diff_mode {
        // Show what would change
        match preview_configs(&manifest, &provider, base_dir) {
            Ok(previews) => {
                let mut has_changes = false;
                for preview in &previews {
                    let changed = match &preview.existing {
                        Some(existing) => existing != &preview.content,
                        None => true,
                    };

                    if changed {
                        has_changes = true;
                        println!("--- {}", preview.path.display());
                        print_diff(&preview.existing, &preview.content);
                        println!();
                    } else {
                        println!("unchanged: {} -> {}", preview.tool, preview.path.display());
                    }
                }
                if !has_changes {
                    println!("no changes");
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        }
    } else {
        match generate_configs(&manifest, &provider, base_dir) {
            Ok(results) => {
                let mut generated = 0;
                let mut skipped = 0;
                for result in &results {
                    match result {
                        GenerateResult::Generated(config) => {
                            println!("generated: {} -> {}", config.tool, config.path.display());
                            generated += 1;
                        }
                        GenerateResult::Skipped { tool, reason } => {
                            eprintln!("warning: skipped '{tool}': {reason}");
                            skipped += 1;
                        }
                    }
                }
                if generated > 0 {
                    println!("generated {} config(s)", generated);
                }
                if skipped > 0 {
                    println!("skipped {} tool(s)", skipped);
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("error: {e}");
                ExitCode::FAILURE
            }
        }
    }
}

/// Print a simple line-based diff.
fn print_diff(old: &Option<String>, new: &str) {
    let old_lines: Vec<&str> = old.as_deref().unwrap_or("").lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    if old.is_none() {
        println!("+++ (new file)");
        for line in &new_lines {
            println!("+{line}");
        }
        return;
    }

    // Simple diff: show removed lines, then added lines
    // For a more sophisticated diff, we'd use a diff library
    for line in &old_lines {
        if !new_lines.contains(line) {
            println!("-{line}");
        }
    }
    for line in &new_lines {
        if !old_lines.contains(line) {
            println!("+{line}");
        }
    }
}

pub fn watch(path: &PathBuf) -> ExitCode {
    // Run initial generation
    println!("watching: {}", path.display());
    if run(path, false, false) == ExitCode::FAILURE {
        eprintln!("initial generation failed, continuing to watch...");
    }

    let (tx, rx) = mpsc::channel();

    let mut watcher = match RecommendedWatcher::new(
        move |res| {
            if let Ok(event) = res {
                let _ = tx.send(event);
            }
        },
        Config::default().with_poll_interval(Duration::from_secs(1)),
    ) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("error: failed to create watcher: {e}");
            return ExitCode::FAILURE;
        }
    };

    // Watch the manifest file
    if let Err(e) = watcher.watch(path, RecursiveMode::NonRecursive) {
        eprintln!("error: failed to watch {}: {e}", path.display());
        return ExitCode::FAILURE;
    }

    println!("press Ctrl+C to stop");

    // Debounce: wait a short time after events to batch rapid changes
    let debounce = Duration::from_millis(100);
    let mut last_event = std::time::Instant::now() - debounce;

    loop {
        match rx.recv() {
            Ok(_event) => {
                let now = std::time::Instant::now();
                if now.duration_since(last_event) < debounce {
                    continue;
                }
                last_event = now;

                println!("\ndetected change, regenerating...");
                if run(path, false, false) == ExitCode::FAILURE {
                    eprintln!("generation failed");
                }
            }
            Err(e) => {
                eprintln!("error: watcher error: {e}");
                return ExitCode::FAILURE;
            }
        }
    }
}
