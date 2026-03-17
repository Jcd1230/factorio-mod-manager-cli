use std::fs;
use std::path::{Path, PathBuf};

use dialoguer::{Confirm, Input, Password};

use crate::cli::{ConfigInitArgs, ConfigSubcommand};
use crate::config::{self, AppConfig};
use crate::domain::ModListFile;
use crate::error::AppError;
use crate::ui::Ui;

pub fn handle_config(command: ConfigSubcommand, ui: &Ui, explicit_config: Option<&Path>) -> Result<(), AppError> {
    match command {
        ConfigSubcommand::Init(args) => {
            let path = explicit_config
                .map(PathBuf::from)
                .unwrap_or_else(config::default_config_path);
            if path.exists() && !args.force {
                return Err(AppError::message(format!(
                    "{} already exists, use --force to overwrite it",
                    path.display()
                )));
            }
            let existing = config::load(explicit_config)?.map(|loaded| loaded.config).unwrap_or_default();
            let config = run_setup_wizard(existing, &path, ui, args.non_interactive, &args)?;
            config::write(&path, &config)?;
            ui.success(&format!("Wrote {}", path.display()));
            Ok(())
        }
        ConfigSubcommand::Show => {
            let loaded = config::load(explicit_config)?
                .ok_or_else(|| AppError::message("no config.toml found"))?;
            ui.info(&toml::to_string_pretty(&loaded.config)?);
            Ok(())
        }
    }
}

pub fn maybe_run_first_time_setup(path: &Path, config: &mut AppConfig, ui: &Ui) -> Result<(), AppError> {
    if path.exists() {
        return Ok(());
    }
    if !std::io::IsTerminal::is_terminal(&std::io::stdin()) {
        return Ok(());
    }
    ui.heading("First-time setup");
    ui.info("No config.toml found. Starting a guided setup.");
    *config = run_setup_wizard(config.clone(), path, ui, false, &ConfigInitArgs {
        non_interactive: false,
        factorio_path: None,
        factorio_data_path: None,
        username: None,
        token: None,
        force: false,
    })?;
    config::write(path, config)?;
    ui.success(&format!("Wrote {}", path.display()));
    Ok(())
}

fn run_setup_wizard(
    mut config: AppConfig,
    path: &Path,
    ui: &Ui,
    non_interactive: bool,
    args: &ConfigInitArgs,
) -> Result<AppConfig, AppError> {
    if non_interactive {
        config.factorio.path = args.factorio_path.clone().or(config.factorio.path);
        config.factorio.data_path = args.factorio_data_path.clone().or(config.factorio.data_path);
        config.auth.username = args.username.clone().or(config.auth.username);
        config.auth.token = args.token.clone().or(config.auth.token);
        bootstrap_writable_paths(&config, ui)?;
        return Ok(config);
    }

    let theme = ui.theme();
    let default_factorio_path = args
        .factorio_path
        .clone()
        .or(config.factorio.path.clone())
        .or_else(suggest_factorio_path);
    let default_data_path = args
        .factorio_data_path
        .clone()
        .or(config.factorio.data_path.clone())
        .or_else(suggest_factorio_data_path);

    config.factorio.path = Some(
        Input::with_theme(&theme)
            .with_prompt("Factorio install path")
            .default(path_to_string(default_factorio_path.as_ref()))
            .interact_text()?
            .into(),
    );
    config.factorio.data_path = Some(
        Input::with_theme(&theme)
            .with_prompt("Factorio data path")
            .default(path_to_string(default_data_path.as_ref()))
            .interact_text()?
            .into(),
    );

    let configure_auth = Confirm::with_theme(&theme)
        .with_prompt("Configure portal credentials now?")
        .default(config.auth.username.is_some() || config.auth.token.is_some())
        .interact()?;
    if configure_auth {
        config.auth.username = Some(
            Input::with_theme(&theme)
                .with_prompt("Factorio username")
                .default(config.auth.username.clone().unwrap_or_default())
                .interact_text()?,
        );
        let token = Password::with_theme(&theme)
            .with_prompt("Factorio token")
            .allow_empty_password(true)
            .interact()?;
        if !token.is_empty() {
            config.auth.token = Some(token);
        }
    }

    let enable_reload = Confirm::with_theme(&theme)
        .with_prompt("Enable automatic service reloads after changes?")
        .default(config.reload.enabled)
        .interact()?;
    config.reload.enabled = enable_reload;
    if enable_reload {
        config.reload.service_name = Some(
            Input::with_theme(&theme)
                .with_prompt("systemd service name")
                .default(config.reload.service_name.clone().unwrap_or_else(|| "factorio".to_string()))
                .interact_text()?,
        );
    }

    bootstrap_writable_paths(&config, ui)?;
    ui.debug(&format!("Prepared setup output at {}", path.display()));

    Ok(config)
}

fn suggest_factorio_path() -> Option<PathBuf> {
    let mut candidates = vec![
        PathBuf::from("/opt/factorio"),
        PathBuf::from("/usr/local/games/factorio"),
    ];

    if let Some(home) = dirs::home_dir() {
        candidates.push(home.join(".steam/steam/steamapps/common/Factorio"));
        candidates.push(home.join(".local/share/Steam/steamapps/common/Factorio"));
        candidates.push(home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam/steamapps/common/Factorio"));
    }

    candidates
        .into_iter()
        .find(|path| path.join("bin/x64/factorio").is_file())
}

fn suggest_factorio_data_path() -> Option<PathBuf> {
    let home = dirs::home_dir()?;
    let candidate = home.join(".factorio");
    if candidate.is_dir() {
        Some(candidate)
    } else {
        None
    }
}

fn path_to_string(path: Option<&PathBuf>) -> String {
    path.map(|value| value.display().to_string()).unwrap_or_default()
}

fn bootstrap_writable_paths(config: &AppConfig, ui: &Ui) -> Result<(), AppError> {
    let Some(data_path) = config.factorio.data_path.as_ref() else {
        return Ok(());
    };
    if !data_path.exists() {
        fs::create_dir_all(data_path)?;
        ui.debug(&format!("Created data directory {}", data_path.display()));
    }

    let mods_dir = data_path.join("mods");
    if !mods_dir.exists() {
        fs::create_dir_all(&mods_dir)?;
        ui.debug(&format!("Created mods directory {}", mods_dir.display()));
    }

    let mod_list_path = mods_dir.join("mod-list.json");
    if !mod_list_path.exists() {
        let empty_mod_list = ModListFile { mods: Vec::new() };
        fs::write(&mod_list_path, serde_json::to_vec_pretty(&empty_mod_list)?)?;
        ui.debug(&format!("Created {}", mod_list_path.display()));
    }

    Ok(())
}
