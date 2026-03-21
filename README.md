# Factorio Mods Manager

`factorio-mods-manager` is a command-line tool for managing mods on a local or headless Factorio installation.

It supports:

- listing installed mods
- installing mods from the Factorio mod portal
- updating installed mods
- removing mods and dependency trees
- enabling or disabling mods in `mod-list.json`
- validating a setup with `doctor`
- bootstrapping configuration with `config init`

## Requirements

- Linux, Windows, or macOS
- a Factorio installation
- Rust toolchain

## Installation

### With `mise` (recommended)

This repository includes a local [`./mise.toml`](./mise.toml) so the Rust toolchain can be installed per project.

1. Install `mise`: <https://mise.jdx.dev/>
2. Install the project toolchain:

```sh
mise install
```

3. Build the binary:

```sh
cargo build --release
```

The binary will be available at:

```text
./target/release/factorio-mods-manager
```

### Without `mise`

Install a current Rust toolchain with `rustup`, then build normally:

```sh
cargo build --release
```

## Configuration

The tool reads `config.toml`. By default it uses:

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

Important settings:

- `factorio.path`: installation directory containing the Factorio binary (e.g. `bin/x64/factorio` on Linux, `bin/x64/factorio.exe` on Windows, or `factorio.app/Contents/MacOS/factorio` on macOS)
- `factorio.data_path`: writable data directory containing `mods/` and `mod-list.json`
- `auth.username`: Factorio portal username
- `auth.token`: Factorio portal token
- `reload.enabled`: restart the configured systemd service after successful changes (Linux only)

`FACTORIO_USERNAME` and `FACTORIO_TOKEN` override stored credentials.

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
cargo run -- install bobvehicleequipment --prompt-optional-dependencies
cargo run -- update --enabled-only
cargo run -- remove FNEI
cargo run -- enable bobplates bobgreenhouse
cargo run -- disable IndustrialRevolution
cargo run -- config show
```

`install` always enables the full required dependency chain for the requested mod. Use `--prompt-optional-dependencies` to interactively choose optional dependencies encountered during installation.

## Doctor

`doctor` checks the local setup and reports issues such as:

- missing Factorio install path
- missing data path
- missing `mod-list.json`
- missing Factorio binary
- missing portal credentials
- incomplete reload configuration

Run it with:

```sh
cargo run -- doctor
```

## Development

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
