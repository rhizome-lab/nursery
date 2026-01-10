# Getting Started

## Installation

```bash
cargo install rhizome-nursery-cli
```

## Create a Project

Start from a seed:

```bash
nursery new my-project --seed creation
cd my-project
```

This creates a new directory with a pre-configured `nursery.toml` and starter files.

## The Manifest

Open `nursery.toml` to see how tools are configured:

```toml
[project]
name = "my-project"
version = "0.1.0"

[lotus]
target = "web-wasm"
port = 8080
```

## Run

```bash
nursery run
```

Nursery reads the manifest, ensures dependencies are satisfied, and launches the appropriate tools.
