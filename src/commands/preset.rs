use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::cli::{PresetActionArgs, PresetRenameArgs, PresetSubcommand};
use crate::config::{AppConfig, LoadedConfig};
use crate::error::AppError;
use crate::factorio::{self, FactorioPaths};
use crate::portal_api::PortalClient;
use crate::ui::Ui;
use crate::commands::install::{install_one, install_policy};
use crate::cli::InstallArgs;
use std::collections::HashSet;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PresetFile {
    pub mods: Vec<PresetMod>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct PresetMod {
    pub name: String,
    pub version: Option<String>,
}

fn presets_dir(loaded: &LoadedConfig) -> Result<PathBuf, AppError> {
    let mut path = loaded.path.clone();
    path.pop(); // remove config.toml
    path.push("presets");
    fs::create_dir_all(&path)?;
    Ok(path)
}

fn get_installed_version(mods_dir: &Path, mod_name: &str) -> Option<String> {
    let prefix = format!("{mod_name}_");
    let mapped = fs::read_dir(mods_dir).ok()?;
    for entry in mapped {
        let entry = entry.ok()?;
        let file_name = entry.file_name().to_string_lossy().to_string();
        if file_name.starts_with(&prefix) && file_name.ends_with(".zip") {
            let version = file_name.strip_prefix(&prefix)?.strip_suffix(".zip")?;
            return Some(version.to_string());
        }
    }
    None
}

pub async fn handle_preset(
    command: PresetSubcommand,
    loaded: &LoadedConfig,
    portal: &PortalClient,
    ui: &Ui,
) -> Result<(), AppError> {
    let dir = presets_dir(loaded)?;
    let paths = factorio::FactorioPaths::from_config(&loaded.config)?;

    match command {
        PresetSubcommand::Save(args) => {
            let list = factorio::read_mod_list(&paths)?;
            let mut preset_mods = Vec::new();
            
            for entry in list.mods {
                if entry.enabled && entry.name != "base" {
                    let version = get_installed_version(&paths.mods_dir, &entry.name);
                    if version.is_none() {
                        ui.debug(&format!("Could not lock version for unversioned local mod {}", entry.name));
                    }
                    preset_mods.push(PresetMod {
                        name: entry.name,
                        version,
                    });
                }
            }
            
            let preset = PresetFile { mods: preset_mods };
            let target = dir.join(format!("{}.json", args.name));
            let content = serde_json::to_string_pretty(&preset)?;
            fs::write(&target, content)?;
            ui.success(&format!("Saved preset '{}' with {} mods.", args.name, preset.mods.len()));
        }
        PresetSubcommand::Load(args) => {
            let target = dir.join(format!("{}.json", args.name));
            if !target.is_file() {
                return Err(AppError::message(format!("preset '{}' not found", args.name)));
            }
            let content = fs::read_to_string(&target)?;
            let preset: PresetFile = serde_json::from_str(&content)?;
            
            let mut list = factorio::read_mod_list(&paths)?;
            
            // disable everything first (except base)
            for m in &mut list.mods {
                if m.name != "base" {
                    m.enabled = false;
                }
            }
            
            let factorio_version = factorio::detect_version(&loaded.config)?;
            
            let mut policy = install_policy(&loaded.config, &InstallArgs {
                mod_name: "".into(),
                min_version: None,
                prompt_optional_dependencies: false,
                dry_run: false,
            });
            // Never prompt optional dependencies when loading a preset blindly
            policy.optional_mode = crate::commands::OptionalDependencyMode::Disabled;
            
            let mut seen = HashSet::new();

            for m in &preset.mods {
                 // Try to formulate strict exact version requirement if it was locked
                 let req_string = m.version.as_ref().map(|v| format!("={v}"));
                 
                 // Install one will fetch and download, and automatically set_enabled_state to true
                 install_one(
                     &paths,
                     &loaded.config,
                     portal,
                     ui,
                     &mut list,
                     &factorio_version,
                     &m.name,
                     req_string.as_deref(),
                     &mut seen,
                     policy,
                 ).await?;
            }
            
            factorio::write_mod_list(&paths, &list)?;
            ui.success(&format!("Loaded preset '{}'.", args.name));
        }
        PresetSubcommand::List => {
            ui.heading("Saved Presets");
            let mut found = false;
            if let Ok(entries) = fs::read_dir(&dir) {
                for entry in entries.flatten() {
                    let name = entry.file_name().to_string_lossy().to_string();
                    if name.ends_with(".json") {
                        ui.info(name.strip_suffix(".json").unwrap());
                        found = true;
                    }
                }
            }
            if !found {
                ui.info("No presets found.");
            }
        }
        PresetSubcommand::Rename(args) => {
            let old_target = dir.join(format!("{}.json", args.old_name));
            if !old_target.is_file() {
                return Err(AppError::message(format!("preset '{}' not found", args.old_name)));
            }
            let new_target = dir.join(format!("{}.json", args.new_name));
            if new_target.is_file() {
                return Err(AppError::message(format!("preset '{}' already exists", args.new_name)));
            }
            fs::rename(old_target, new_target)?;
            ui.success(&format!("Renamed preset '{}' to '{}'.", args.old_name, args.new_name));
        }
        PresetSubcommand::Delete(args) => {
            let target = dir.join(format!("{}.json", args.name));
            if !target.is_file() {
                return Err(AppError::message(format!("preset '{}' not found", args.name)));
            }
            fs::remove_file(target)?;
            ui.success(&format!("Deleted preset '{}'.", args.name));
        }
    }
    
    Ok(())
}
