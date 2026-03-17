use factorio_mods_manager::domain::FactorioVersion;
use factorio_mods_manager::portal_api::{
    ModApiResponse, Release, ReleaseInfoJson, classify_dependencies, parse_version_requirement,
};

fn make_release(version: &str, factorio_version: &str, released_at: &str) -> Release {
    Release {
        download_url: format!("/download/test-mod/{version}"),
        file_name: format!("test-mod_{version}.zip"),
        released_at: released_at.to_string(),
        sha1: "deadbeef".to_string(),
        version: version.to_string(),
        info_json: ReleaseInfoJson {
            factorio_version: factorio_version.to_string(),
            dependencies: vec![],
        },
    }
}

fn make_release_with_deps(version: &str, factorio_version: &str, released_at: &str, deps: Vec<&str>) -> Release {
    let mut release = make_release(version, factorio_version, released_at);
    release.info_json.dependencies = deps.into_iter().map(String::from).collect();
    release
}

fn mock_client() -> factorio_mods_manager::portal_api::PortalClient {
    factorio_mods_manager::portal_api::PortalClient::new(None, None).unwrap()
}

#[test]
fn select_release_picks_latest_compatible_release() {
    let client = mock_client();
    let response = ModApiResponse {
        releases: vec![
            make_release("1.0.0", "1.1", "2024-01-01T00:00:00Z"),
            make_release("1.1.0", "1.1", "2024-06-01T00:00:00Z"),
            make_release("1.2.0", "1.1", "2024-12-01T00:00:00Z"),
        ],
    };
    let factorio = FactorioVersion::parse("1.1").unwrap();
    let result = client.select_release(&response, &factorio, None, false).unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().version, "1.2.0");
}

#[test]
fn select_release_returns_none_when_no_match() {
    let client = mock_client();
    let response = ModApiResponse {
        releases: vec![
            make_release("1.0.0", "2.0", "2024-01-01T00:00:00Z"),
        ],
    };
    let factorio = FactorioVersion::parse("1.1").unwrap();
    let result = client.select_release(&response, &factorio, None, false).unwrap();
    assert!(result.is_none());
}

#[test]
fn select_release_respects_min_version_constraint() {
    let client = mock_client();
    let response = ModApiResponse {
        releases: vec![
            make_release("1.0.0", "1.1", "2024-01-01T00:00:00Z"),
            make_release("1.1.0", "1.1", "2024-06-01T00:00:00Z"),
            make_release("2.0.0", "1.1", "2024-12-01T00:00:00Z"),
        ],
    };
    let factorio = FactorioVersion::parse("1.1").unwrap();
    let min_version = parse_version_requirement(">=2.0.0").unwrap();
    let result = client.select_release(&response, &factorio, Some(&min_version), false).unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().version, "2.0.0");
}

#[test]
fn select_release_with_downgrade_picks_older_factorio_versions() {
    let client = mock_client();
    let response = ModApiResponse {
        releases: vec![
            make_release("1.0.0", "1.0", "2024-01-01T00:00:00Z"),
            make_release("1.1.0", "1.1", "2024-06-01T00:00:00Z"),
        ],
    };
    let factorio = FactorioVersion::parse("1.1").unwrap();
    // Without downgrade, only 1.1 matches
    let result = client.select_release(&response, &factorio, None, false).unwrap();
    assert_eq!(result.unwrap().version, "1.1.0");
    // With downgrade, should still pick the latest released_at that is <= factorio version
    let result = client.select_release(&response, &factorio, None, true).unwrap();
    assert_eq!(result.unwrap().version, "1.1.0");
}

#[test]
fn select_release_downgrade_allows_older_factorio_version() {
    let client = mock_client();
    let response = ModApiResponse {
        releases: vec![
            make_release("2.0.0", "1.0", "2024-12-01T00:00:00Z"),
        ],
    };
    let factorio = FactorioVersion::parse("1.1").unwrap();
    // Without downgrade: no match (mod targets 1.0, we have 1.1)
    let result = client.select_release(&response, &factorio, None, false).unwrap();
    assert!(result.is_none());
    // With downgrade: should match (1.0 <= 1.1)
    let result = client.select_release(&response, &factorio, None, true).unwrap();
    assert!(result.is_some());
    assert_eq!(result.unwrap().version, "2.0.0");
}

#[test]
fn classify_dependencies_buckets_correctly() {
    let release = make_release_with_deps("1.0.0", "1.1", "2024-01-01T00:00:00Z", vec![
        "boblibrary >= 1.1.0",
        "? FNEI",
        "(!) badmod",
        "(?) hidden-helper",
    ]);
    let client = mock_client();
    let deps = client.dependencies_for_release(&release);
    let (required, optional, conflicts) = classify_dependencies(&deps);
    assert_eq!(required.len(), 1);
    assert_eq!(required[0].name, "boblibrary");
    assert_eq!(optional.len(), 1);
    assert_eq!(optional[0].name, "FNEI");
    assert_eq!(conflicts.len(), 1);
    assert_eq!(conflicts[0].name, "badmod");
    // hidden-optional should be filtered out
    assert_eq!(deps.len(), 4); // parsed 4 deps total
}

#[test]
fn classify_dependencies_empty_input() {
    let (required, optional, conflicts) = classify_dependencies(&[]);
    assert!(required.is_empty());
    assert!(optional.is_empty());
    assert!(conflicts.is_empty());
}

#[test]
fn parse_version_requirement_with_operators() {
    let req = parse_version_requirement(">=1.2.0").unwrap();
    let v = FactorioVersion::parse("1.2.0").unwrap();
    assert!(req.matches(&v));
    let v_lower = FactorioVersion::parse("1.1.0").unwrap();
    assert!(!req.matches(&v_lower));
}

#[test]
fn parse_version_requirement_bare_version_defaults_to_gte() {
    let req = parse_version_requirement("1.0.0").unwrap();
    let v = FactorioVersion::parse("1.0.0").unwrap();
    assert!(req.matches(&v));
    let v_higher = FactorioVersion::parse("2.0.0").unwrap();
    assert!(req.matches(&v_higher));
}
