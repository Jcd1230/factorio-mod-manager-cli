use crate::config::AppConfig;
use crate::error::AppError;
use crate::factorio;
use crate::ui::Ui;

use super::{factorio_binary_path, validated_paths};

pub fn doctor(config: &AppConfig, ui: &Ui) -> Result<(), AppError> {
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
    if let Some(binary_path) = factorio_binary_path(config) {
        if binary_path.is_file() {
            match factorio::detect_version(config) {
                Ok(version) => ui.success(&format!("Detected Factorio version: {version}")),
                Err(error) => ui.warn(&format!("Version detection failed: {error}")),
            }
        } else {
            ui.warn(&format!(
                "Factorio binary not found: {}",
                binary_path.display()
            ));
        }
    } else {
        ui.warn("Version detection skipped because Factorio path is not configured.");
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
