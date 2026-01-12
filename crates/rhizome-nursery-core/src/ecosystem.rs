//! Ecosystem detection and package manager interaction.

use std::process::Command;

/// Known package manager ecosystems.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Ecosystem {
    /// Arch Linux pacman
    Pacman,
    /// Debian/Ubuntu apt
    Apt,
    /// Fedora/RHEL dnf
    Dnf,
    /// Alpine apk
    Apk,
    /// macOS/Linux Homebrew
    Brew,
    /// NixOS/Nix
    Nix,
    /// Windows Scoop
    Scoop,
    /// Windows winget
    Winget,
    /// Rust cargo
    Cargo,
}

impl Ecosystem {
    /// Get the string identifier for this ecosystem.
    pub fn id(&self) -> &'static str {
        match self {
            Ecosystem::Pacman => "pacman",
            Ecosystem::Apt => "apt",
            Ecosystem::Dnf => "dnf",
            Ecosystem::Apk => "apk",
            Ecosystem::Brew => "brew",
            Ecosystem::Nix => "nix",
            Ecosystem::Scoop => "scoop",
            Ecosystem::Winget => "winget",
            Ecosystem::Cargo => "cargo",
        }
    }

    /// Parse from string identifier.
    pub fn from_id(id: &str) -> Option<Self> {
        match id {
            "pacman" => Some(Ecosystem::Pacman),
            "apt" => Some(Ecosystem::Apt),
            "dnf" => Some(Ecosystem::Dnf),
            "apk" => Some(Ecosystem::Apk),
            "brew" => Some(Ecosystem::Brew),
            "nix" => Some(Ecosystem::Nix),
            "scoop" => Some(Ecosystem::Scoop),
            "winget" => Some(Ecosystem::Winget),
            "cargo" => Some(Ecosystem::Cargo),
            _ => None,
        }
    }

    /// Get the command to check if a package is installed.
    pub fn check_installed_cmd(&self, package: &str) -> Vec<String> {
        match self {
            Ecosystem::Pacman => vec!["pacman".into(), "-Q".into(), package.into()],
            Ecosystem::Apt => vec!["dpkg".into(), "-s".into(), package.into()],
            Ecosystem::Dnf => vec!["rpm".into(), "-q".into(), package.into()],
            Ecosystem::Apk => vec!["apk".into(), "info".into(), "-e".into(), package.into()],
            Ecosystem::Brew => vec!["brew".into(), "list".into(), package.into()],
            Ecosystem::Nix => vec!["nix-env".into(), "-q".into(), package.into()],
            Ecosystem::Scoop => vec!["scoop".into(), "list".into(), package.into()],
            Ecosystem::Winget => vec![
                "winget".into(),
                "list".into(),
                "--id".into(),
                package.into(),
            ],
            Ecosystem::Cargo => vec!["cargo".into(), "install".into(), "--list".into()],
        }
    }

    /// Get the command to install packages.
    pub fn install_cmd(&self, packages: &[&str]) -> Vec<String> {
        match self {
            Ecosystem::Pacman => {
                let mut cmd = vec![
                    "sudo".into(),
                    "pacman".into(),
                    "-S".into(),
                    "--noconfirm".into(),
                ];
                cmd.extend(packages.iter().map(|s| s.to_string()));
                cmd
            }
            Ecosystem::Apt => {
                let mut cmd = vec!["sudo".into(), "apt".into(), "install".into(), "-y".into()];
                cmd.extend(packages.iter().map(|s| s.to_string()));
                cmd
            }
            Ecosystem::Dnf => {
                let mut cmd = vec!["sudo".into(), "dnf".into(), "install".into(), "-y".into()];
                cmd.extend(packages.iter().map(|s| s.to_string()));
                cmd
            }
            Ecosystem::Apk => {
                let mut cmd = vec!["sudo".into(), "apk".into(), "add".into()];
                cmd.extend(packages.iter().map(|s| s.to_string()));
                cmd
            }
            Ecosystem::Brew => {
                let mut cmd = vec!["brew".into(), "install".into()];
                cmd.extend(packages.iter().map(|s| s.to_string()));
                cmd
            }
            Ecosystem::Nix => {
                let mut cmd = vec!["nix-env".into(), "-iA".into()];
                cmd.extend(packages.iter().map(|p| format!("nixpkgs.{p}")));
                cmd
            }
            Ecosystem::Scoop => {
                let mut cmd = vec!["scoop".into(), "install".into()];
                cmd.extend(packages.iter().map(|s| s.to_string()));
                cmd
            }
            Ecosystem::Winget => {
                let mut cmd = vec!["winget".into(), "install".into()];
                cmd.extend(packages.iter().map(|s| s.to_string()));
                cmd
            }
            Ecosystem::Cargo => {
                let mut cmd = vec!["cargo".into(), "install".into()];
                cmd.extend(packages.iter().map(|s| s.to_string()));
                cmd
            }
        }
    }

    /// Format install command for display (without --noconfirm etc.).
    pub fn install_cmd_display(&self, packages: &[&str]) -> String {
        let pkgs = packages.join(" ");
        match self {
            Ecosystem::Pacman => format!("sudo pacman -S {pkgs}"),
            Ecosystem::Apt => format!("sudo apt install {pkgs}"),
            Ecosystem::Dnf => format!("sudo dnf install {pkgs}"),
            Ecosystem::Apk => format!("sudo apk add {pkgs}"),
            Ecosystem::Brew => format!("brew install {pkgs}"),
            Ecosystem::Nix => {
                let nix_pkgs: Vec<_> = packages.iter().map(|p| format!("nixpkgs.{p}")).collect();
                format!("nix-env -iA {}", nix_pkgs.join(" "))
            }
            Ecosystem::Scoop => format!("scoop install {pkgs}"),
            Ecosystem::Winget => format!("winget install {pkgs}"),
            Ecosystem::Cargo => format!("cargo install {pkgs}"),
        }
    }

    /// Whether this ecosystem typically requires sudo.
    pub fn needs_sudo(&self) -> bool {
        matches!(
            self,
            Ecosystem::Pacman | Ecosystem::Apt | Ecosystem::Dnf | Ecosystem::Apk
        )
    }
}

