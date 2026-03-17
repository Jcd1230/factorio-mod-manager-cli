use std::collections::{BTreeSet, HashSet};
use std::fs;

use crate::cli::RemoveArgs;
use crate::config::AppConfig;
use crate::domain::{FactorioVersion, ModListFile};
use crate::error::AppError;
use crate::factorio::{self, FactorioPaths};
use crate::portal_api::{PortalClient, classify_dependencies};
use crate::ui::Ui;

use super::{reload_if_needed, validated_paths};

pub async fn remove_mod(config: &AppConfig, portal: &PortalClient, ui: &Ui, args: &RemoveArgs) -> Result<(), AppError> {
    let paths = validated_paths(config)?;
    let mut list = factorio::read_mod_list(&paths)?;
    let factorio_version = factorio::detect_version(config)?;
    let mut seen = HashSet::new();
    Box::pin(remove_one(
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
    )).await?;
    if !(args.dry_run || config.behavior.dry_run) {
        factorio::write_mod_list(&paths, &list)?;
        reload_if_needed(config, ui, "Removal complete.")?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn remove_one(
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

    let response = portal.fetch_mod(mod_name).await?;
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
    ).await?;

    if config.dependencies.remove_required {
        for dependency in required {
            if protected.contains(&dependency.name) {
                ui.debug(&format!(
                    "Keeping {} because another installed mod still depends on it.",
                    dependency.name
                ));
                continue;
            }
            Box::pin(remove_one(
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
            )).await?;
        }
    }
    if include_optional_dependencies && config.dependencies.remove_optional {
        for dependency in optional {
            if protected.contains(&dependency.name) {
                continue;
            }
            Box::pin(remove_one(
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
            )).await?;
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

async fn collect_required_dependencies_for_other_mods(
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
        let response = match portal.fetch_mod(&mod_entry.name).await {
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
