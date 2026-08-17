mod fill;

use crate::fill::{FillBuild, fetch_project_versions};
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue};

pub const FILL_ENDPOINT: &str = "https://fill.papermc.io/v3";
pub const PROJECTS: [&str; 4] = ["paper", "folia", "velocity", "waterfall"];

#[tokio::main]
async fn main() {
  let mut headers = HeaderMap::default();
  headers.append("User-Agent", HeaderValue::from_static("fill-build-hasher (strokkur.24@gmail.com)"));
  headers.append("accept", HeaderValue::from_static("application/json"));

  let client = Client::builder().default_headers(headers).build().expect("Failed to build reqwest client.");

  let mut requests_count: u32 = 0;
  let mut builds_count: u32 = 0;
  
  let versions = fetch_project_versions(&client, "paper").await.unwrap();
  requests_count += 1;
  for version in versions {
    let builds = FillBuild::from_url(&client, "paper", version.as_str()).await.unwrap();
    requests_count += 1;
    for build in builds {
      let id = build.id;
      let name = build.name;
      let sha = build.sha256;
      builds_count += 1;
      println!("{id} | {name} | {sha}")
    }
  }
  
  println!();
  print!("Total API requests: {requests_count} | Total Paper builds: {builds_count}");
}
