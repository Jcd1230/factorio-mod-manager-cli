# Factorio Mod Manager

`factorio-mod-manager` is a command-line tool for managing mods on a local or headless Factorio installation.

- **Cross-platform support** for Linux, Windows, and macOS
- **Recursive dependency management** that automatically handles required and optional mod chains during installation and removal
- **Install mods** directly from the Factorio mod portal
- **Update all** installed mods with a single command
- **Mod presets** to freely save, load, and version-lock entire mod loadouts effortlessly
- **List installed mods** and their active status
- **Enable or disable** mods without launching the game
- **Validate setup** using the built-in `doctor` command
- **Interactive configuration** wizard via `config init`

## Quickstart

### 1. Install

**Via GitHub Releases:**
Simply download the pre-compiled binary for your system (Windows, Linux, or macOS) from the [Releases](https://github.com/Jcd1230/factorio-mod-manager-cli/releases) page, extract it, and run it in any directory in your terminal.

**Via `mise` (Recommended):**
Using the [`mise`](https://mise.jdx.dev/) GitHub backend lets you seamlessly install and switch between versions:
```sh
mise use -g github:Jcd1230/factorio-mod-manager-cli
```

### 2. Configure

Initialize your configuration via the interactive wizard:

```sh
factorio-mod-manager config init
```

The wizard will attempt to automatically find your local Factorio installation path and data path (where mods are stored). Next, provide your Factorio username and web token (found directly on your Factorio profile page) so the CLI can securely authenticate with the mod portal.

### 3. Install a Modpack

To install a mod, you'll need its "slug" from the [Factorio Mod Portal](https://mods.factorio.com/). This is the last part of the mod's URL. For example, the URL for [Space Exploration](https://mods.factorio.com/mod/space-exploration) is `https://mods.factorio.com/mod/space-exploration`, so its slug is `space-exploration`.

Install a project like Space Exploration, and let the manager effortlessly resolve and download the entire recursive dependency tree:

```sh
factorio-mod-manager install space-exploration --prompt-optional-dependencies
```

The tool will locate the mod, fetch all of its required dependencies natively, and download them. By appending `--prompt-optional-dependencies`, the manager will also interactively ask whether you'd like to install any of the optional, recommended dependencies it discovers as it resolves the tree.

---

## Configuration

The tool reads `config.toml`. By default, it will drop your configuration file at:

```text
~/.config/factorio-mod-manager/config.toml
```

Generate a config non-interactively:

```sh
factorio-mod-manager config init --non-interactive \
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

`FACTORIO_USERNAME` and `FACTORIO_TOKEN` environment variables override stored credentials.

## Usage

Show help and available commands:

```sh
factorio-mod-manager --help
```

### Common Commands

| Command | Description | Example |
|---|---|---|
| `doctor` | Check setup and report issues. | `factorio-mod-manager doctor` |
| `list` | List all installed mods and their versions. | `factorio-mod-manager list` |
| `install <mod>` | Install a mod and its required dependencies. | `factorio-mod-manager install bobvehicleequipment` |
| `update` | Update all enabled mods. | `factorio-mod-manager update --enabled-only` |
| `preset save/load <name>` | Save or load mod loadout presets. | `factorio-mod-manager preset save my-loadout` |
| `remove <mod>` | Uninstall a mod and its dependencies. | `factorio-mod-manager remove FNEI` |
| `enable <mods...>` | Enable one or more mods. | `factorio-mod-manager enable bobplates bobgreenhouse` |
| `disable <mods...>` | Disable one or more mods. | `factorio-mod-manager disable IndustrialRevolution` |
| `config show` | Print current configuration. | `factorio-mod-manager config show` |

**Modifiers:**
- `--dry-run`: Preview changes without applying them. (e.g. `factorio-mod-manager install bobvehicleequipment --dry-run`)
- `--prompt-optional-dependencies`: Interactively choose optional dependencies during installation.

## Building from Source

If you prefer to compile the tool yourself, you will need a Rust toolchain.

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

The binary will be available at: `./target/release/factorio-mod-manager`

### Without `mise`

Install a current Rust toolchain with `rustup`, then build normally:

```sh
cargo build --release
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
