use std::cmp::Reverse;

use reqwest::{Client, Response};
use serde::Deserialize;

use crate::domain::{Comparator, DependencyKind, DependencySpec, FactorioVersion, VersionRequirement, parse_dependency};
use crate::error::AppError;

const PORTAL_BASE: &str = "https://mods.factorio.com";
const API_BASE: &str = "https://mods.factorio.com/api/mods";

#[derive(Clone)]
pub struct PortalClient {
    http: Client,
    username: Option<String>,
    token: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ModApiResponse {
    pub releases: Vec<Release>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Release {
    pub download_url: String,
    pub file_name: String,
    pub released_at: String,
    pub sha1: String,
    pub version: String,
    pub info_json: ReleaseInfoJson,
}

#[derive(Clone, Debug, Deserialize)]
pub struct ReleaseInfoJson {
    pub factorio_version: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
}

impl PortalClient {
    pub fn new(username: Option<String>, token: Option<String>) -> Result<Self, AppError> {
        let http = Client::builder().user_agent("factorio-mod-manager-rust").build()?;
        Ok(Self {
            http,
            username,
            token,
        })
    }

    pub async fn fetch_mod(&self, mod_name: &str) -> Result<ModApiResponse, AppError> {
        let response = self
            .http
            .get(format!("{API_BASE}/{mod_name}/full"))
            .send()
            .await?;
        if !response.status().is_success() {
            return Err(AppError::message(format!(
                "could not fetch metadata for mod `{mod_name}`"
            )));
        }
        Ok(response.json().await?)
    }

    pub fn select_release(
        &self,
        response: &ModApiResponse,
        factorio_version: &FactorioVersion,
        min_version: Option<&VersionRequirement>,
        downgrade: bool,
    ) -> Result<Option<Release>, AppError> {
        let mut releases = response.releases.clone();
        releases.sort_by_key(|release| Reverse(release.released_at.clone()));
        for release in releases {
            let version = FactorioVersion::parse(&release.info_json.factorio_version)?.major_minor();
            let matches_factorio = if downgrade {
                version <= *factorio_version
            } else {
                version == *factorio_version
            };
            if !matches_factorio {
                continue;
            }
            if let Some(requirement) = min_version {
                let candidate = FactorioVersion::parse(&release.version)?;
                if !requirement.matches(&candidate) {
                    continue;
                }
            }
            return Ok(Some(release));
        }
        Ok(None)
    }

    pub fn dependencies_for_release(&self, release: &Release) -> Vec<DependencySpec> {
        release
            .info_json
            .dependencies
            .iter()
            .filter_map(|entry| parse_dependency(entry).ok())
            .collect()
    }

    pub async fn download_release(&self, release: &Release) -> Result<Response, AppError> {
        let mut request = self.http.get(format!("{PORTAL_BASE}{}", release.download_url));
        if let (Some(username), Some(token)) = (&self.username, &self.token) {
            request = request.query(&[("username", username), ("token", token)]);
        }
        Ok(request.send().await?)
    }
}

pub fn parse_version_requirement(input: &str) -> Result<VersionRequirement, AppError> {
    for token in ["<=", ">=", "<", ">", "="] {
        if let Some((_, version)) = input.split_once(token) {
            return Ok(VersionRequirement {
                comparator: match token {
                    "<" => Comparator::Lt,
                    "<=" => Comparator::Lte,
                    "=" => Comparator::Eq,
                    ">=" => Comparator::Gte,
                    ">" => Comparator::Gt,
                    _ => unreachable!(),
                },
                version: FactorioVersion::parse(version)?,
            });
        }
    }

    Ok(VersionRequirement {
        comparator: Comparator::Gte,
        version: FactorioVersion::parse(input)?,
    })
}

pub fn classify_dependencies(
    dependencies: &[DependencySpec],
) -> (Vec<DependencySpec>, Vec<DependencySpec>, Vec<DependencySpec>) {
    let mut required = Vec::new();
    let mut optional = Vec::new();
    let mut conflicts = Vec::new();
    for dependency in dependencies {
        match dependency.kind {
            DependencyKind::Required => required.push(dependency.clone()),
            DependencyKind::Optional => optional.push(dependency.clone()),
            DependencyKind::Conflict => conflicts.push(dependency.clone()),
            DependencyKind::HiddenOptional => {}
        }
    }
    (required, optional, conflicts)
}
