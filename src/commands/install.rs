use std::collections::HashSet;
use std::fs;
use std::io::IsTerminal;
use std::io::Write;

use dialoguer::Confirm;
use futures_util::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use sha1::{Digest, Sha1};

use crate::cli::InstallArgs;
use crate::config::AppConfig;
use crate::domain::{DependencySpec, FactorioVersion, ModListFile};
use crate::error::AppError;
use crate::factorio::{self, FactorioPaths};
use crate::portal_api::{PortalClient, Release, classify_dependencies, parse_version_requirement};
use crate::ui::Ui;

use super::{InstallPolicy, OptionalDependencyMode, ensure_credentials, reload_if_needed, validated_paths};

pub async fn install_mod(config: &AppConfig, portal: &PortalClient, ui: &Ui, args: &InstallArgs) -> Result<(), AppError> {
    let paths = validated_paths(config)?;
    ensure_credentials(config)?;
    let factorio_version = factorio::detect_version(config)?;
    let mut list = factorio::read_mod_list(&paths)?;
    let mut seen = HashSet::new();
    let policy = install_policy(config, args);
    Box::pin(install_one(
        &paths,
        config,
        portal,
        ui,
        &mut list,
        &factorio_version,
        &args.mod_name,
        args.min_version.as_deref(),
        &mut seen,
        policy,
    )).await?;
    if !policy.dry_run {
        factorio::write_mod_list(&paths, &list)?;
        reload_if_needed(config, ui, "Install complete.")?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub async fn install_one(
    paths: &FactorioPaths,
    config: &AppConfig,
    portal: &PortalClient,
    ui: &Ui,
    list: &mut ModListFile,
    factorio_version: &FactorioVersion,
    mod_name: &str,
    min_version: Option<&str>,
    seen: &mut HashSet<String>,
    policy: InstallPolicy,
) -> Result<(), AppError> {
    if !seen.insert(mod_name.to_string()) {
        ui.debug(&format!("Already evaluated {mod_name}, skipping recursion."));
        return Ok(());
    }
    
    let built_in_mods = ["base", "elevated-rails", "quality", "space-age"];
    if built_in_mods.contains(&mod_name) {
        ui.debug(&format!("Skipping built-in engine mod {mod_name}."));
        return Ok(());
    }

    ui.status("fetch", &format!("Resolving {mod_name}"));
    let response = match portal.fetch_mod(mod_name).await {
        Ok(res) => res,
        Err(_) => {
            ui.warn(&format!("Could not fetch metadata for {mod_name} from the portal. Skipping."));
            return Ok(());
        }
    };
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
            Box::pin(install_one(
                paths,
                config,
                portal,
                ui,
                list,
                factorio_version,
                &dependency.name,
                dependency.version_requirement.as_ref().map(|req| req.version.to_string()).as_deref(),
                seen,
                policy,
            )).await?;
        }
    }
    for dependency in optional {
        if !should_install_optional_dependency(ui, &dependency.name, policy)? {
            ui.debug(&format!("Skipping optional dependency {}", dependency.name));
            continue;
        }
        Box::pin(install_one(
            paths,
            config,
            portal,
            ui,
            list,
            factorio_version,
            &dependency.name,
            dependency.version_requirement.as_ref().map(|req| req.version.to_string()).as_deref(),
            seen,
            policy,
        )).await?;
    }

    factorio::set_enabled_state(list, &[mod_name.to_string()], true);
    let target_path = paths.mods_dir.join(&release.file_name);
    if factorio::find_existing_release(paths, &release.file_name, &release.sha1)? {
        ui.success(&format!("{mod_name} is already current."));
        return Ok(());
    }
    if policy.dry_run {
        ui.info(&format!(
            "Would download {} to {}",
            release.file_name,
            target_path.display()
        ));
        return Ok(());
    }
    download_release(paths, portal, ui, &release).await?;
    ui.success(&format!(
        "Installed {mod_name} {} for Factorio {}",
        release.version, release.info_json.factorio_version
    ));
    Ok(())
}

