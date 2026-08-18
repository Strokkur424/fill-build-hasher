mod fill;

use crate::fill::{FillBuild, fetch_project_versions};
use clap::Parser;
use futures::StreamExt;
use indicatif::{ProgressBar, ProgressStyle};
use md5::Md5;
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use sha2::{Digest, Sha256};
use std::time::Duration;

pub const FILL_ENDPOINT: &str = "https://fill.papermc.io/v3";
pub const PROJECTS: [&str; 4] = ["paper", "folia", "velocity", "waterfall"];

#[derive(clap::Parser)]
struct Args {
  #[arg(long)]
  limit: Option<u32>,

  #[arg(long)]
  buffer_size: Option<usize>,
}

struct HashedBuild {
  fill_build: FillBuild,
  md5: String,
}

#[derive(Debug)]
enum HashingError {
  DownloadFailed,
  DownloadFailedNoBody,
  Sha256Mismatch,
}

#[tokio::main]
async fn main() {
  let args = Args::parse();

  let mut headers = HeaderMap::default();
  headers.append("User-Agent", HeaderValue::from_static("fill-build-hasher (strokkur.24@gmail.com)"));
  headers.append("accept", HeaderValue::from_static("application/json"));

  let client = Client::builder()
    .timeout(Duration::from_mins(10))
    .default_headers(headers).build().expect("Failed to build reqwest client.");

  let builds = get_all_builds(&args, &client, "paper").await;

  let progress = ProgressBar::new(builds.len() as u64);
  progress.set_style(
    ProgressStyle::with_template("{bar:50.blue/cyan} {pos}/{len} ({elapsed}) {msg}").unwrap()
  );
  progress.enable_steady_tick(Duration::from_millis(50));

  let hashed_builds: Vec<Result<HashedBuild, HashingError>> = futures::stream::iter(builds)
    .map(|build| hash_build(build, &client))
    .buffer_unordered(args.buffer_size.unwrap_or(32))
    .inspect(|_| progress.inc(1))
    .collect()
    .await;

  for build in hashed_builds {
    let build_str = match build {
      Ok(hashed) => {
        let name = hashed.fill_build.name;
        let md5 = hashed.md5;
        format!("MD5 hash of {name} is {md5}")
      }
      Err(err) => format!("Failed to hash: {:?}", err),
    };
    progress.println(build_str);
  }

  progress.finish_with_message("Done!");
}

async fn get_all_builds(args: &Args, client: &Client, project: &str) -> Vec<FillBuild> {
  let mut requests_count: u32 = 0;
  let mut all_builds: Vec<FillBuild> = Vec::new();

  let versions = fetch_project_versions(&client, project).await.unwrap();
  requests_count += 1;

  for version in versions {
    let builds = FillBuild::from_url(&client, project, version.as_str()).await.unwrap();
    requests_count += 1;
    all_builds.extend(builds);

    if args.limit.is_some() && all_builds.len() > args.limit.unwrap() as usize {
      break;
    }
  }

  if args.limit.is_some() {
    all_builds.truncate(args.limit.unwrap() as usize)
  }

  println!();
  println!("Total API requests: {requests_count}");
  all_builds
}

async fn hash_build(build: FillBuild, client: &Client) -> Result<HashedBuild, HashingError> {
  let response = client.get(&build.url).send().await.map_err(|_| HashingError::DownloadFailed)?;
  let bytes = response.bytes().await.map_err(|_| HashingError::DownloadFailedNoBody)?;
  let sha256 = hex::encode(Sha256::digest(&bytes));
  if !sha256.eq(&build.sha256) {
    return Err(HashingError::Sha256Mismatch);
  }
  let md5 = hex::encode(Md5::digest(&bytes));
  Ok(HashedBuild { fill_build: build, md5 })
}