/// Detect available ecosystems on the current system.
pub fn detect_ecosystems() -> Vec<Ecosystem> {
    let mut found = Vec::new();

    let candidates = [
        (Ecosystem::Pacman, "pacman"),
        (Ecosystem::Apt, "apt"),
        (Ecosystem::Dnf, "dnf"),
        (Ecosystem::Apk, "apk"),
        (Ecosystem::Brew, "brew"),
        (Ecosystem::Nix, "nix-env"),
        (Ecosystem::Scoop, "scoop"),
        (Ecosystem::Winget, "winget"),
        (Ecosystem::Cargo, "cargo"),
    ];

    for (ecosystem, binary) in candidates {
        if command_exists(binary) {
            found.push(ecosystem);
        }
    }

    found
}

/// Detect the primary ecosystem (first available system package manager).
pub fn detect_primary_ecosystem() -> Option<Ecosystem> {
    // Prefer system package managers over language-specific ones
    let priority = [
        Ecosystem::Pacman,
        Ecosystem::Apt,
        Ecosystem::Dnf,
        Ecosystem::Apk,
        Ecosystem::Nix,
        Ecosystem::Brew,
        Ecosystem::Scoop,
        Ecosystem::Winget,
    ];

    let available = detect_ecosystems();
    priority.into_iter().find(|e| available.contains(e))
}

/// Check if a command exists in PATH.
fn command_exists(cmd: &str) -> bool {
    Command::new("which")
        .arg(cmd)
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

/// Check if a package is installed via an ecosystem.
pub fn is_installed(ecosystem: Ecosystem, package: &str) -> bool {
    let cmd = ecosystem.check_installed_cmd(package);
    if cmd.is_empty() {
        return false;
    }

    Command::new(&cmd[0])
        .args(&cmd[1..])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ecosystem_id_roundtrip() {
        let ecosystems = [
            Ecosystem::Pacman,
            Ecosystem::Apt,
            Ecosystem::Dnf,
            Ecosystem::Brew,
            Ecosystem::Nix,
            Ecosystem::Cargo,
        ];

        for eco in ecosystems {
            assert_eq!(Ecosystem::from_id(eco.id()), Some(eco));
        }
    }

    #[test]
    fn install_cmd_display() {
        assert_eq!(
            Ecosystem::Pacman.install_cmd_display(&["ripgrep", "fd"]),
            "sudo pacman -S ripgrep fd"
        );
        assert_eq!(
            Ecosystem::Brew.install_cmd_display(&["ripgrep"]),
            "brew install ripgrep"
        );
        assert_eq!(
            Ecosystem::Nix.install_cmd_display(&["ripgrep"]),
            "nix-env -iA nixpkgs.ripgrep"
        );
    }
}
