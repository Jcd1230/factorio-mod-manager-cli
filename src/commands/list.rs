use crate::config::AppConfig;
use crate::error::AppError;
use crate::factorio;
use crate::ui::Ui;

use super::validated_paths;

pub fn list_mods(config: &AppConfig, ui: &Ui) -> Result<(), AppError> {
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
