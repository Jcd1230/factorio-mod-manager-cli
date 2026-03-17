use std::fs;
use std::io::Write;

use tempfile::tempdir;

use factorio_mods_manager::domain::{InstalledMod, ModListFile};
use factorio_mods_manager::factorio::{self, FactorioPaths};

fn make_paths(dir: &std::path::Path) -> FactorioPaths {
    let mods_dir = dir.join("mods");
    fs::create_dir_all(&mods_dir).unwrap();
    FactorioPaths {
        factorio_path: dir.to_path_buf(),
        data_path: dir.to_path_buf(),
        mods_dir: mods_dir.clone(),
        mod_list_path: mods_dir.join("mod-list.json"),
    }
}

fn make_mod_list(mods: Vec<(&str, bool)>) -> ModListFile {
    ModListFile {
        mods: mods
            .into_iter()
            .map(|(name, enabled)| InstalledMod {
                name: name.to_string(),
                enabled,
            })
            .collect(),
    }
}

#[test]
fn set_enabled_state_adds_new_mod() {
    let mut list = ModListFile { mods: Vec::new() };
    factorio::set_enabled_state(&mut list, &["newmod".to_string()], true);
    assert_eq!(list.mods.len(), 1);
    assert_eq!(list.mods[0].name, "newmod");
    assert!(list.mods[0].enabled);
}

#[test]
fn set_enabled_state_toggles_existing_mod() {
    let mut list = make_mod_list(vec![("mymod", true)]);
    factorio::set_enabled_state(&mut list, &["mymod".to_string()], false);
    assert_eq!(list.mods.len(), 1);
    assert!(!list.mods[0].enabled);
}

#[test]
fn set_enabled_state_sorts_after_mutation() {
    let mut list = make_mod_list(vec![("zebra-mod", true), ("alpha-mod", false)]);
    factorio::set_enabled_state(&mut list, &["middle-mod".to_string()], true);
    let names: Vec<&str> = list.mods.iter().map(|m| m.name.as_str()).collect();
    assert_eq!(names, vec!["alpha-mod", "middle-mod", "zebra-mod"]);
}

#[test]
fn remove_mod_entry_removes_correct_mod() {
    let mut list = make_mod_list(vec![("keep-me", true), ("remove-me", true), ("also-keep", false)]);
    factorio::remove_mod_entry(&mut list, "remove-me");
    assert_eq!(list.mods.len(), 2);
    assert!(!list.mods.iter().any(|m| m.name == "remove-me"));
}

#[test]
fn remove_mod_entry_no_op_for_missing_mod() {
    let mut list = make_mod_list(vec![("keep-me", true)]);
    factorio::remove_mod_entry(&mut list, "not-here");
    assert_eq!(list.mods.len(), 1);
}

#[test]
fn write_and_read_mod_list_roundtrips() {
    let dir = tempdir().unwrap();
    let paths = make_paths(dir.path());
    let list = make_mod_list(vec![("bobplates", true), ("angelsrefining", false)]);
    factorio::write_mod_list(&paths, &list).unwrap();
    let loaded = factorio::read_mod_list(&paths).unwrap();
    assert_eq!(loaded.mods.len(), 2);
    assert_eq!(loaded.mods[0].name, "bobplates");
    assert!(loaded.mods[0].enabled);
    assert_eq!(loaded.mods[1].name, "angelsrefining");
    assert!(!loaded.mods[1].enabled);
}

#[test]
fn compute_sha1_produces_correct_hash() {
    let dir = tempdir().unwrap();
    let file_path = dir.path().join("test.txt");
    let mut file = fs::File::create(&file_path).unwrap();
    file.write_all(b"hello world").unwrap();
    drop(file);
    let sha1 = factorio::compute_sha1(&file_path).unwrap();
    // sha1("hello world") = 2aae6c35c94fcfb415dbe95f408b9ce91ee846ed
    assert_eq!(sha1, "2aae6c35c94fcfb415dbe95f408b9ce91ee846ed");
}

#[test]
fn find_existing_release_returns_true_when_matching() {
    let dir = tempdir().unwrap();
    let paths = make_paths(dir.path());
    let file_path = paths.mods_dir.join("test-mod_1.0.0.zip");
    fs::write(&file_path, b"mod content").unwrap();
    let sha1 = factorio::compute_sha1(&file_path).unwrap();
    assert!(factorio::find_existing_release(&paths, "test-mod_1.0.0.zip", &sha1).unwrap());
}

#[test]
fn find_existing_release_returns_false_when_sha1_mismatch() {
    let dir = tempdir().unwrap();
    let paths = make_paths(dir.path());
    let file_path = paths.mods_dir.join("test-mod_1.0.0.zip");
    fs::write(&file_path, b"mod content").unwrap();
    assert!(!factorio::find_existing_release(&paths, "test-mod_1.0.0.zip", "wrong_sha1").unwrap());
}

#[test]
fn find_existing_release_returns_false_when_file_missing() {
    let dir = tempdir().unwrap();
    let paths = make_paths(dir.path());
    assert!(!factorio::find_existing_release(&paths, "nonexistent.zip", "any_sha1").unwrap());
}
