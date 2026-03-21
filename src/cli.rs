use std::path::PathBuf;

use clap::builder::styling::{AnsiColor, Effects, Styles};
use clap::{Args, ColorChoice, Parser, Subcommand};

fn clap_styles() -> Styles {
    Styles::styled()
        .header(AnsiColor::Cyan.on_default().effects(Effects::BOLD))
        .usage(AnsiColor::Yellow.on_default().effects(Effects::BOLD))
        .literal(AnsiColor::Green.on_default())
        .placeholder(AnsiColor::BrightBlue.on_default())
        .valid(AnsiColor::Green.on_default())
        .invalid(AnsiColor::Red.on_default().effects(Effects::BOLD))
        .error(AnsiColor::Red.on_default().effects(Effects::BOLD))
}

#[derive(Debug, Parser)]
#[command(
    name = "mod-manager",
    version,
    about = "Manage Factorio mods with a structured CLI and TOML config.",
    color = ColorChoice::Auto,
    styles = clap_styles()
)]
pub struct Cli {
    /// Path to a custom config.toml file. Defaults to `~/.config/factorio-mods-manager/config.toml`.
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,
    /// Enable verbose logging for debug information.
    #[arg(long, global = true, short = 'v')]
    pub verbose: bool,
    /// Disable colored output in the terminal.
    #[arg(long, global = true)]
    pub no_color: bool,
    /// The subcommand to execute.
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    /// List all currently installed and managed mods.
    List,
    /// Install a mod and its dependencies from the portal.
    Install(InstallArgs),
    /// Update installed mods to their latest compatible versions.
    Update(UpdateArgs),
    /// Remove a mod and optionally its now-unused dependencies.
    Remove(RemoveArgs),
    /// Enable one or more mods in mod-list.json.
    Enable(ModifyStateArgs),
    /// Disable one or more mods in mod-list.json.
    Disable(ModifyStateArgs),
    /// Check the health of your Factorio installation and configuration.
    Doctor,
    /// Manage the application configuration.
    Config(ConfigCommand),
    /// Manage mod presets to save and swap mod loadouts.
    Preset(PresetCommand),
}

#[derive(Debug, Args)]
pub struct InstallArgs {
    /// The name of the mod to install (internal name, e.g., 'FNEI').
    pub mod_name: String,
    /// Minimum version of the mod to install.
    #[arg(long)]
    pub min_version: Option<String>,
    /// Prompt for each optional dependency found during resolution.
    #[arg(long)]
    pub prompt_optional_dependencies: bool,
    /// See what would happen without making any changes.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct UpdateArgs {
    /// Only check for updates for mods that are currently enabled.
    #[arg(long)]
    pub enabled_only: bool,
    /// See what would be updated without downloading anything.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    /// The name of the mod to remove.
    pub mod_name: String,
    /// See which files would be deleted without actually removing them.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct ModifyStateArgs {
    /// The internal names of the mods to modify.
    pub mod_names: Vec<String>,
    /// Show the planned state change without writing to mod-list.json.
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct ConfigCommand {
    /// Configuration subcommands.
    #[command(subcommand)]
    pub command: ConfigSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ConfigSubcommand {
    /// Initialize a new configuration file interactively.
    Init(ConfigInitArgs),
    /// Show the current configuration as TOML.
    Show,
    /// Print the absolute path to the active configuration file.
    Path,
}

#[derive(Debug, Args)]
pub struct ConfigInitArgs {
    /// Skip interactive prompts and use provided arguments or defaults.
    #[arg(long)]
    pub non_interactive: bool,
    /// Explicit path to the Factorio installation directory.
    #[arg(long)]
    pub factorio_path: Option<PathBuf>,
    /// Explicit path to the Factorio data directory (where mods/ are).
    #[arg(long)]
    pub factorio_data_path: Option<PathBuf>,
    /// Factorio portal username.
    #[arg(long)]
    pub username: Option<String>,
    /// Factorio portal token.
    #[arg(long)]
    pub token: Option<String>,
    /// Force overwrite of an existing config file.
    #[arg(long)]
    pub force: bool,
}

#[derive(Debug, Args)]
pub struct PresetCommand {
    #[command(subcommand)]
    pub command: PresetSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum PresetSubcommand {
    /// Save the currently enabled mods as a preset, locking their exact versions.
    Save(PresetActionArgs),
    /// Load a preset, downloading missing mods and completely overwriting the current mod loadout.
    Load(PresetActionArgs),
    /// List all currently saved presets.
    List,
    /// Rename an existing preset.
    Rename(PresetRenameArgs),
    /// Delete an existing preset.
    Delete(PresetActionArgs),
}

#[derive(Debug, Args)]
pub struct PresetActionArgs {
    /// The name of the preset.
    pub name: String,
}

#[derive(Debug, Args)]
pub struct PresetRenameArgs {
    /// The current name of the preset.
    pub old_name: String,
    /// The new name for the preset.
    pub new_name: String,
}
