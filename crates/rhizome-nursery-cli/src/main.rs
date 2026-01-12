mod commands;

use clap::{Parser, Subcommand};
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

    /// Initialize a new project from a seed template
    Init {
        /// Project name
        name: String,

        /// Seed template to use (default: creation)
        #[arg(default_value = "creation")]
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

        /// Include dev tools
        #[arg(long)]
        dev: bool,

        /// Include build dependencies
        #[arg(long)]
        build: bool,
    },

    /// Install missing tools
    Install {
        /// Path to the manifest file
        #[arg(short, long, default_value = "nursery.toml")]
        manifest: PathBuf,

        /// Only show what would be installed
        #[arg(long)]
        dry_run: bool,

        /// Include dev tools
        #[arg(long)]
        dev: bool,

        /// Include build dependencies
        #[arg(long)]
        build: bool,
    },

    /// Show detected package managers
    Ecosystems,

    /// Look up a tool's package names via Repology
    Lookup {
        /// Tool name to look up
        tool: String,
    },

    /// Resolve all tool dependencies and write lockfile
    Lock {
        /// Path to the manifest file
        #[arg(short, long, default_value = "nursery.toml")]
        manifest: PathBuf,
    },
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
                commands::generate::watch(&manifest)
            } else {
                commands::generate::run(&manifest, check, diff)
            }
        }
        Command::Config { action } => match action {
            ConfigAction::Push { manifest } => commands::generate::run(&manifest, false, false),
            ConfigAction::Pull {
                manifest,
                tools,
                dry_run,
            } => commands::config::pull(&manifest, tools, dry_run),
        },
        Command::Init {
            name,
            seed,
            vars,
            raw,
            no_prompt,
        } => commands::init::run(&name, &seed, vars, raw, no_prompt),
        Command::Seeds => commands::seeds::run(),
        Command::Tools { action } => match action {
            ToolsAction::Check {
                manifest,
                dev,
                build,
            } => commands::tools::check(&manifest, dev, build),
            ToolsAction::Install {
                manifest,
                dry_run,
                dev,
                build,
            } => commands::tools::install(&manifest, dry_run, dev, build),
            ToolsAction::Ecosystems => commands::tools::ecosystems(),
            ToolsAction::Lookup { tool } => commands::tools::lookup(&tool),
            ToolsAction::Lock { manifest } => commands::tools::lock(&manifest),
        },
    }
}