pub fn install_policy(config: &AppConfig, args: &InstallArgs) -> InstallPolicy {
    InstallPolicy {
        dry_run: args.dry_run || config.behavior.dry_run,
        interactive: std::io::stdin().is_terminal(),
        optional_mode: if args.prompt_optional_dependencies {
            OptionalDependencyMode::Prompt
        } else if config.dependencies.install_optional {
            OptionalDependencyMode::AutoInstall
        } else {
            OptionalDependencyMode::Disabled
        },
    }
}

fn should_install_optional_dependency(
    ui: &Ui,
    dependency_name: &str,
    policy: InstallPolicy,
) -> Result<bool, AppError> {
    match policy.optional_mode {
        OptionalDependencyMode::Disabled => Ok(false),
        OptionalDependencyMode::AutoInstall => Ok(true),
        OptionalDependencyMode::Prompt => {
            if !policy.interactive {
                ui.warn(&format!(
                    "Skipping optional dependency {dependency_name} because prompting requires an interactive terminal."
                ));
                return Ok(false);
            }
            Confirm::with_theme(&ui.theme())
                .with_prompt(format!("Install optional dependency {dependency_name}?"))
                .default(false)
                .interact()
                .map_err(AppError::from)
        }
    }
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

pub async fn download_release(
    paths: &FactorioPaths,
    portal: &PortalClient,
    ui: &Ui,
    release: &Release,
) -> Result<(), AppError> {
    fs::create_dir_all(&paths.mods_dir)?;
    let target_path = paths.mods_dir.join(&release.file_name);
    let response = portal.download_release(release).await?;
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
    let mut hasher = Sha1::new();
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        file.write_all(&chunk)?;
        hasher.update(&chunk);
        if let Some(bar) = &progress {
            bar.inc(chunk.len() as u64);
        }
    }
    if let Some(bar) = progress {
        bar.finish_and_clear();
    }

    let actual_sha1 = format!("{:x}", hasher.finalize());
    if actual_sha1 != release.sha1 {
        return Err(AppError::message(format!(
            "SHA1 mismatch for {}",
            release.file_name
        )));
    }
    ui.debug(&format!("Saved {}", target_path.display()));
    Ok(())
}

#[cfg(test)]
fn should_install_optional_dependency_with_decider<F>(
    dependency_name: &str,
    policy: InstallPolicy,
    decider: F,
) -> bool
where
    F: FnOnce(&str) -> bool,
{
    match policy.optional_mode {
        OptionalDependencyMode::Disabled => false,
        OptionalDependencyMode::AutoInstall => true,
        OptionalDependencyMode::Prompt => {
            if !policy.interactive {
                return false;
            }
            decider(dependency_name)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::should_install_optional_dependency_with_decider;
    use super::super::{InstallPolicy, OptionalDependencyMode};

    #[test]
    fn optional_dependency_policy_defaults_to_disabled() {
        let policy = InstallPolicy {
            dry_run: false,
            interactive: true,
            optional_mode: OptionalDependencyMode::Disabled,
        };
        assert!(!should_install_optional_dependency_with_decider("FNEI", policy, |_| true));
    }

    #[test]
    fn optional_dependency_policy_auto_installs_recursively() {
        let policy = InstallPolicy {
            dry_run: false,
            interactive: false,
            optional_mode: OptionalDependencyMode::AutoInstall,
        };
        assert!(should_install_optional_dependency_with_decider("FNEI", policy, |_| false));
    }

    #[test]
    fn optional_dependency_prompt_requires_interactive_terminal() {
        let policy = InstallPolicy {
            dry_run: false,
            interactive: false,
            optional_mode: OptionalDependencyMode::Prompt,
        };
        assert!(!should_install_optional_dependency_with_decider("FNEI", policy, |_| true));
    }

    #[test]
    fn optional_dependency_prompt_uses_user_decision() {
        let policy = InstallPolicy {
            dry_run: false,
            interactive: true,
            optional_mode: OptionalDependencyMode::Prompt,
        };
        assert!(should_install_optional_dependency_with_decider("FNEI", policy, |_| true));
        assert!(!should_install_optional_dependency_with_decider("FNEI", policy, |_| false));
    }
}
