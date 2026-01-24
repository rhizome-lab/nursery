# Seeds

Seeds are starter templates for common rhi workflows.

## Available Seeds

### seed-archaeology

For lifting legacy games from obsolete runtimes.

```bash
myenv new my-remake --seed archaeology
```

Includes:
- Siphon configuration for asset extraction
- Sap pipeline for asset processing
- Lotus runtime for playback

### seed-creation

For new Lotus projects from scratch.

```bash
myenv new my-game --seed creation
```

Includes:
- Lotus runtime configuration
- Basic project structure
- Development workflow

### seed-lab

Full ecosystem sandbox with all tools configured.

```bash
myenv new my-lab --seed lab
```

Includes:
- All rhi tools pre-configured
- Example pipelines
- Documentation stubs

## Custom Seeds

Create your own seeds by placing a `rhizome.toml` template in `~/.config/myenv/seeds/`.
