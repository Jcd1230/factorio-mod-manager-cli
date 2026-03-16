use std::collections::{BTreeSet, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::io::IsTerminal;
use std::path::{Path, PathBuf};
use std::process::Command;

use dialoguer::{Confirm, Input, Password};
use indicatif::{ProgressBar, ProgressStyle};

use crate::cli::{Cli, Commands, ConfigInitArgs, ConfigSubcommand, InstallArgs, ModifyStateArgs, RemoveArgs, UpdateArgs};
use crate::config::{self, AppConfig};
use crate::domain::{DependencySpec, FactorioVersion, ModListFile};
use crate::error::AppError;
use crate::factorio::{self, FactorioPaths};
use crate::portal_api::{PortalClient, classify_dependencies, parse_version_requirement};
use crate::ui::Ui;

pub fn run(cli: Cli) -> Result<(), AppError> {
    let ui = Ui::new(!cli.no_color, cli.verbose);
    match cli.command {
        Some(Commands::Config(config_command)) => handle_config(config_command.command, &ui, cli.config.as_deref()),
        command => {
            let (config_path, mut config) = config::load_or_default(cli.config.as_deref())?;
            if cli.verbose {
                config.behavior.verbose = true;
            }
            maybe_run_first_time_setup(&config_path, &mut config, &ui)?;
            let portal = PortalClient::new(config.auth.username.clone(), config.auth.token.clone())?;
            match command.unwrap_or(Commands::Doctor) {
                Commands::List => list_mods(&config, &ui),
                Commands::Doctor => doctor(&config, &ui),
                Commands::Enable(args) => modify_enabled_state(&config, &ui, &args, true),
                Commands::Disable(args) => modify_enabled_state(&config, &ui, &args, false),
                Commands::Install(args) => install_mod(&config, &portal, &ui, &args),
                Commands::Update(args) => update_mods(&config, &portal, &ui, &args),
                Commands::Remove(args) => remove_mod(&config, &portal, &ui, &args),
                Commands::Config(_) => unreachable!(),
            }
        }
    }
}

fn maybe_run_first_time_setup(path: &Path, config: &mut AppConfig, ui: &Ui) -> Result<(), AppError> {
    if path.exists() {
        return Ok(());
    }
    if !std::io::stdin().is_terminal() {
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
    ui.success(&format!("Wrote {}", path.display()));
    Ok(())
}

fn handle_config(command: ConfigSubcommand, ui: &Ui, explicit_config: Option<&Path>) -> Result<(), AppError> {
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

fn run_setup_wizard(
    mut config: AppConfig,
    _path: &Path,
    ui: &Ui,
    non_interactive: bool,
    args: &ConfigInitArgs,
) -> Result<AppConfig, AppError> {
    if non_interactive {
        config.factorio.path = args.factorio_path.clone().or(config.factorio.path);
        config.factorio.data_path = args.factorio_data_path.clone().or(config.factorio.data_path);
        config.auth.username = args.username.clone().or(config.auth.username);
        config.auth.token = args.token.clone().or(config.auth.token);
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

    Ok(config)
}

fn suggest_factorio_path() -> Option<PathBuf> {
    [
        "/opt/factorio",
        "/usr/local/games/factorio",
        "/home/jason/.factorio",
    ]
    .into_iter()
    .map(PathBuf::from)
    .find(|path| path.is_dir())
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

fn list_mods(config: &AppConfig, ui: &Ui) -> Result<(), AppError> {
    let paths = validated_paths(config)?;
    let list = factorio::read_mod_list(&paths)?;
    ui.heading("Installed mods");
    if list.mods.is_empty() {
        ui.info("No mods are installed.");
        return Ok(());
    }
    for entry in list.mods {
        let state = if entry.enabled { "enabled" } else { "disabled" };
        ui.info(&format!("{} ({state})", entry.name));
    }
    Ok(())
}

fn doctor(config: &AppConfig, ui: &Ui) -> Result<(), AppError> {
    ui.heading("Doctor");
    match validated_paths(config) {
        Ok(paths) => {
            if paths.factorio_path.is_dir() {
                ui.success(&format!("Factorio path: {}", paths.factorio_path.display()));
            } else {
                ui.warn(&format!("Factorio path missing: {}", paths.factorio_path.display()));
            }
            if paths.data_path.is_dir() {
                ui.success(&format!("Factorio data path: {}", paths.data_path.display()));
            } else {
                ui.warn(&format!("Factorio data path missing: {}", paths.data_path.display()));
            }
            if paths.mod_list_path.is_file() {
                ui.success(&format!("Mod list: {}", paths.mod_list_path.display()));
            } else {
                ui.warn(&format!("mod-list.json missing: {}", paths.mod_list_path.display()));
            }
        }
        Err(error) => ui.warn(&error.to_string()),
    }
    match factorio::detect_version(config) {
        Ok(version) => ui.success(&format!("Detected Factorio version: {version}")),
        Err(error) => ui.warn(&format!("Version detection failed: {error}")),
    }
    if config.auth.username.is_some() && config.auth.token.is_some() {
        ui.success("Portal credentials are configured.");
    } else {
        ui.warn("Portal credentials are not fully configured.");
    }
    if config.reload.enabled {
        if let Some(service) = &config.reload.service_name {
            ui.success(&format!("Reload service configured: {service}"));
        } else {
            ui.warn("Reload is enabled but no service name is configured.");
        }
    }
    Ok(())
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

fn install_mod(config: &AppConfig, portal: &PortalClient, ui: &Ui, args: &InstallArgs) -> Result<(), AppError> {
    let paths = validated_paths(config)?;
    ensure_credentials(config)?;
    let factorio_version = factorio::detect_version(config)?;
    let mut list = factorio::read_mod_list(&paths)?;
    let mut seen = HashSet::new();
    install_one(
        &paths,
        config,
        portal,
        ui,
        &mut list,
        &factorio_version,
        &args.mod_name,
        args.min_version.as_deref(),
        &mut seen,
        args.dry_run || config.behavior.dry_run,
        true,
    )?;
    if !(args.dry_run || config.behavior.dry_run) {
        factorio::write_mod_list(&paths, &list)?;
        reload_if_needed(config, ui, "Install complete.")?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn install_one(
    paths: &FactorioPaths,
    config: &AppConfig,
    portal: &PortalClient,
    ui: &Ui,
    list: &mut ModListFile,
    factorio_version: &FactorioVersion,
    mod_name: &str,
    min_version: Option<&str>,
    seen: &mut HashSet<String>,
    dry_run: bool,
    include_optional_dependencies: bool,
) -> Result<(), AppError> {
    if !seen.insert(mod_name.to_string()) {
        ui.debug(&format!("Already evaluated {mod_name}, skipping recursion."));
        return Ok(());
    }
    ui.status("fetch", &format!("Resolving {mod_name}"));
    let response = portal.fetch_mod(mod_name)?;
    let requirement = min_version.map(parse_version_requirement).transpose()?;
    let release = portal
        .select_release(
            &response,
            factorio_version,
            requirement.as_ref(),
            config.behavior.downgrade,
        )?
        .ok_or_else(|| AppError::message(format!("no compatible release found for {mod_name}")))?;

    let dependencies = portal.dependencies_for_release(&release);
    let (required, optional, conflicts) = classify_dependencies(&dependencies);
    fail_on_conflicts(list, &conflicts, config)?;

    if config.dependencies.install_required {
        for dependency in required {
            install_one(
                paths,
                config,
                portal,
                ui,
                list,
                factorio_version,
                &dependency.name,
                dependency.version_requirement.as_ref().map(|req| req.version.to_string()).as_deref(),
                seen,
                dry_run,
                false,
            )?;
        }
    }
    if include_optional_dependencies && config.dependencies.install_optional {
        for dependency in optional {
            install_one(
                paths,
                config,
                portal,
                ui,
                list,
                factorio_version,
                &dependency.name,
                dependency.version_requirement.as_ref().map(|req| req.version.to_string()).as_deref(),
                seen,
                dry_run,
                false,
            )?;
        }
    }

    factorio::set_enabled_state(list, &[mod_name.to_string()], true);
    let target_path = paths.mods_dir.join(&release.file_name);
    if factorio::find_existing_release(paths, &release.file_name, &release.sha1)? {
        ui.success(&format!("{mod_name} is already current."));
        return Ok(());
    }
    if dry_run {
        ui.info(&format!(
            "Would download {} to {}",
            release.file_name,
            target_path.display()
        ));
        return Ok(());
    }
    download_release(paths, portal, ui, &release)?;
    ui.success(&format!(
        "Installed {mod_name} {} for Factorio {}",
        release.version, release.info_json.factorio_version
    ));
    Ok(())
}

fn update_mods(config: &AppConfig, portal: &PortalClient, ui: &Ui, args: &UpdateArgs) -> Result<(), AppError> {
    let paths = validated_paths(config)?;
    ensure_credentials(config)?;
    let factorio_version = factorio::detect_version(config)?;
    let mut list = factorio::read_mod_list(&paths)?;
    for entry in list.mods.clone() {
        if args.enabled_only && !entry.enabled {
            continue;
        }
        install_one(
            &paths,
            config,
            portal,
            ui,
            &mut list,
            &factorio_version,
            &entry.name,
            None,
            &mut HashSet::new(),
            args.dry_run || config.behavior.dry_run,
            false,
        )?;
    }
    if !(args.dry_run || config.behavior.dry_run) {
        factorio::write_mod_list(&paths, &list)?;
        reload_if_needed(config, ui, "Update complete.")?;
    }
    Ok(())
}

fn remove_mod(config: &AppConfig, portal: &PortalClient, ui: &Ui, args: &RemoveArgs) -> Result<(), AppError> {
    let paths = validated_paths(config)?;
    let mut list = factorio::read_mod_list(&paths)?;
    let factorio_version = factorio::detect_version(config)?;
    let mut seen = HashSet::new();
    remove_one(
        &paths,
        config,
        portal,
        ui,
        &mut list,
        &factorio_version,
        &args.mod_name,
        &mut seen,
        args.dry_run || config.behavior.dry_run,
        true,
    )?;
    if !(args.dry_run || config.behavior.dry_run) {
        factorio::write_mod_list(&paths, &list)?;
        reload_if_needed(config, ui, "Removal complete.")?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn remove_one(
    paths: &FactorioPaths,
    config: &AppConfig,
    portal: &PortalClient,
    ui: &Ui,
    list: &mut ModListFile,
    factorio_version: &FactorioVersion,
    mod_name: &str,
    seen: &mut HashSet<String>,
    dry_run: bool,
    include_optional_dependencies: bool,
) -> Result<(), AppError> {
    if !seen.insert(mod_name.to_string()) {
        return Ok(());
    }

    let response = portal.fetch_mod(mod_name)?;
    let release = portal
        .select_release(&response, factorio_version, None, config.behavior.downgrade)?
        .ok_or_else(|| AppError::message(format!("no compatible release found for {mod_name}")))?;
    let dependencies = portal.dependencies_for_release(&release);
    let (required, optional, _) = classify_dependencies(&dependencies);
    let protected = collect_required_dependencies_for_other_mods(
        config,
        portal,
        list,
        factorio_version,
        mod_name,
    )?;

    if config.dependencies.remove_required {
        for dependency in required {
            if protected.contains(&dependency.name) {
                ui.debug(&format!(
                    "Keeping {} because another installed mod still depends on it.",
                    dependency.name
                ));
                continue;
            }
            remove_one(
                paths,
                config,
                portal,
                ui,
                list,
                factorio_version,
                &dependency.name,
                seen,
                dry_run,
                false,
            )?;
        }
    }
    if include_optional_dependencies && config.dependencies.remove_optional {
        for dependency in optional {
            if protected.contains(&dependency.name) {
                continue;
            }
            remove_one(
                paths,
                config,
                portal,
                ui,
                list,
                factorio_version,
                &dependency.name,
                seen,
                dry_run,
                false,
            )?;
        }
    }

    for release in response.releases {
        let candidate = paths.mods_dir.join(release.file_name);
        if candidate.exists() {
            if dry_run {
                ui.info(&format!("Would remove {}", candidate.display()));
            } else {
                fs::remove_file(candidate)?;
            }
        }
    }
    factorio::remove_mod_entry(list, mod_name);
    ui.success(&format!("Removed {mod_name}"));
    Ok(())
}

fn collect_required_dependencies_for_other_mods(
    config: &AppConfig,
    portal: &PortalClient,
    list: &ModListFile,
    factorio_version: &FactorioVersion,
    removing: &str,
) -> Result<BTreeSet<String>, AppError> {
    let mut protected = BTreeSet::new();
    for mod_entry in &list.mods {
        if mod_entry.name == removing {
            continue;
        }
        let response = match portal.fetch_mod(&mod_entry.name) {
            Ok(response) => response,
            Err(_) => continue,
        };
        let Some(release) = portal.select_release(&response, factorio_version, None, config.behavior.downgrade)? else {
            continue;
        };
        let (required, _, _) = classify_dependencies(&portal.dependencies_for_release(&release));
        for dependency in required {
            protected.insert(dependency.name);
        }
    }
    Ok(protected)
}

fn fail_on_conflicts(list: &ModListFile, conflicts: &[DependencySpec], config: &AppConfig) -> Result<(), AppError> {
    if config.dependencies.ignore_conflicts {
        return Ok(());
    }
    for conflict in conflicts {
        if list.mods.iter().any(|entry| entry.name == conflict.name) {
            return Err(AppError::message(format!(
                "mod conflict detected with installed mod `{}`",
                conflict.name
            )));
        }
    }
    Ok(())
}

fn download_release(
    paths: &FactorioPaths,
    portal: &PortalClient,
    ui: &Ui,
    release: &crate::portal_api::Release,
) -> Result<(), AppError> {
    fs::create_dir_all(&paths.mods_dir)?;
    let target_path = paths.mods_dir.join(&release.file_name);
    let mut response = portal.download_release(release)?;
    if !response.status().is_success() {
        return Err(AppError::message(format!(
            "download failed for {}",
            release.file_name
        )));
    }
    let total = response.content_length().unwrap_or(0);
    let progress = if total > 0 {
        let bar = ProgressBar::new(total);
        let style = ProgressStyle::with_template("{msg} {bar:40.cyan/blue} {bytes}/{total_bytes}")
            .map_err(|error| AppError::message(error.to_string()))?;
        bar.set_style(style);
        bar.set_message(format!("Downloading {}", release.file_name));
        Some(bar)
    } else {
        None
    };

    let mut file = fs::File::create(&target_path)?;
    let mut buffer = [0u8; 8192];
    loop {
        let read = response.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        file.write_all(&buffer[..read])?;
        if let Some(bar) = &progress {
            bar.inc(read as u64);
        }
    }
    if let Some(bar) = progress {
        bar.finish_and_clear();
    }

    let actual_sha1 = factorio::compute_sha1(&target_path)?;
    if actual_sha1 != release.sha1 {
        return Err(AppError::message(format!(
            "SHA1 mismatch for {}",
            release.file_name
        )));
    }
    ui.debug(&format!("Saved {}", target_path.display()));
    Ok(())
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
    Ok(())
}

fn ensure_credentials(config: &AppConfig) -> Result<(), AppError> {
    if config.auth.username.is_some() && config.auth.token.is_some() {
        return Ok(());
    }
    Err(AppError::message(
        "portal credentials are required for install/update operations",
    ))
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
