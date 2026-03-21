use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

use regex::Regex;
use sha1::{Digest, Sha1};
use tempfile::NamedTempFile;

use crate::config::AppConfig;
use crate::domain::{FactorioVersion, InstalledMod, ModListFile};
use crate::error::AppError;

#[cfg(target_os = "windows")]
pub const FACTORIO_BINARY_PATH: &str = "bin/x64/factorio.exe";
#[cfg(target_os = "macos")]
pub const FACTORIO_BINARY_PATH: &str = "factorio.app/Contents/MacOS/factorio";
#[cfg(not(any(target_os = "windows", target_os = "macos")))]
pub const FACTORIO_BINARY_PATH: &str = "bin/x64/factorio";

#[derive(Clone, Debug)]
pub struct FactorioPaths {
    pub factorio_path: PathBuf,
    pub data_path: PathBuf,
    pub mods_dir: PathBuf,
    pub mod_list_path: PathBuf,
}

impl FactorioPaths {
    pub fn from_config(config: &AppConfig) -> Result<Self, AppError> {
        let factorio_path = config
            .factorio
            .path
            .clone()
            .ok_or_else(|| AppError::message("Factorio path is not configured"))?;
        let data_path = config
            .factorio
            .data_path
            .clone()
            .unwrap_or_else(|| factorio_path.clone());
        let mods_dir = data_path.join("mods");
        let mod_list_path = mods_dir.join("mod-list.json");

        Ok(Self {
            factorio_path,
            data_path,
            mods_dir,
            mod_list_path,
        })
    }
}

pub fn detect_version(config: &AppConfig) -> Result<FactorioVersion, AppError> {
    if let Some(override_version) = &config.factorio.version_override {
        return FactorioVersion::parse(override_version);
    }

    let paths = FactorioPaths::from_config(config)?;
    let binary_path = paths.factorio_path.join(FACTORIO_BINARY_PATH);
    if !binary_path.is_file() {
        return Err(AppError::message(format!(
            "Factorio binary not found at {}",
            binary_path.display()
        )));
    }
    let mut command = if let (Some(glibc_dir), Some(glibc_version)) = (
        config.runtime.alternative_glibc_directory.as_ref(),
        config.runtime.alternative_glibc_version.as_ref(),
    ) {
        let loader = glibc_dir.join(format!("lib/ld-{glibc_version}.so"));
        let mut command = Command::new(loader);
        command.arg("--library-path");
        command.arg(glibc_dir.join("lib"));
        command.arg(binary_path);
        command
    } else {
        Command::new(binary_path)
    };
    command.arg("--version");
    let output = command.output()?;
    if !output.status.success() {
        return Err(AppError::message("failed to detect Factorio version"));
    }
    let stdout = String::from_utf8_lossy(&output.stdout);
    let regex = Regex::new(r"Version:\s+(\d+\.\d+(?:\.\d+)?)")?;
    let captures = regex
        .captures(&stdout)
        .ok_or_else(|| AppError::message("could not parse Factorio version output"))?;
    FactorioVersion::parse(&captures[1]).map(|version| version.major_minor())
}

pub fn read_mod_list(paths: &FactorioPaths) -> Result<ModListFile, AppError> {
    let content = fs::read_to_string(&paths.mod_list_path)?;
    Ok(serde_json::from_str(&content)?)
}

pub fn write_mod_list(paths: &FactorioPaths, list: &ModListFile) -> Result<(), AppError> {
    fs::create_dir_all(&paths.mods_dir)?;
    let parent = paths
        .mod_list_path
        .parent()
        .ok_or_else(|| AppError::message("mod-list.json parent directory is invalid"))?;
    let mut temp = NamedTempFile::new_in(parent)?;
    let content = serde_json::to_vec_pretty(list)?;
    temp.write_all(&content)?;
    temp.flush()?;
    temp.persist(&paths.mod_list_path)
        .map_err(|error| AppError::Io(error.error))?;
    Ok(())
}

pub fn set_enabled_state(list: &mut ModListFile, mod_names: &[String], enabled: bool) {
    for name in mod_names {
        if let Some(existing) = list.mods.iter_mut().find(|entry| entry.name == *name) {
            existing.enabled = enabled;
        } else {
            list.mods.push(InstalledMod {
                name: name.clone(),
                enabled,
            });
        }
    }
    list.mods.sort_by(|a, b| a.name.cmp(&b.name));
}

pub fn remove_mod_entry(list: &mut ModListFile, mod_name: &str) {
    list.mods.retain(|entry| entry.name != mod_name);
}

pub fn compute_sha1(path: &Path) -> Result<String, AppError> {
    let mut file = fs::File::open(path)?;
    let mut hasher = Sha1::new();
    let mut buffer = [0u8; 8192];
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

pub fn find_existing_release(paths: &FactorioPaths, file_name: &str, sha1: &str) -> Result<bool, AppError> {
    let candidate = paths.mods_dir.join(file_name);
    if !candidate.is_file() {
        return Ok(false);
    }
    Ok(compute_sha1(&candidate)? == sha1)
}
