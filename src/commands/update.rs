use std::collections::HashSet;

use crate::cli::UpdateArgs;
use crate::config::AppConfig;
use crate::error::AppError;
use crate::factorio;
use crate::portal_api::PortalClient;
use crate::ui::Ui;

use super::{InstallPolicy, OptionalDependencyMode, ensure_credentials, reload_if_needed, validated_paths};
use super::install::install_one;

pub async fn update_mods(config: &AppConfig, portal: &PortalClient, ui: &Ui, args: &UpdateArgs) -> Result<(), AppError> {
    let paths = validated_paths(config)?;
    ensure_credentials(config)?;
    let factorio_version = factorio::detect_version(config)?;
    let mut list = factorio::read_mod_list(&paths)?;
    let built_in_mods = ["base", "elevated-rails", "quality", "space-age"];
    for entry in list.mods.clone() {
        if built_in_mods.contains(&entry.name.as_str()) {
            continue;
        }
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
            InstallPolicy {
                dry_run: args.dry_run || config.behavior.dry_run,
                interactive: false,
                optional_mode: OptionalDependencyMode::Disabled,
            },
        ).await?;
    }
    if !(args.dry_run || config.behavior.dry_run) {
        factorio::write_mod_list(&paths, &list)?;
        reload_if_needed(config, ui, "Update complete.")?;
    }
    Ok(())
}
