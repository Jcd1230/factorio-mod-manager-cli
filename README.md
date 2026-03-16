# Factorio Mods Manager

Factorio Mods Manager is a local CLI for managing a headless Factorio install:

- list installed mods
- install mods from the Factorio portal
- update installed mods
- remove mods and dependency trees
- enable or disable mods in `mod-list.json`
- validate a setup with `doctor`
- bootstrap a fresh machine with an interactive first-time setup flow

The repository still includes the legacy Python script, but the primary implementation is now the Rust CLI in `src/`.

## Highlights

- TOML config instead of JSON comments-as-data
- guided `config init` flow for first-time setup
- narrower, clearer CLI help and structured status output
- explicit subcommands instead of one large flag matrix
- modular Rust code split into config, API, Factorio filesystem, domain logic, and command orchestration

## Build

Requirements:

- Rust toolchain with Cargo

Build the CLI:

```sh
cargo build --release
```

Run it directly during development:

```sh
cargo run -- doctor
```

## First-Time Setup

If no `config.toml` exists, the Rust CLI will offer a guided setup in interactive terminals.

You can also run setup explicitly:

```sh
cargo run -- config init
```

For automation:

```sh
cargo run -- config init --non-interactive \
  --factorio-path /opt/factorio \
  --factorio-data-path /srv/factorio-data \
  --username YOUR_USER \
  --token YOUR_TOKEN
```

The default config path is the platform config directory, typically:

```text
~/.config/factorio-mods-manager/config.toml
```

The repo also ships a sample file at [config.example.toml](/home/jason/projects/Factorio-mods-manager/config.example.toml).

If a legacy `config.json` exists in the repo root, `config init` will import its values into TOML.

## Configuration

```toml
[factorio]
path = "/opt/factorio"
data_path = "/home/you/.factorio"

[auth]
username = "your-factorio-username"
token = "your-factorio-token"

[behavior]
verbose = false
dry_run = false
downgrade = false

[dependencies]
install_required = true
install_optional = false
remove_required = true
remove_optional = false
ignore_conflicts = false

[reload]
enabled = false
service_name = "factorio"
```

Notes:

- `factorio.path` is the installation directory containing `bin/x64/factorio`.
- `factorio.data_path` is the writable data directory containing `mods/` and `mod-list.json`.
  For portable installs this is often the same as `factorio.path`.
  For Steam installs it is commonly `~/.factorio`.
- `auth.username` and `auth.token` are required for install and update operations.
- You can override credentials with `FACTORIO_USERNAME` and `FACTORIO_TOKEN`.

## Commands

Get full help:

```sh
cargo run -- --help
```

Common commands:

```sh
cargo run -- list
cargo run -- doctor
cargo run -- install bobvehicleequipment
cargo run -- install bobvehicleequipment --dry-run
cargo run -- update --enabled-only
cargo run -- remove FNEI
cargo run -- enable bobplates bobgreenhouse
cargo run -- disable IndustrialRevolution
cargo run -- config show
```

### CLI Mapping From The Python Script

The Rust CLI is a clean break from the old flag-heavy interface:

- `python mods_manager.py -l` -> `mods-manager list`
- `python mods_manager.py -i bobvehicleequipment` -> `mods-manager install bobvehicleequipment`
- `python mods_manager.py -U -e` -> `mods-manager update --enabled-only`
- `python mods_manager.py -r FNEI` -> `mods-manager remove FNEI`
- `python mods_manager.py -E bobplates -E bobgreenhouse` -> `mods-manager enable bobplates bobgreenhouse`
- `python mods_manager.py -D IndustrialRevolution` -> `mods-manager disable IndustrialRevolution`

## Behavior Notes

- Install and update operations detect the local Factorio version by invoking `bin/x64/factorio --version`.
- If `behavior.downgrade = true`, the resolver will fall back to the latest older compatible mod release when an exact Factorio version match is unavailable.
- Dry-run mode uses the same planning path as live execution and prints intended downloads or removals without mutating files.
- Removal attempts to avoid deleting dependencies that are still required by other installed mods.
- If `[reload].enabled = true`, the CLI runs `systemctl restart <service_name>` after a mutating command succeeds.

## Doctor

`doctor` validates the local environment and highlights the most common setup issues:

- missing Factorio install path
- missing data path
- missing `mod-list.json`
- failed version detection
- missing portal credentials
- incomplete reload configuration

Run it with:

```sh
cargo run -- doctor
```

## Legacy Python Script

The original Python entrypoint remains in [mods_manager.py](/home/jason/projects/Factorio-mods-manager/mods_manager.py) during migration. It now has improved help output and supports a separate Factorio data path via `--path-to-factorio-data`.

## Status

The Rust port covers the main CLI flows and is structured for continued iteration. Areas likely to improve next:

- release packaging and install instructions
- richer integration tests with filesystem and mocked HTTP fixtures
- machine-readable output mode
- shell completions and man pages
# Factorio Mods Manager

`factorio-mods-manager` is a command-line tool for managing mods on a local or headless Factorio installation.

It can:

- list installed mods
- install mods from the Factorio mod portal
- update installed mods
- remove mods and dependency trees
- enable or disable mods in `mod-list.json`
- validate a setup with `doctor`
- bootstrap configuration with `config init`

## Requirements

- Linux
- a Factorio installation
- Rust toolchain

## Install

### With `mise` (recommended)

This repository includes a local `mise.toml` so contributors and operators can install the required Rust toolchain in the project directory.

1. Install `mise`: <https://mise.jdx.dev/>
2. Trust and install the project tools:

```sh
mise install
```

3. Build the binary:

```sh
cargo build --release
```

The compiled binary will be available at:

```text
./target/release/factorio-mods-manager
```

### Without `mise`

Install a current Rust toolchain with `rustup`, then build normally:

```sh
cargo build --release
```

## Configuration

The tool reads `config.toml`. By default it uses the platform config directory:

```text
~/.config/factorio-mods-manager/config.toml
```

Generate a config interactively:

```sh
cargo run -- config init
```

Generate a config non-interactively:

```sh
cargo run -- config init --non-interactive \
  --factorio-path /opt/factorio \
  --factorio-data-path /srv/factorio-data \
  --username YOUR_USER \
  --token YOUR_TOKEN
```

A sample config is provided in [`./config.example.toml`](./config.example.toml).

Key settings:

- `factorio.path`: Factorio install directory containing `bin/x64/factorio`
- `factorio.data_path`: writable data directory containing `mods/` and `mod-list.json`
- `auth.username`: Factorio portal username
- `auth.token`: Factorio portal token
- `reload.enabled`: restart the configured systemd service after successful changes

The environment variables `FACTORIO_USERNAME` and `FACTORIO_TOKEN` override stored credentials.

## Usage

Show help:

```sh
cargo run -- --help
```

Common commands:

```sh
cargo run -- doctor
cargo run -- list
cargo run -- install bobvehicleequipment
cargo run -- install bobvehicleequipment --dry-run
cargo run -- update --enabled-only
cargo run -- remove FNEI
cargo run -- enable bobplates bobgreenhouse
cargo run -- disable IndustrialRevolution
cargo run -- config show
```

## Build And Test

Run tests:

```sh
cargo test
```

Run the binary in development:

```sh
cargo run -- doctor
```

## License

MIT. See [`./LICENSE.md`](./LICENSE.md).
