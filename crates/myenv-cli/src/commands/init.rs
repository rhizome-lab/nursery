//! Init command implementation.

use myenv_seed::{SeedResolver, VariableResolver};
use std::collections::HashMap;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

pub fn run(
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
            eprintln!("hint: run 'myenv seeds' to list available templates");
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
