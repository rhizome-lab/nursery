# TODO

## Backlog

- [x] `nursery config pull` / `nursery config push`
  - Pull: Read existing tool configs back into nursery.toml
  - Push: Alias for generate (or replace it?)
  - Enables round-tripping configs

- [ ] Lua templating upgrade path
  - Simple `{{var}}` for most cases
  - `{{lua: expression}}` for complex logic
  - Uses spore runtime

- [ ] Config format detection
  - Auto-detect format from file extension if tool doesn't specify

- [ ] Watch mode
  - `nursery generate --watch`
  - Regenerate on nursery.toml changes

- [x] Diff mode
  - `nursery generate --diff`
  - Show what would change without writing

## Done

- [x] Basic manifest parsing with `[variables]` section
- [x] Seed scaffolding with templates
- [x] Variable resolution (CLI, config, defaults, inferred)
- [x] User-local seeds (~/.config/nursery/seeds/)
- [x] Tool schema convention (`config_path`, `format`, `schema`)
- [x] Config generation (`nursery generate`)
- [x] Design documentation
