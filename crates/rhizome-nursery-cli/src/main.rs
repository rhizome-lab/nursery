use clap::{Parser, Subcommand};
use rhizome_nursery_core::{
    generate_configs, merge_to_manifest, pull_configs, CliSchemaProvider, Manifest, SchemaProvider,
};
use rhizome_nursery_seed::{SeedResolver, VariableResolver};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

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
        Command::Generate { manifest, check } => cmd_generate(&manifest, check),
        Command::Config { action } => match action {
            ConfigAction::Push { manifest } => cmd_generate(&manifest, false),
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
    }
}

fn cmd_generate(path: &PathBuf, check_only: bool) -> ExitCode {
    let manifest = match Manifest::from_path(path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    if manifest.tools.is_empty() {
        println!("no tools configured");
        return ExitCode::SUCCESS;
    }

    let provider = CliSchemaProvider;
    let base_dir = path.parent().unwrap_or(std::path::Path::new("."));

    if check_only {
        // Just validate, don't write
        for tool_name in manifest.tools.keys() {
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

fn cmd_pull(path: &PathBuf, tools: Vec<String>, dry_run: bool) -> ExitCode {
    let provider = CliSchemaProvider;
    let base_dir = path.parent().unwrap_or(std::path::Path::new("."));

    // Determine which tools to pull
    let tool_names: Vec<String> = if tools.is_empty() {
        // Try to read existing manifest to get tool list
        match Manifest::from_path(path) {
            Ok(m) => m.tools.keys().cloned().collect(),
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
