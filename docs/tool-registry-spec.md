# Tool Registry Spec

Specification for the cross-platform package registry that maps tool names to package manager packages.

## Overview

The registry is a machine-readable database mapping canonical tool names to their package names, versions, and metadata across multiple ecosystems (apt, pacman, nix, brew, etc.).

Nursery consumes this registry to resolve `[tools]` entries in `nursery.toml` to concrete install commands.

## Registry Format

JSONL (JSON Lines) for streaming/incremental updates. One tool per line.

```jsonl
{"tool":"ripgrep","source":"github:BurntSushi/ripgrep","bin":["rg"],"ecosystems":{"apt":{"package":"ripgrep"},"pacman":{"package":"ripgrep"},"nix":{"attr":"ripgrep"},"brew":{"formula":"ripgrep"},"cargo":{"crate":"ripgrep"}}}
{"tool":"fd","source":"github:sharkdp/fd","bin":["fd"],"ecosystems":{"apt":{"package":"fd-find","bin":["fdfind"]},"pacman":{"package":"fd"},"nix":{"attr":"fd"},"brew":{"formula":"fd"}}}
```

### Tool Entry Schema

```json
{
  "tool": "string, canonical name",
  "source": "string, authoritative source (github:owner/repo, gitlab:..., url:...)",
  "description": "string, optional",
  "bin": ["array of binary names produced"],
  "ecosystems": {
    "<ecosystem>": {
      "package": "string, package name in this ecosystem",
      "bin": ["optional, if binary names differ from canonical"],
      "notes": "optional, special instructions"
    }
  }
}
```

### Supported Ecosystems

| ID | Package Manager | Version Source |
|----|-----------------|----------------|
| `apt` | apt (Debian/Ubuntu) | `apt-cache policy` |
| `pacman` | pacman (Arch) | `pacman -Si`, ALA for historical |
| `nix` | Nix | nixpkgs, pinnable |
| `brew` | Homebrew (macOS/Linux) | `brew info` |
| `dnf` | dnf (Fedora/RHEL) | `dnf info` |
| `apk` | apk (Alpine) | `apk info` |
| `scoop` | Scoop (Windows) | `scoop info` |
| `winget` | winget (Windows) | `winget show` |
| `cargo` | cargo install | crates.io |
| `npm` | npm | npmjs.com |
| `pip` | pip | pypi.org |

## Version Index

Separate file tracking available versions per ecosystem. Updated more frequently than the main registry.

```jsonl
{"tool":"ripgrep","ecosystem":"apt","versions":[{"version":"14.1.0-1","distros":["bookworm","noble"]},{"version":"13.0.0-2","distros":["bullseye","jammy"]}]}
{"tool":"ripgrep","ecosystem":"pacman","versions":[{"version":"14.1.0-1","ala":"https://archive.archlinux.org/packages/r/ripgrep/ripgrep-14.1.0-1-x86_64.pkg.tar.zst"}]}
{"tool":"ripgrep","ecosystem":"nix","versions":[{"version":"14.1.0","nixpkgs":"github:NixOS/nixpkgs/nixos-24.05","hash":"sha256-..."}]}
```

## Verification

How we verify that packages across ecosystems are the same tool:

### Primary: Source Match

Compare the upstream source URL. Most packages include:
- `Homepage` field
- `Source` / `Repository` URL
- Build scripts pointing to source

If multiple ecosystems point to the same GitHub repo, high confidence they're the same tool.

### Secondary: Binary Match

Verify the package produces the expected binaries:
- Check package file lists
- Compare binary names to canonical `bin` array

### Tertiary: Description Similarity

NLP/fuzzy match on descriptions as a weak signal.

### Confidence Levels

- `verified` — Source URL matches, binaries match
- `likely` — Source URL matches, binaries unchecked
- `name-only` — Same package name, unverified (flag for review)
- `manual` — Human-verified override

## Registry CI Pipeline

```
┌─────────────────┐
│  Fetch Metadata │ ← Query each ecosystem's package index
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│    Correlate    │ ← Group by source URL
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│     Verify      │ ← Check binaries, descriptions
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Flag Conflicts │ ← Same name, different source → manual review
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│    Publish      │ ← registry.jsonl, versions.jsonl
└─────────────────┘
```

### Fetching

Each ecosystem needs a fetcher:

- **apt**: Parse `Sources` files from mirrors, extract Homepage/Vcs-Git
- **pacman**: Parse `.SRCINFO` from AUR/repos, extract `url`, `source`
- **nix**: Parse nixpkgs, extract `meta.homepage`, `src` derivation
- **brew**: Parse formula JSON API, extract `homepage`, `url`
- **cargo**: Query crates.io API, extract `repository`

### Scheduling

- Full rebuild: weekly
- Version updates: daily
- On-demand for new tool requests

## Nursery Integration

### nursery.toml

```toml
[tools]
ripgrep = ">=14"
fd = "*"
jq = "=1.7"

# Optional: limit ecosystems in lockfile
ecosystems = ["pacman", "nix"]
```

### nursery.lock

```toml
# Auto-generated, do not edit

[ripgrep]
source = "github:BurntSushi/ripgrep"
constraint = ">=14"

[ripgrep.pacman]
package = "ripgrep"
version = "14.1.0-1"
ala = "https://archive.archlinux.org/packages/r/ripgrep/ripgrep-14.1.0-1-x86_64.pkg.tar.zst"

[ripgrep.nix]
attr = "ripgrep"
version = "14.1.0"
nixpkgs = "github:NixOS/nixpkgs/abc123def"
hash = "sha256-..."
```

### Commands

- `nursery tools check` — Verify installed tools match constraints
- `nursery tools lock` — Resolve and write lockfile
- `nursery tools install` — Install missing tools via local ecosystem
- `nursery tools install --dry-run` — Show commands without running

### Install Flow

1. Detect local ecosystem (pacman, apt, brew, etc.)
2. Read lockfile for that ecosystem's packages
3. Check what's already installed
4. Build install command for missing packages
5. **Prompt user** with command to run
6. Execute (with sudo if needed) on confirmation

```
$ nursery tools install

Missing tools for pacman:
  ripgrep 14.1.0-1
  fd 10.1.0-1

Run this command?

  sudo pacman -S ripgrep fd

[Y/n]
```

## Open Questions

- [ ] How to handle tools not in any ecosystem? (fallback to direct download?)
- [ ] AUR vs official repos for Arch?
- [ ] Flatpak/Snap as ecosystems?
- [ ] How to handle conflicting package names? (different tools with same name)
- [ ] Signed registry for security?

## Implementation Notes

The registry tooling is a separate project from nursery. Suggested structure:

```
rhizome-registry/
  fetchers/
    apt.rs
    pacman.rs
    nix.rs
    brew.rs
    ...
  correlate.rs
  verify.rs
  publish.rs
  registry.jsonl      # output
  versions.jsonl      # output
```

Rust recommended for consistency with nursery, but could be Python for easier scraping.
