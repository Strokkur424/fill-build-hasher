mod fill;

use crate::fill::{FillBuild, fetch_project_versions};
use clap::Parser;
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue};

pub const FILL_ENDPOINT: &str = "https://fill.papermc.io/v3";
pub const PROJECTS: [&str; 4] = ["paper", "folia", "velocity", "waterfall"];

#[derive(clap::Parser)]
struct Args {
  #[arg(long)]
  limit: Option<u32>,
}

#[tokio::main]
async fn main() {
  let args = Args::parse();

  let mut headers = HeaderMap::default();
  headers.append("User-Agent", HeaderValue::from_static("fill-build-hasher (strokkur.24@gmail.com)"));
  headers.append("accept", HeaderValue::from_static("application/json"));

  let client = Client::builder().default_headers(headers).build().expect("Failed to build reqwest client.");

  let mut requests_count: u32 = 0;
  let mut all_builds: Vec<FillBuild> = Vec::new();

  let versions = fetch_project_versions(&client, "paper").await.unwrap();
  requests_count += 1;

  for version in versions {
    let builds = FillBuild::from_url(&client, "paper", version.as_str()).await.unwrap();
    requests_count += 1;
    all_builds.extend(builds);

    if args.limit.is_some() && all_builds.len() > args.limit.unwrap() as usize {
      break;
    }
  }

  if args.limit.is_some() {
    all_builds.truncate(args.limit.unwrap() as usize)
  }

  for build in all_builds {
    let name = build.name;
    let url = build.url;
    println!("{name} | {url}")
  }

  println!();
  println!("Total API requests: {requests_count}");
}
