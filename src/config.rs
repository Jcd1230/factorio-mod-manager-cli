use std::env;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::error::AppError;

const APP_DIR: &str = "factorio-mods-manager";
const CONFIG_FILE_NAME: &str = "config.toml";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default)]
    pub factorio: FactorioSection,
    #[serde(default)]
    pub auth: AuthSection,
    #[serde(default)]
    pub behavior: BehaviorSection,
    #[serde(default)]
    pub dependencies: DependenciesSection,
    #[serde(default)]
    pub reload: ReloadSection,
    #[serde(default)]
    pub runtime: RuntimeSection,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct FactorioSection {
    pub path: Option<PathBuf>,
    pub data_path: Option<PathBuf>,
    pub version_override: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct AuthSection {
    pub username: Option<String>,
    pub token: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct BehaviorSection {
    #[serde(default)]
    pub verbose: bool,
    #[serde(default)]
    pub dry_run: bool,
    #[serde(default)]
    pub downgrade: bool,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct DependenciesSection {
    #[serde(default = "default_true")]
    pub install_required: bool,
    #[serde(default)]
    pub install_optional: bool,
    #[serde(default = "default_true")]
    pub remove_required: bool,
    #[serde(default)]
    pub remove_optional: bool,
    #[serde(default)]
    pub ignore_conflicts: bool,
}

impl Default for DependenciesSection {
    fn default() -> Self {
        Self {
            install_required: true,
            install_optional: false,
            remove_required: true,
            remove_optional: false,
            ignore_conflicts: false,
        }
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ReloadSection {
    #[serde(default)]
    pub enabled: bool,
    pub service_name: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct RuntimeSection {
    pub alternative_glibc_directory: Option<PathBuf>,
    pub alternative_glibc_version: Option<String>,
}

#[derive(Clone, Debug)]
pub struct LoadedConfig {
    pub path: PathBuf,
    pub config: AppConfig,
}

fn default_true() -> bool {
    true
}

pub fn default_config_path() -> PathBuf {
    if let Some(base) = dirs::config_dir() {
        return base.join(APP_DIR).join(CONFIG_FILE_NAME);
    }
    PathBuf::from(CONFIG_FILE_NAME)
}

pub fn discover_config_path(explicit: Option<&Path>) -> Option<PathBuf> {
    if let Some(explicit_path) = explicit {
        return Some(explicit_path.to_path_buf());
    }

    let xdg = default_config_path();
    if xdg.is_file() {
        return Some(xdg);
    }

    let local = PathBuf::from(CONFIG_FILE_NAME);
    if local.is_file() {
        return Some(local);
    }

    None
}

pub fn load(explicit: Option<&Path>) -> Result<Option<LoadedConfig>, AppError> {
    let Some(path) = discover_config_path(explicit) else {
        return Ok(None);
    };
    let content = fs::read_to_string(&path)?;
    let config = toml::from_str::<AppConfig>(&content)?;
    Ok(Some(LoadedConfig { path, config }))
}

pub fn write(path: &Path, config: &AppConfig) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let content = toml::to_string_pretty(config)?;
    fs::write(path, content)?;
    Ok(())
}

pub fn load_or_default(explicit: Option<&Path>) -> Result<(PathBuf, AppConfig), AppError> {
    if let Some(loaded) = load(explicit)? {
        Ok((loaded.path, apply_env_overrides(loaded.config)))
    } else {
        Ok((default_config_path(), apply_env_overrides(AppConfig::default())))
    }
}

fn apply_env_overrides(mut config: AppConfig) -> AppConfig {
    if let Ok(username) = env::var("FACTORIO_USERNAME") {
        if !username.is_empty() {
            config.auth.username = Some(username);
        }
    }
    if let Ok(token) = env::var("FACTORIO_TOKEN") {
        if !token.is_empty() {
            config.auth.token = Some(token);
        }
    }
    config
}

#[cfg(test)]
mod tests {
    use std::fs;

    use tempfile::tempdir;

    use super::{AppConfig, BehaviorSection, FactorioSection, load, write};

    #[test]
    fn writes_and_loads_toml() {
        let dir = tempdir().unwrap();
        let path = dir.path().join("config.toml");
        let config = AppConfig {
            factorio: FactorioSection {
                path: Some("/opt/factorio".into()),
                data_path: Some("/srv/factorio-data".into()),
                version_override: None,
            },
            behavior: BehaviorSection {
                verbose: true,
                dry_run: false,
                downgrade: true,
            },
            ..AppConfig::default()
        };

        write(&path, &config).unwrap();
        let loaded = load(Some(&path)).unwrap().unwrap();
        assert_eq!(
            loaded
                .config
                .factorio
                .path
                .unwrap()
                .to_string_lossy(),
            "/opt/factorio"
        );
        assert!(loaded.config.behavior.verbose);
        assert!(fs::read_to_string(path).unwrap().contains("[factorio]"));
    }
}
