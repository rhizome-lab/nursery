//! Seeds command implementation.

use rhi_myenv_seed::SeedResolver;
use std::process::ExitCode;

pub fn run() -> ExitCode {
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
