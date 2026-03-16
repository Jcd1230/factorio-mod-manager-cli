use std::cmp::Ordering;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use serde::{Deserialize, Serialize};

use crate::error::AppError;

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
pub struct InstalledMod {
    pub name: String,
    pub enabled: bool,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct ModListFile {
    pub mods: Vec<InstalledMod>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FactorioVersion {
    parts: Vec<u32>,
}

impl FactorioVersion {
    pub fn parse(input: &str) -> Result<Self, AppError> {
        let parts = input
            .trim()
            .split('.')
            .map(|part| {
                part.parse::<u32>()
                    .map_err(|_| AppError::message(format!("invalid version component: {part}")))
            })
            .collect::<Result<Vec<_>, _>>()?;
        if parts.is_empty() {
            return Err(AppError::message("version cannot be empty"));
        }
        Ok(Self { parts })
    }

    pub fn major_minor(&self) -> Self {
        let mut parts = self.parts.clone();
        parts.truncate(2);
        Self { parts }
    }
}

impl Display for FactorioVersion {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let joined = self
            .parts
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(".");
        f.write_str(&joined)
    }
}

impl FromStr for FactorioVersion {
    type Err = AppError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Self::parse(s)
    }
}

impl Ord for FactorioVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        let max_len = self.parts.len().max(other.parts.len());
        for index in 0..max_len {
            let left = *self.parts.get(index).unwrap_or(&0);
            let right = *other.parts.get(index).unwrap_or(&0);
            match left.cmp(&right) {
                Ordering::Equal => continue,
                ordering => return ordering,
            }
        }
        Ordering::Equal
    }
}

impl PartialOrd for FactorioVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DependencySpec {
    pub name: String,
    pub version_requirement: Option<VersionRequirement>,
    pub kind: DependencyKind,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DependencyKind {
    Required,
    Optional,
    HiddenOptional,
    Conflict,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct VersionRequirement {
    pub comparator: Comparator,
    pub version: FactorioVersion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Comparator {
    Lt,
    Lte,
    Eq,
    Gte,
    Gt,
}

impl VersionRequirement {
    pub fn matches(&self, version: &FactorioVersion) -> bool {
        match self.comparator {
            Comparator::Lt => version < &self.version,
            Comparator::Lte => version <= &self.version,
            Comparator::Eq => version == &self.version,
            Comparator::Gte => version >= &self.version,
            Comparator::Gt => version > &self.version,
        }
    }
}

pub fn parse_dependency(input: &str) -> Result<DependencySpec, AppError> {
    let collapsed = input.split_whitespace().collect::<String>();
    let (kind, remainder) = if let Some(rest) = collapsed.strip_prefix("(!)") {
        (DependencyKind::Conflict, rest)
    } else if let Some(rest) = collapsed.strip_prefix("(?)") {
        (DependencyKind::HiddenOptional, rest)
    } else if let Some(rest) = collapsed.strip_prefix('!') {
        (DependencyKind::Conflict, rest)
    } else if let Some(rest) = collapsed.strip_prefix('?') {
        (DependencyKind::Optional, rest)
    } else if let Some(rest) = collapsed.strip_prefix('~') {
        (DependencyKind::Required, rest)
    } else {
        (DependencyKind::Required, collapsed.as_str())
    };

    if remainder == "base" || remainder.is_empty() {
        return Err(AppError::message("dependency is not actionable"));
    }

    for token in ["<=", ">=", "<", ">", "="] {
        if let Some((name, version)) = remainder.split_once(token) {
            return Ok(DependencySpec {
                name: name.to_string(),
                version_requirement: Some(VersionRequirement {
                    comparator: match token {
                        "<" => Comparator::Lt,
                        "<=" => Comparator::Lte,
                        "=" => Comparator::Eq,
                        ">=" => Comparator::Gte,
                        ">" => Comparator::Gt,
                        _ => unreachable!(),
                    },
                    version: FactorioVersion::parse(version)?,
                }),
                kind,
            });
        }
    }

    Ok(DependencySpec {
        name: remainder.to_string(),
        version_requirement: None,
        kind,
    })
}

#[cfg(test)]
mod tests {
    use super::{Comparator, DependencyKind, FactorioVersion, parse_dependency};

    #[test]
    fn parses_required_dependency() {
        let spec = parse_dependency("boblibrary >= 1.1.0").unwrap();
        assert_eq!(spec.name, "boblibrary");
        assert_eq!(spec.kind, DependencyKind::Required);
        assert_eq!(
            spec.version_requirement.unwrap().comparator,
            Comparator::Gte
        );
    }

    #[test]
    fn parses_optional_conflict_and_hidden_dependencies() {
        assert_eq!(
            parse_dependency("?FNEI").unwrap().kind,
            DependencyKind::Optional
        );
        assert_eq!(
            parse_dependency("(!)badmod").unwrap().kind,
            DependencyKind::Conflict
        );
        assert_eq!(
            parse_dependency("(?)helper").unwrap().kind,
            DependencyKind::HiddenOptional
        );
    }

    #[test]
    fn version_comparison_handles_minor_only() {
        let a = FactorioVersion::parse("1.1").unwrap();
        let b = FactorioVersion::parse("1.1.2").unwrap();
        assert!(a < b);
        assert_eq!(b.major_minor().to_string(), "1.1");
    }
}
