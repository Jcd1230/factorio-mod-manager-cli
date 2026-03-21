mod config_wizard;
mod doctor;
mod install;
mod list;
mod remove;
mod update;

use std::path::PathBuf;
use std::process::Command;

use crate::cli::{Cli, Commands, ModifyStateArgs};
use crate::config::{self, AppConfig};
use crate::error::AppError;
use crate::factorio::{self, FactorioPaths};
use crate::portal_api::PortalClient;
use crate::ui::Ui;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OptionalDependencyMode {
    Disabled,
    AutoInstall,
    Prompt,
}

#[derive(Clone, Copy, Debug)]
pub struct InstallPolicy {
    pub dry_run: bool,
    pub interactive: bool,
    pub optional_mode: OptionalDependencyMode,
}

pub async fn run(cli: Cli) -> Result<(), AppError> {
    let ui = Ui::new(!cli.no_color, cli.verbose);
    match cli.command {
        Some(Commands::Config(config_command)) => config_wizard::handle_config(config_command.command, &ui, cli.config.as_deref()),
        command => {
            let (config_path, mut config) = config::load_or_default(cli.config.as_deref())?;
            if cli.verbose {
                config.behavior.verbose = true;
            }
            config_wizard::maybe_run_first_time_setup(&config_path, &mut config, &ui)?;
            let portal = PortalClient::new(config.auth.username.clone(), config.auth.token.clone())?;
            match command.unwrap_or(Commands::Doctor) {
                Commands::List => list::list_mods(&config, &ui),
                Commands::Doctor => doctor::doctor(&config, &ui),
                Commands::Enable(args) => modify_enabled_state(&config, &ui, &args, true),
                Commands::Disable(args) => modify_enabled_state(&config, &ui, &args, false),
                Commands::Install(args) => install::install_mod(&config, &portal, &ui, &args).await,
                Commands::Update(args) => update::update_mods(&config, &portal, &ui, &args).await,
                Commands::Remove(args) => remove::remove_mod(&config, &portal, &ui, &args).await,
                Commands::Config(_) => unreachable!(),
            }
        }
    }
}

fn modify_enabled_state(
    config: &AppConfig,
    ui: &Ui,
    args: &ModifyStateArgs,
    enabled: bool,
) -> Result<(), AppError> {
    let paths = validated_paths(config)?;
    let mut list = factorio::read_mod_list(&paths)?;
    factorio::set_enabled_state(&mut list, &args.mod_names, enabled);
    if args.dry_run || config.behavior.dry_run {
        ui.heading("Dry run");
        for mod_name in &args.mod_names {
            ui.info(&format!(
                "Would mark {mod_name} as {}",
                if enabled { "enabled" } else { "disabled" }
            ));
        }
        return Ok(());
    }
    factorio::write_mod_list(&paths, &list)?;
    reload_if_needed(config, ui, "Updated mod state.")?;
    Ok(())
}

fn validated_paths(config: &AppConfig) -> Result<FactorioPaths, AppError> {
    let paths = FactorioPaths::from_config(config)?;
    if !paths.factorio_path.is_dir() {
        return Err(AppError::message(format!(
            "Factorio path does not exist: {}",
            paths.factorio_path.display()
        )));
    }
    if !paths.data_path.is_dir() {
        return Err(AppError::message(format!(
            "Factorio data path does not exist: {}",
            paths.data_path.display()
        )));
    }
    Ok(paths)
}

fn ensure_credentials(config: &AppConfig) -> Result<(), AppError> {
    if config.auth.username.is_some() && config.auth.token.is_some() {
        return Ok(());
    }
    Err(AppError::message(
        "portal credentials are required for install/update operations",
    ))
}

fn factorio_binary_path(config: &AppConfig) -> Option<PathBuf> {
    config
        .factorio
        .path
        .as_ref()
        .map(|path| path.join(crate::factorio::FACTORIO_BINARY_PATH))
}

fn reload_if_needed(config: &AppConfig, ui: &Ui, message: &str) -> Result<(), AppError> {
    ui.success(message);
    if !config.reload.enabled {
        ui.info("Automatic reload is disabled.");
        return Ok(());
    }
    let service_name = config
        .reload
        .service_name
        .as_ref()
        .ok_or_else(|| AppError::message("reload is enabled but service_name is not configured"))?;
    
    #[cfg(target_os = "linux")]
    {
        ui.status("reload", &format!("Restarting {service_name}"));
        let status = Command::new("systemctl")
            .arg("restart")
            .arg(service_name)
            .status()?;
        if !status.success() {
            return Err(AppError::message(format!(
                "systemctl restart {service_name} failed"
            )));
        }
    }
    
    #[cfg(not(target_os = "linux"))]
    {
        ui.info(&format!("Automatic reload for '{service_name}' is currently only supported on Linux."));
    }

    Ok(())
}
