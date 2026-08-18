use crate::FILL_ENDPOINT;
use reqwest::Client;
use serde::Deserialize;
use serde_json::Error;

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
pub struct FillBuild {
  pub id: u16,
  pub name: String,
  pub sha256: String,
  pub url: String,
  pub size: u64,
}

impl From<FillBuildResponse> for FillBuild {
  fn from(value: FillBuildResponse) -> Self {
    FillBuild {
      id: value.id,
      name: value.downloads.server_default.name,
      sha256: value.downloads.server_default.checksums.sha256,
      url: value.downloads.server_default.url,
      size: value.downloads.server_default.size,
    }
  }
}

impl FillBuild {
  pub async fn from_url(client: &Client, project: &str, version: &str) -> Result<Vec<FillBuild>, FillError> {
    let res = client
      .get(format!("{FILL_ENDPOINT}/projects/{project}/versions/{version}/builds"))
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
    )
    .map_err(|err| format!("Failed to fetch builds for {project} {version}: {err}"))
  }

  pub fn from_response(response: String) -> Result<Vec<FillBuild>, Error> {
    let deserialized: Vec<FillBuildResponse> = serde_json::from_str(response.as_str())?;
    Ok(deserialized.into_iter().map(Into::into).collect())
  }
}

pub async fn fetch_project_versions(client: &Client, project: &str) -> Result<Vec<String>, FillError> {
  let res = client
    .get(format!("{FILL_ENDPOINT}/projects/{project}/versions"))
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
