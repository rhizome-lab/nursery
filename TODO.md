# TODO

## Backlog

- [ ] Tool registry integration
  - Fetch package name mappings from registry
  - `nursery tools lock` command to generate lockfile

- [ ] Config format detection
  - Auto-detect format from file extension if tool doesn't specify

## Done

- [x] Tool dependency management (`nursery tools check/install/ecosystems`)
- [x] `nursery config pull` / `nursery config push`
  - Pull: Read existing tool configs back into nursery.toml
  - Push: Alias for generate
  - Enables round-tripping configs
- [x] Watch mode
  - `nursery generate --watch`
  - Regenerate on nursery.toml changes
- [x] Diff mode
  - `nursery generate --diff`
  - Show what would change without writing
- [x] Basic manifest parsing with `[variables]` section
- [x] Seed scaffolding with templates
- [x] Variable resolution (CLI, config, defaults, inferred)
- [x] User-local seeds (~/.config/nursery/seeds/)
- [x] Tool schema convention (`config_path`, `format`, `schema`)
- [x] Config generation (`nursery generate`)
- [x] Design documentation

## Complexity Hotspots (threshold >21)
- [ ] `crates/rhizome-nursery-cli/src/main.rs:cmd_generate` (25)
- [ ] `crates/rhizome-nursery-cli/src/main.rs:cmd_tools_install` (24)

## Maybe

- [ ] Transformation hooks
  - VFS-based or hook-based arbitrary editing
  - before_push / after_push / before_pull / after_pull
  - Could use spore for Lua scripting
