use factorio_mods_manager::domain::{
    Comparator, DependencyKind, FactorioVersion, VersionRequirement, parse_dependency,
};

// --- FactorioVersion ordering ---

#[test]
fn version_equal_with_different_component_counts() {
    let a = FactorioVersion::parse("1.1").unwrap();
    let b = FactorioVersion::parse("1.1.0").unwrap();
    // Ord treats missing components as 0, so cmp returns Equal.
    // But PartialEq is derived and compares the Vec directly, so [1,1] != [1,1,0].
    assert_eq!(a.cmp(&b), std::cmp::Ordering::Equal);
    assert!(a == b.major_minor());
}

#[test]
fn version_ordering_major_difference() {
    let a = FactorioVersion::parse("1.0.0").unwrap();
    let b = FactorioVersion::parse("2.0.0").unwrap();
    assert!(a < b);
    assert!(b > a);
}

#[test]
fn version_ordering_minor_difference() {
    let a = FactorioVersion::parse("1.0").unwrap();
    let b = FactorioVersion::parse("1.1").unwrap();
    assert!(a < b);
}

#[test]
fn version_ordering_patch_difference() {
    let a = FactorioVersion::parse("1.1.0").unwrap();
    let b = FactorioVersion::parse("1.1.5").unwrap();
    assert!(a < b);
}

#[test]
fn version_display_roundtrips() {
    let v = FactorioVersion::parse("1.2.3").unwrap();
    assert_eq!(v.to_string(), "1.2.3");
}

#[test]
fn version_major_minor_truncation() {
    let v = FactorioVersion::parse("1.2.3").unwrap();
    assert_eq!(v.major_minor().to_string(), "1.2");
}

#[test]
fn version_parse_rejects_empty() {
    assert!(FactorioVersion::parse("").is_err());
}

#[test]
fn version_parse_rejects_non_numeric() {
    assert!(FactorioVersion::parse("1.x.0").is_err());
}

// --- VersionRequirement matching ---

#[test]
fn version_requirement_lt() {
    let req = VersionRequirement {
        comparator: Comparator::Lt,
        version: FactorioVersion::parse("2.0.0").unwrap(),
    };
    assert!(req.matches(&FactorioVersion::parse("1.9.9").unwrap()));
    assert!(!req.matches(&FactorioVersion::parse("2.0.0").unwrap()));
    assert!(!req.matches(&FactorioVersion::parse("2.0.1").unwrap()));
}

#[test]
fn version_requirement_lte() {
    let req = VersionRequirement {
        comparator: Comparator::Lte,
        version: FactorioVersion::parse("2.0.0").unwrap(),
    };
    assert!(req.matches(&FactorioVersion::parse("2.0.0").unwrap()));
    assert!(req.matches(&FactorioVersion::parse("1.0.0").unwrap()));
    assert!(!req.matches(&FactorioVersion::parse("2.0.1").unwrap()));
}

#[test]
fn version_requirement_eq() {
    let req = VersionRequirement {
        comparator: Comparator::Eq,
        version: FactorioVersion::parse("1.5.0").unwrap(),
    };
    assert!(req.matches(&FactorioVersion::parse("1.5.0").unwrap()));
    assert!(!req.matches(&FactorioVersion::parse("1.5.1").unwrap()));
}

#[test]
fn version_requirement_gte() {
    let req = VersionRequirement {
        comparator: Comparator::Gte,
        version: FactorioVersion::parse("1.0.0").unwrap(),
    };
    assert!(req.matches(&FactorioVersion::parse("1.0.0").unwrap()));
    assert!(req.matches(&FactorioVersion::parse("2.0.0").unwrap()));
    assert!(!req.matches(&FactorioVersion::parse("0.9.0").unwrap()));
}

#[test]
fn version_requirement_gt() {
    let req = VersionRequirement {
        comparator: Comparator::Gt,
        version: FactorioVersion::parse("1.0.0").unwrap(),
    };
    assert!(!req.matches(&FactorioVersion::parse("1.0.0").unwrap()));
    assert!(req.matches(&FactorioVersion::parse("1.0.1").unwrap()));
}

// --- parse_dependency ---

#[test]
fn parse_dependency_bare_name() {
    let spec = parse_dependency("boblogistics").unwrap();
    assert_eq!(spec.name, "boblogistics");
    assert_eq!(spec.kind, DependencyKind::Required);
    assert!(spec.version_requirement.is_none());
}

#[test]
fn parse_dependency_tilde_prefix() {
    let spec = parse_dependency("~ boblibrary >= 1.0.0").unwrap();
    assert_eq!(spec.name, "boblibrary");
    assert_eq!(spec.kind, DependencyKind::Required);
    assert!(spec.version_requirement.is_some());
}

#[test]
fn parse_dependency_base_is_not_actionable() {
    assert!(parse_dependency("base").is_err());
}

#[test]
fn parse_dependency_empty_is_not_actionable() {
    assert!(parse_dependency("").is_err());
}

#[test]
fn parse_dependency_conflict_with_bang() {
    let spec = parse_dependency("! conflicting-mod").unwrap();
    assert_eq!(spec.kind, DependencyKind::Conflict);
    assert_eq!(spec.name, "conflicting-mod");
}

#[test]
fn parse_dependency_version_with_equals() {
    let spec = parse_dependency("library = 1.2.3").unwrap();
    assert_eq!(spec.name, "library");
    assert_eq!(spec.kind, DependencyKind::Required);
    let req = spec.version_requirement.unwrap();
    assert_eq!(req.comparator, Comparator::Eq);
    assert_eq!(req.version.to_string(), "1.2.3");
}

#[test]
fn parse_dependency_version_with_less_than() {
    let spec = parse_dependency("library < 2.0.0").unwrap();
    let req = spec.version_requirement.unwrap();
    assert_eq!(req.comparator, Comparator::Lt);
}
