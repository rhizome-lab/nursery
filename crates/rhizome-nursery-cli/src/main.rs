use clap::{Parser, Subcommand};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use rhizome_nursery_core::{
    detect_ecosystems, detect_primary_ecosystem, generate_configs, is_installed,
    merge_to_manifest, preview_configs, pull_configs, CliSchemaProvider, Lockfile, Manifest,
    SchemaProvider,
};
use rhizome_nursery_seed::{SeedResolver, VariableResolver};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::mpsc;
use std::time::Duration;

#[derive(Parser)]
#[command(name = "nursery")]
#[command(about = "A configuration manager for the Rhizome ecosystem")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Generate per-tool config files from nursery.toml
    Generate {
        /// Path to the manifest file
        #[arg(short, long, default_value = "nursery.toml")]
        manifest: PathBuf,

        /// Only validate, don't write files
        #[arg(long)]
        check: bool,

        /// Show what would change without writing
        #[arg(long)]
        diff: bool,

        /// Watch for changes and regenerate
        #[arg(long)]
        watch: bool,
    },

    /// Sync configs between nursery.toml and tool config files
    Config {
        #[command(subcommand)]
        action: ConfigAction,
    },

    /// Create a new project from a seed
    New {
        /// Project name
        name: String,

        /// Seed template to use
        #[arg(short, long, default_value = "creation")]
        seed: String,

        /// Set a variable (can be repeated)
        #[arg(long = "var", value_name = "KEY=VALUE", value_parser = parse_var)]
        vars: Vec<(String, String)>,

        /// Skip variable substitution
        #[arg(long)]
        raw: bool,

        /// Don't prompt for missing variables
        #[arg(long)]
        no_prompt: bool,
    },

    /// List available seed templates
    Seeds,

    /// Manage tool dependencies
    Tools {
        #[command(subcommand)]
        action: ToolsAction,
    },
}

#[derive(Subcommand)]
enum ToolsAction {
    /// Check if required tools are installed
    Check {
        /// Path to the manifest file
        #[arg(short, long, default_value = "nursery.toml")]
        manifest: PathBuf,
    },

    /// Install missing tools
    Install {
        /// Path to the manifest file
        #[arg(short, long, default_value = "nursery.toml")]
        manifest: PathBuf,

        /// Only show what would be installed
        #[arg(long)]
        dry_run: bool,
    },

    /// Show detected package managers
    Ecosystems,
}

#[derive(Subcommand)]
enum ConfigAction {
    /// Push nursery.toml to tool config files (alias for generate)
    Push {
        /// Path to the manifest file
        #[arg(short, long, default_value = "nursery.toml")]
        manifest: PathBuf,
    },

    /// Pull tool config files into nursery.toml
    Pull {
        /// Path to the manifest file
        #[arg(short, long, default_value = "nursery.toml")]
        manifest: PathBuf,

        /// Tools to pull (if not specified, pulls all from manifest)
        #[arg(value_name = "TOOL")]
        tools: Vec<String>,

        /// Don't write, just show what would be pulled
        #[arg(long)]
        dry_run: bool,
    },
}

fn parse_var(s: &str) -> Result<(String, String), String> {
    let pos = s
        .find('=')
        .ok_or_else(|| format!("invalid KEY=VALUE: no '=' found in '{s}'"))?;
    Ok((s[..pos].to_string(), s[pos + 1..].to_string()))
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    match cli.command {
        Command::Generate {
            manifest,
            check,
            diff,
            watch,
        } => {
            if watch {
                cmd_watch(&manifest)
            } else {
                cmd_generate(&manifest, check, diff)
            }
        }
        Command::Config { action } => match action {
            ConfigAction::Push { manifest } => cmd_generate(&manifest, false, false),
            ConfigAction::Pull {
                manifest,
                tools,
                dry_run,
            } => cmd_pull(&manifest, tools, dry_run),
        },
        Command::New {
            name,
            seed,
            vars,
            raw,
            no_prompt,
        } => cmd_new(&name, &seed, vars, raw, no_prompt),
        Command::Seeds => cmd_seeds(),
        Command::Tools { action } => match action {
            ToolsAction::Check { manifest } => cmd_tools_check(&manifest),
            ToolsAction::Install { manifest, dry_run } => cmd_tools_install(&manifest, dry_run),
            ToolsAction::Ecosystems => cmd_tools_ecosystems(),
        },
    }
}

