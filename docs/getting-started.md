# Getting Started

## Installation

```bash
cargo install rhizome-myenv-cli
```

## Create a Project

Start from a seed:

```bash
myenv new my-project --seed creation
cd my-project
```

This creates a new directory with a pre-configured `myenv.toml` and starter files.

## The Manifest

Open `myenv.toml` to see how tools are configured:

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
myenv run
```

myenv reads the manifest, ensures dependencies are satisfied, and launches the appropriate tools.
