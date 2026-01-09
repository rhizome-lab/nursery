# Seeds

Seeds are starter templates for common Rhizome workflows.

## Available Seeds

### seed-archaeology

For lifting legacy games from obsolete runtimes.

```bash
nursery new my-remake --seed archaeology
```

Includes:
- Winnow configuration for asset extraction
- Sap pipeline for asset processing
- Lotus runtime for playback

### seed-creation

For new Lotus projects from scratch.

```bash
nursery new my-game --seed creation
```

Includes:
- Lotus runtime configuration
- Basic project structure
- Development workflow

### seed-lab

Full ecosystem sandbox with all tools configured.

```bash
nursery new my-lab --seed lab
```

Includes:
- All Rhizome tools pre-configured
- Example pipelines
- Documentation stubs

## Custom Seeds

Create your own seeds by placing a `rhizome.toml` template in `~/.config/nursery/seeds/`.
