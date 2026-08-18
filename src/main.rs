mod fill;

use crate::fill::{FillBuild, fetch_project_versions};
use clap::Parser;
use futures::StreamExt;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
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
  md5: Result<String, HashingError>,
}

impl HashedBuild {
  fn err(fill_build: FillBuild, err: HashingError) -> Self {
    Self { fill_build, md5: Err(err) }
  }

  fn ok(fill_build: FillBuild, md5: String) -> Self {
    Self { fill_build, md5: Ok(md5) }
  }
}

#[derive(Debug)]
enum HashingError {
  BadStatus(u16),
  DownloadFailed(reqwest::Error),
  DownloadFailedNoBody,
  Sha256Mismatch,
}

impl HashingError {
  pub fn to_string(&self) -> String {
    match self {
      HashingError::BadStatus(code) => format!("Bad return code: {}", code),
      HashingError::DownloadFailed(err) => format!("Failed to download: {}", err),
      HashingError::DownloadFailedNoBody => "Failed to download: no body returned".to_string(),
      HashingError::Sha256Mismatch => "Invalid Sha256 of downloaded artefact".to_string(),
    }
  }
}

#[tokio::main]
async fn main() {
  let args = Args::parse();

  let mut headers = HeaderMap::default();
  headers.append("User-Agent", HeaderValue::from_static("fill-build-hasher (strokkur.24@gmail.com)"));
  headers.append("accept", HeaderValue::from_static("application/json"));

  let client = Client::builder()
    .timeout(Duration::from_mins(10))
    .default_headers(headers)
    .build()
    .expect("Failed to build reqwest client.");

  let builds = get_all_builds(&args, &client, "paper").await;

  let total_bytes: u64 = builds.iter().map(|b| b.size).sum();

  let progress = MultiProgress::new();

  let builds_bar = progress.add(ProgressBar::new(builds.len() as u64));
  builds_bar.set_style(ProgressStyle::with_template("{bar:50.blue/cyan} {pos}/{len} ({elapsed}) {msg}").unwrap());
  builds_bar.enable_steady_tick(Duration::from_millis(50));

  let bytes_bar = progress.add(ProgressBar::new(total_bytes));
  bytes_bar.set_style(ProgressStyle::with_template("{bar:50.blue/cyan} {binary_bytes}/{binary_total_bytes} ({binary_bytes_per_sec}, {eta})").unwrap());
  builds_bar.enable_steady_tick(Duration::from_millis(50));

  let hashed_builds: Vec<HashedBuild> = futures::stream::iter(builds)
    .map(|build| hash_build(build, &client, bytes_bar.clone()))
    .buffer_unordered(args.buffer_size.unwrap_or(32))
    .inspect(|_| builds_bar.inc(1))
    .collect()
    .await;

  for build in hashed_builds {
    let name = build.fill_build.name;
    let build_str = match build.md5 {
      Ok(hash) => {
        format!("MD5 hash of {name} is {hash}")
      }
      Err(err) => format!("Failed to hash {name}: {}", err.to_string()),
    };
    builds_bar.println(build_str);
  }

  builds_bar.finish_with_message("Done!");
  bytes_bar.finish();
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

async fn hash_build(build: FillBuild, client: &Client, bytes_bar: ProgressBar) -> HashedBuild {
  let response = client.get(&build.url).send().await;
  let response = match response {
    Ok(ok) => ok,
    Err(e) => return HashedBuild::err(build, HashingError::DownloadFailed(e)),
  };

  if !response.status().is_success() {
    return HashedBuild::err(build, HashingError::BadStatus(response.status().as_u16()));
  }

  let mut sha256_hasher = Sha256::new();
  let mut md5_hasher = Md5::new();

  let mut stream = response.bytes_stream();
  while let Some(chunk) = stream.next().await {
    let chunk = match chunk {
      Ok(ok) => ok,
      Err(_) => return HashedBuild::err(build, HashingError::DownloadFailedNoBody),
    };
    sha256_hasher.update(&chunk);
    md5_hasher.update(&chunk);
    bytes_bar.inc(chunk.len() as u64);
  }

  let sha256 = hex::encode(sha256_hasher.finalize());
  if sha256 != build.sha256 {
    return HashedBuild::err(build, HashingError::Sha256Mismatch);
  }

  let md5 = hex::encode(md5_hasher.finalize());
  HashedBuild::ok(build, md5)
}
