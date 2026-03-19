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
    name = "mods-manager",
    version,
    about = "Manage Factorio mods with a structured CLI and TOML config.",
    color = ColorChoice::Auto,
    styles = clap_styles()
)]
pub struct Cli {
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,
    #[arg(long, global = true, short = 'v')]
    pub verbose: bool,
    #[arg(long, global = true)]
    pub no_color: bool,
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Debug, Subcommand)]
pub enum Commands {
    List,
    Install(InstallArgs),
    Update(UpdateArgs),
    Remove(RemoveArgs),
    Enable(ModifyStateArgs),
    Disable(ModifyStateArgs),
    Doctor,
    Config(ConfigCommand),
}

#[derive(Debug, Args)]
pub struct InstallArgs {
    pub mod_name: String,
    #[arg(long)]
    pub min_version: Option<String>,
    #[arg(long)]
    pub prompt_optional_dependencies: bool,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct UpdateArgs {
    #[arg(long)]
    pub enabled_only: bool,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct RemoveArgs {
    pub mod_name: String,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct ModifyStateArgs {
    pub mod_names: Vec<String>,
    #[arg(long)]
    pub dry_run: bool,
}

#[derive(Debug, Args)]
pub struct ConfigCommand {
    #[command(subcommand)]
    pub command: ConfigSubcommand,
}

#[derive(Debug, Subcommand)]
pub enum ConfigSubcommand {
    Init(ConfigInitArgs),
    Show,
}

#[derive(Debug, Args)]
pub struct ConfigInitArgs {
    #[arg(long)]
    pub non_interactive: bool,
    #[arg(long)]
    pub factorio_path: Option<PathBuf>,
    #[arg(long)]
    pub factorio_data_path: Option<PathBuf>,
    #[arg(long)]
    pub username: Option<String>,
    #[arg(long)]
    pub token: Option<String>,
    #[arg(long)]
    pub force: bool,
}