fn cmd_generate(path: &PathBuf, check_only: bool, diff_mode: bool) -> ExitCode {
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
                for result in &results {
                    println!("generated: {} -> {}", result.tool, result.path.display());
                }
                println!("generated {} config(s)", results.len());
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

fn cmd_watch(path: &PathBuf) -> ExitCode {
    // Run initial generation
    println!("watching: {}", path.display());
    if cmd_generate(path, false, false) == ExitCode::FAILURE {
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
                if cmd_generate(path, false, false) == ExitCode::FAILURE {
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

fn cmd_pull(path: &PathBuf, tools: Vec<String>, dry_run: bool) -> ExitCode {
    let provider = CliSchemaProvider;
    let base_dir = path.parent().unwrap_or(std::path::Path::new("."));

    // Determine which tools to pull
    let tool_names: Vec<String> = if tools.is_empty() {
        // Try to read existing manifest to get tool list
        match Manifest::from_path(path) {
            Ok(m) => m.tool_configs.keys().cloned().collect(),
            Err(_) => {
                eprintln!("error: no tools specified and no existing manifest");
                eprintln!("hint: specify tools to pull, e.g., 'nursery config pull siphon dew'");
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

fn cmd_new(
    name: &str,
    seed_name: &str,
    cli_vars: Vec<(String, String)>,
    raw: bool,
    no_prompt: bool,
) -> ExitCode {
    let resolver = SeedResolver::new();

    let seed = match resolver.get(seed_name) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: {e}");
            eprintln!("hint: run 'nursery seeds' to list available templates");
            return ExitCode::FAILURE;
        }
    };

    // Build variables
    let mut cli_map: HashMap<String, String> = cli_vars.into_iter().collect();
    // Always include name from CLI arg
    cli_map.insert("name".to_string(), name.to_string());

    let vars = if raw {
        HashMap::new()
    } else {
        let var_resolver = VariableResolver::new()
            .with_cli(cli_map)
            .with_global_config()
            .with_seed_defaults(seed.variables.clone())
            .with_inferred();

        // Find required variables (those without defaults)
        let required: Vec<String> = seed
            .variables
            .iter()
            .filter(|(_, default)| default.is_none())
            .map(|(name, _)| name.clone())
            .collect();

        match var_resolver.resolve_all(&required) {
            Ok(vars) => vars,
            Err(missing) => {
                if no_prompt {
                    eprintln!("error: missing required variable: {missing}");
                    eprintln!("hint: use --var {missing}=VALUE");
                    return ExitCode::FAILURE;
                }

                // Prompt for missing variable
                match prompt_variable(&missing) {
                    Ok(value) => {
                        let mut cli_map: HashMap<String, String> = HashMap::new();
                        cli_map.insert("name".to_string(), name.to_string());
                        cli_map.insert(missing, value);

                        let var_resolver = VariableResolver::new()
                            .with_cli(cli_map)
                            .with_global_config()
                            .with_seed_defaults(seed.variables.clone())
                            .with_inferred();

                        match var_resolver.resolve_all(&required) {
                            Ok(vars) => vars,
                            Err(missing) => {
                                eprintln!("error: missing required variable: {missing}");
                                return ExitCode::FAILURE;
                            }
                        }
                    }
                    Err(e) => {
                        eprintln!("error: {e}");
                        return ExitCode::FAILURE;
                    }
                }
            }
        }
    };

    let dest = PathBuf::from(name);

    match seed.scaffold(&dest, &vars, raw) {
        Ok(()) => {
            println!("created project '{name}' from seed '{seed_name}'");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn prompt_variable(name: &str) -> io::Result<String> {
    print!("{name}: ");
    io::stdout().flush()?;

    let mut value = String::new();
    io::stdin().read_line(&mut value)?;

    Ok(value.trim().to_string())
}

fn cmd_seeds() -> ExitCode {
    let resolver = SeedResolver::new();

    match resolver.list() {
        Ok(seeds) => {
            println!("Available seeds:");
            for seed in seeds {
                println!("  {:<15} {}", seed.name, seed.description);
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn cmd_tools_ecosystems() -> ExitCode {
    let ecosystems = detect_ecosystems();

    if ecosystems.is_empty() {
        println!("no supported package managers detected");
        return ExitCode::SUCCESS;
    }

    println!("Detected package managers:");
    for eco in &ecosystems {
        println!("  {}", eco.id());
    }

    if let Some(primary) = detect_primary_ecosystem() {
        println!("\nPrimary: {}", primary.id());
    }

    ExitCode::SUCCESS
}

fn cmd_tools_check(manifest_path: &PathBuf) -> ExitCode {
    let manifest = match Manifest::from_path(manifest_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    if manifest.tool_deps.is_empty() {
        println!("no tool dependencies configured");
        return ExitCode::SUCCESS;
    }

    let ecosystem = match detect_primary_ecosystem() {
        Some(e) => e,
        None => {
            eprintln!("error: no supported package manager detected");
            return ExitCode::FAILURE;
        }
    };

    // Try to load lockfile for package names
    let lockfile_path = manifest_path.with_file_name("nursery.lock");
    let lockfile = Lockfile::load_or_default(&lockfile_path);

    let mut all_ok = true;
    let mut missing = Vec::new();

    for (tool_name, dep) in &manifest.tool_deps {
        // Get package name from lockfile or use tool name
        let package_name = lockfile
            .get(tool_name, ecosystem.id())
            .map(|p| p.package.as_str())
            .unwrap_or(tool_name.as_str());

        let installed = is_installed(ecosystem, package_name);
        let status = if installed { "OK" } else { "MISSING" };
        let optional = if dep.optional { " (optional)" } else { "" };

        println!("  {tool_name}: {status}{optional}");

        if !installed && !dep.optional {
            all_ok = false;
            missing.push(package_name.to_string());
        }
    }

    if all_ok {
        println!("\nall required tools installed");
        ExitCode::SUCCESS
    } else {
        println!("\nmissing {} required tool(s)", missing.len());
        println!("run 'nursery tools install' to install them");
        ExitCode::FAILURE
    }
}

fn cmd_tools_install(manifest_path: &PathBuf, dry_run: bool) -> ExitCode {
    let manifest = match Manifest::from_path(manifest_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    if manifest.tool_deps.is_empty() {
        println!("no tool dependencies configured");
        return ExitCode::SUCCESS;
    }

    let ecosystem = match detect_primary_ecosystem() {
        Some(e) => e,
        None => {
            eprintln!("error: no supported package manager detected");
            return ExitCode::FAILURE;
        }
    };

    // Try to load lockfile for package names
    let lockfile_path = manifest_path.with_file_name("nursery.lock");
    let lockfile = Lockfile::load_or_default(&lockfile_path);

    // Find missing packages
    let mut missing: Vec<String> = Vec::new();

    for (tool_name, dep) in &manifest.tool_deps {
        if dep.optional {
            continue;
        }

        let package_name = lockfile
            .get(tool_name, ecosystem.id())
            .map(|p| p.package.clone())
            .unwrap_or_else(|| tool_name.clone());

        if !is_installed(ecosystem, &package_name) {
            missing.push(package_name);
        }
    }

    if missing.is_empty() {
        println!("all required tools already installed");
        return ExitCode::SUCCESS;
    }

    let packages: Vec<&str> = missing.iter().map(|s| s.as_str()).collect();
    let cmd_display = ecosystem.install_cmd_display(&packages);

    println!("Missing tools for {}:", ecosystem.id());
    for pkg in &missing {
        println!("  {pkg}");
    }
    println!("\nRun this command?\n");
    println!("  {cmd_display}");

    if dry_run {
        println!("\n(dry run, not executing)");
        return ExitCode::SUCCESS;
    }

    // Prompt for confirmation
    print!("\n[Y/n] ");
    io::stdout().flush().unwrap();

    let mut input = String::new();
    if io::stdin().read_line(&mut input).is_err() {
        eprintln!("error: failed to read input");
        return ExitCode::FAILURE;
    }

    let input = input.trim().to_lowercase();
    if !input.is_empty() && input != "y" && input != "yes" {
        println!("cancelled");
        return ExitCode::SUCCESS;
    }

    // Execute install command
    let cmd = ecosystem.install_cmd(&packages);
    println!("\nrunning: {}\n", cmd.join(" "));

    let status = std::process::Command::new(&cmd[0])
        .args(&cmd[1..])
        .status();

    match status {
        Ok(s) if s.success() => {
            println!("\ninstallation complete");
            ExitCode::SUCCESS
        }
        Ok(s) => {
            eprintln!("\ninstallation failed with exit code: {:?}", s.code());
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("\nfailed to run command: {e}");
            ExitCode::FAILURE
        }
    }
}
