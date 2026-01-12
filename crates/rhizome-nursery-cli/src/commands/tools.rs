//! Tools command implementations.

use rhizome_nursery_core::{
    detect_ecosystems, detect_primary_ecosystem, is_installed, Ecosystem, LockedPackage,
    LockedTool, Lockfile, Manifest, RepologyClient, ToolDep,
};
use std::collections::BTreeMap;
use std::io::{self, Write};
use std::path::PathBuf;
use std::process::ExitCode;

pub fn ecosystems() -> ExitCode {
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

pub fn check(manifest_path: &PathBuf, include_dev: bool, include_build: bool) -> ExitCode {
    let manifest = match Manifest::from_path(manifest_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

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

    // Helper to check a set of deps
    let mut check_deps = |deps: &BTreeMap<String, ToolDep>, section: &str| {
        if !deps.is_empty() {
            println!("\n[{section}]");
        }
        for (tool_name, dep) in deps {
            // Get package name: override > lockfile > tool name
            let package_name = dep
                .overrides
                .get(ecosystem.id())
                .map(|s| s.as_str())
                .or_else(|| {
                    lockfile
                        .get(tool_name, ecosystem.id())
                        .map(|p| p.package.as_str())
                })
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
    };

    // Check runtime tools
    check_deps(&manifest.tool_deps, "tools");

    // Check dev tools if requested
    if include_dev {
        check_deps(&manifest.dev_tool_deps, "dev-tools");
    }

    // Check build deps if requested
    if include_build {
        check_deps(&manifest.build_deps, "build-deps");
    }

    if manifest.tool_deps.is_empty()
        && (!include_dev || manifest.dev_tool_deps.is_empty())
        && (!include_build || manifest.build_deps.is_empty())
    {
        println!("no dependencies configured");
        return ExitCode::SUCCESS;
    }

    if all_ok {
        println!("\nall required dependencies installed");
        ExitCode::SUCCESS
    } else {
        println!("\nmissing {} required dependency(ies)", missing.len());
        println!("run 'nursery tools install' to install them");
        ExitCode::FAILURE
    }
}

pub fn install(
    manifest_path: &PathBuf,
    dry_run: bool,
    include_dev: bool,
    include_build: bool,
) -> ExitCode {
    let manifest = match Manifest::from_path(manifest_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

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

    // Helper to find missing packages in a dep set
    let find_missing = |deps: &BTreeMap<String, ToolDep>| -> Vec<String> {
        deps.iter()
            .filter(|(_, dep)| !dep.optional)
            .filter_map(|(tool_name, dep)| {
                // Get package name: override > lockfile > tool name
                let package_name = dep
                    .overrides
                    .get(ecosystem.id())
                    .cloned()
                    .or_else(|| {
                        lockfile
                            .get(tool_name, ecosystem.id())
                            .map(|p| p.package.clone())
                    })
                    .unwrap_or_else(|| tool_name.clone());

                if !is_installed(ecosystem, &package_name) {
                    Some(package_name)
                } else {
                    None
                }
            })
            .collect()
    };

    // Collect missing packages
    let mut missing: Vec<String> = find_missing(&manifest.tool_deps);

    if include_dev {
        missing.extend(find_missing(&manifest.dev_tool_deps));
    }

    if include_build {
        missing.extend(find_missing(&manifest.build_deps));
    }

    // Deduplicate
    missing.sort();
    missing.dedup();

    if missing.is_empty() {
        println!("all required dependencies already installed");
        return ExitCode::SUCCESS;
    }

    let packages: Vec<&str> = missing.iter().map(|s| s.as_str()).collect();
    let cmd_display = ecosystem.install_cmd_display(&packages);

    println!("Missing dependencies for {}:", ecosystem.id());
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

pub fn lookup(tool: &str) -> ExitCode {
    let client = RepologyClient::new();

    println!("Looking up '{tool}' via Repology...\n");

    match client.lookup(tool) {
        Ok(info) => {
            if info.packages.is_empty() {
                println!("No packages found for '{tool}'");
                println!("hint: the project name on Repology may differ");
                return ExitCode::SUCCESS;
            }

            if let Some(binname) = &info.binname {
                println!("Binary: {binname}");
            }

            println!("Packages:");
            for (ecosystem, pkg) in &info.packages {
                println!("  {:<12} {} ({})", ecosystem.id(), pkg.name, pkg.version);
            }

            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

pub fn lock(manifest_path: &PathBuf) -> ExitCode {
    let manifest = match Manifest::from_path(manifest_path) {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: {e}");
            return ExitCode::FAILURE;
        }
    };

    let total_deps =
        manifest.tool_deps.len() + manifest.dev_tool_deps.len() + manifest.build_deps.len();

    if total_deps == 0 {
        println!("no dependencies to lock");
        return ExitCode::SUCCESS;
    }

    let client = RepologyClient::new();
    let mut lockfile = Lockfile::default();

    // Determine which ecosystems to include
    let detected = detect_ecosystems();
    let ecosystems: Vec<Ecosystem> = if let Some(ref eco_list) = manifest.ecosystems {
        eco_list
            .iter()
            .filter_map(|id| Ecosystem::from_id(id))
            .collect()
    } else {
        detected.clone()
    };

    if ecosystems.is_empty() {
        eprintln!("error: no ecosystems specified or detected");
        return ExitCode::FAILURE;
    }

    println!(
        "Resolving dependencies for ecosystems: {}",
        ecosystems
            .iter()
            .map(|e| e.id())
            .collect::<Vec<_>>()
            .join(", ")
    );

    // Helper to lock a set of deps
    let mut lock_deps = |deps: &BTreeMap<String, ToolDep>, section: &str| {
        if deps.is_empty() {
            return;
        }
        println!("\n[{section}]");

        for (tool_name, dep) in deps {
            print!("  {tool_name}... ");
            io::stdout().flush().unwrap();

            match client.lookup(tool_name) {
                Ok(info) => {
                    let mut eco_packages = BTreeMap::new();

                    for eco in &ecosystems {
                        // Check for manual override first
                        if let Some(override_pkg) = dep.overrides.get(eco.id()) {
                            eco_packages.insert(
                                eco.id().to_string(),
                                LockedPackage {
                                    package: override_pkg.clone(),
                                    version: "override".to_string(),
                                    hash: None,
                                    archive: None,
                                    nixpkgs: None,
                                },
                            );
                        } else if let Some(pkg) = info.packages.get(eco) {
                            eco_packages.insert(
                                eco.id().to_string(),
                                LockedPackage {
                                    package: pkg.name.clone(),
                                    version: pkg.version.clone(),
                                    hash: None,
                                    archive: None,
                                    nixpkgs: None,
                                },
                            );
                        }
                    }

                    if eco_packages.is_empty() {
                        println!("not found");
                    } else {
                        println!("ok ({} ecosystem(s))", eco_packages.len());

                        lockfile.tools.insert(
                            tool_name.clone(),
                            LockedTool {
                                source: format!("repology:{tool_name}"),
                                constraint: dep.version.clone(),
                                ecosystems: eco_packages,
                            },
                        );
                    }
                }
                Err(e) => {
                    // If not found on Repology but has overrides, use those
                    if !dep.overrides.is_empty() {
                        let mut eco_packages = BTreeMap::new();
                        for eco in &ecosystems {
                            if let Some(override_pkg) = dep.overrides.get(eco.id()) {
                                eco_packages.insert(
                                    eco.id().to_string(),
                                    LockedPackage {
                                        package: override_pkg.clone(),
                                        version: "override".to_string(),
                                        hash: None,
                                        archive: None,
                                        nixpkgs: None,
                                    },
                                );
                            }
                        }
                        if !eco_packages.is_empty() {
                            println!("ok (override)");
                            lockfile.tools.insert(
                                tool_name.clone(),
                                LockedTool {
                                    source: "override".to_string(),
                                    constraint: dep.version.clone(),
                                    ecosystems: eco_packages,
                                },
                            );
                        } else {
                            println!("error: {e}");
                        }
                    } else {
                        println!("error: {e}");
                    }
                }
            }
        }
    };

    // Lock all dependency types
    lock_deps(&manifest.tool_deps, "tools");
    lock_deps(&manifest.dev_tool_deps, "dev-tools");
    lock_deps(&manifest.build_deps, "build-deps");

    // Write lockfile
    let lockfile_path = manifest_path.with_file_name("nursery.lock");

    match lockfile.write(&lockfile_path) {
        Ok(()) => {
            println!("\nWrote {}", lockfile_path.display());
            println!("Locked {} dependency(ies)", lockfile.tools.len());
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: failed to write lockfile: {e}");
            ExitCode::FAILURE
        }
    }
}
