use crate::Args;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Error;
use tokio_retry::Retry;
use tokio_retry::strategy::{ExponentialBackoff, jitter};

pub type FillError = String;

// /projects/{project}/version/{version}/builds endpoint
#[derive(Deserialize)]
struct FillBuildResponse {
  id: u16,
  downloads: FillBuildResponseDownloads,
}

#[derive(Deserialize)]
struct FillBuildResponseDownloads {
  #[serde(rename = "server:default")]
  server_default: FillBuildResponseDownload,
}

#[derive(Deserialize)]
struct FillBuildResponseDownload {
  name: String,
  checksums: FillBuildResponseChecksums,
  size: u64,
  url: String,
}

#[derive(Deserialize)]
struct FillBuildResponseChecksums {
  sha256: String,
}

// /projects/{project}/versions endpoint
#[derive(Deserialize)]
struct FillVersionsResponse {
  versions: Vec<FillVersionsVersionsResponse>,
}

#[derive(Deserialize)]
struct FillVersionsVersionsResponse {
  version: FillVersionsVersionResponse,
}

#[derive(Deserialize)]
struct FillVersionsVersionResponse {
  id: String,
}

// impl
#[derive(Clone)]
pub struct FillBuild {
  pub project: String,
  pub version: String,
  pub id: u16,
  pub name: String,
  pub sha256: String,
  pub url: String,
  pub size: u64,
}

impl FillBuild {
  fn from(value: FillBuildResponse, project: String, version: String) -> Self {
    FillBuild {
      project,
      version,
      id: value.id,
      name: value.downloads.server_default.name,
      sha256: value.downloads.server_default.checksums.sha256,
      url: value.downloads.server_default.url,
      size: value.downloads.server_default.size,
    }
  }
}

impl FillBuild {
  pub async fn get_all_builds(args: &Args, client: &Client, project: &str) -> Vec<FillBuild> {
    // let mut requests_count: u32 = 0;
    let mut all_builds: Vec<FillBuild> = Vec::new();

    let versions = Retry::start(ExponentialBackoff::from_millis(500).map(jitter).take(args.retries), || {
      fetch_project_versions(client, args.endpoint.clone(), project)
    })
    .await
    .unwrap_or_else(|err| panic!("Failed to fetch versions for {project} after retries: {err}"));
    // requests_count += 1;

    for version in versions {
      let builds = Retry::start(ExponentialBackoff::from_millis(500).map(jitter).take(args.retries), || {
        FillBuild::from_url(client, args.endpoint.clone(), project, version.clone())
      })
      .await
      .unwrap_or_else(|err| panic!("Failed to fetch builds for {project} {version} after retries: {err}"));
      // requests_count += 1;
      all_builds.extend(builds);

      if args.limit.is_some() && all_builds.len() > args.limit.unwrap() as usize {
        break;
      }
    }

    if args.limit.is_some() {
      all_builds.truncate(args.limit.unwrap() as usize)
    }

    println!();
    // println!("Total API requests: {requests_count}");
    all_builds
  }

  pub async fn from_url(client: &Client, fill: String, project: &str, version: String) -> Result<Vec<FillBuild>, FillError> {
    let res = client
      .get(format!("{fill}/projects/{project}/versions/{version}/builds"))
      .send()
      .await
      .map_err(|err| format!("Failed to fetch builds for {project} {version}: {err}"))?;
    if !res.status().is_success() {
      let status = res.status().as_u16();
      return Err(format!("Failed to fetch builds for {project} {version}: request returned error code {status}"));
    }
    Self::from_response(
      res
        .text()
        .await
        .map_err(|err| format!("Failed to fetch builds for {project} {version}: {err}"))?,
      project.to_string(),
      version.clone(),
    )
    .map_err(|err| format!("Failed to fetch builds for {project} {version}: {err}"))
  }

  pub fn from_response(response: String, project: String, version: String) -> Result<Vec<FillBuild>, Error> {
    let deserialized: Vec<FillBuildResponse> = serde_json::from_str(response.as_str())?;
    Ok(
      deserialized
        .into_iter()
        .map(|res| FillBuild::from(res, project.clone(), version.clone()))
        .collect(),
    )
  }
}

pub async fn fetch_project_versions(client: &Client, fill: String, project: &str) -> Result<Vec<String>, FillError> {
  let res = client
    .get(format!("{fill}/projects/{project}/versions"))
    .send()
    .await
    .map_err(|err| format!("Failed to fetch versions for {project}: {err}"))?;
  if !res.status().is_success() {
    let status = res.status().as_u16();
    return Err(format!("Failed to fetch versions for {project}: request returned error code {status}"));
  }

  let typed_res: FillVersionsResponse = serde_json::from_str(
    res
      .text()
      .await
      .map_err(|err| format!("Failed to fetch versions for {project}: {err}"))?
      .as_str(),
  )
  .map_err(|err| format!("Failed to serialize versions for {project}: {err}"))?;
  Ok(typed_res.versions.into_iter().map(|a| a.version.id).collect())
}
