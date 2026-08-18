mod fill;
mod hashing;
mod writer;

use crate::fill::FillBuild;
use crate::hashing::HashingResult;
use crate::writer::{CsvWriter, HashedFillBuild};
use clap::Parser;
use future::ready;
use futures::{StreamExt, future};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use reqwest::Client;
use reqwest::header::{HeaderMap, HeaderValue};
use std::time::Duration;
use tokio::time::sleep;
use tokio_retry::RetryIf;
use tokio_retry::strategy::{ExponentialBackoff, jitter};

#[derive(clap::Parser)]
pub struct Args {
  /// The max amount of builds to download. Can be used for testing purposes.
  #[arg(long)]
  pub limit: Option<u32>,

  /// The number of concurrent downloads to attempt.
  #[arg(long, default_value = "32")]
  pub buffer_size: usize,

  /// The amount of retries for each fill API call, if it fails.
  #[arg(long, short, default_value = "5")]
  pub retries: usize,

  /// The target *.csv file to write to.
  #[arg(long)]
  pub file_path: Option<String>,

  /// The project to iterate all builds in.
  #[arg(long, short, required = true, num_args = 1..)]
  pub projects: Vec<String>,

  /// The fill endpoint to use.
  #[arg(long, default_value = "https://fill.papermc.io/v3")]
  pub endpoint: String,

  /// Whether to skip the timeout between different project hashing.
  #[arg(long, default_value = "false")]
  pub skip_timeout: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum HashingError {
  BadStatus(u16),
  DownloadFailed(String),
  DownloadFailedNoBody,
  Sha256Mismatch,

  WriterDroppedEntry,
}

impl HashingError {
  pub fn to_string(&self) -> String {
    match self {
      HashingError::BadStatus(code) => format!("Bad return code: {}", code),
      HashingError::DownloadFailed(err) => format!("Failed to download: {}", err),
      HashingError::DownloadFailedNoBody => "Failed to download: no body returned".to_string(),
      HashingError::Sha256Mismatch => "Invalid Sha256 of downloaded artefact".to_string(),
      HashingError::WriterDroppedEntry => "Failed to write hashed to csv file".to_string(),
    }
  }
}

#[tokio::main]
async fn main() {
  let args = &Args::parse();

  let mut headers = HeaderMap::default();
  headers.append("User-Agent", HeaderValue::from_static("fill-build-hasher (strokkur.24@gmail.com)"));
  headers.append("accept", HeaderValue::from_static("application/json"));

  let client = &Client::builder()
    .timeout(Duration::from_mins(60))
    .default_headers(headers)
    .build()
    .expect("Failed to build reqwest client.");

  for project in &args.projects {
    run_for_project(args, client, project.as_str()).await;
  }
}

async fn run_for_project(args: &Args, client: &Client, project: &str) {
  println!("Starting download for project: {project}");

  let builds = FillBuild::get_all_builds(args, client, project).await;
  let finished_builds = CsvWriter::read_existing(args, project);

  let builds_len = builds.len() as u64;
  let builds: Vec<FillBuild> = builds
    .into_iter()
    .filter(|b| !finished_builds.contains(&(project.to_string(), b.version.clone(), b.id)))
    .collect();

  let skipped_builds = builds_len - builds.len() as u64;
  if skipped_builds > 0 {
    println!("Skipped {skipped_builds} builds already hashed.");
  }

  let total_bytes: u64 = builds.iter().map(|b| b.size).sum();

  let progress = MultiProgress::new();

  let builds_bar = progress.add(ProgressBar::new(builds_len));
  builds_bar.inc(skipped_builds);
  builds_bar.set_style(ProgressStyle::with_template("{bar:50.blue/cyan} {pos}/{len} ({elapsed}) {msg}").unwrap());
  builds_bar.enable_steady_tick(Duration::from_millis(50));

  let bytes_bar = progress.add(ProgressBar::new(total_bytes));
  bytes_bar.set_style(ProgressStyle::with_template("{bar:50.blue/cyan} {binary_bytes}/{binary_total_bytes} ({binary_bytes_per_sec}, {eta})").unwrap());
  builds_bar.enable_steady_tick(Duration::from_millis(50));

  let writer = CsvWriter::spawn(args, project);

  futures::stream::iter(builds)
    .map(|build| {
      let tx = writer.tx.clone();
      let bytes_bar_clone = &bytes_bar;
      let client = &client;
      async move {
        let result = RetryIf::start(
          ExponentialBackoff::from_millis(500).map(jitter).take(args.retries),
          || HashingResult::hash_build(&build, client, bytes_bar_clone),
          |err: &HashingError| match err {
            HashingError::BadStatus(code) => *code >= 500,
            _ => true,
          },
        )
        .await;
        match result {
          Ok(result) => {
            let _ = tx.send(HashedFillBuild::from(result)).await.expect("Writer task closed unexpectedly");
            Ok(())
          }
          Err(err) => Err((build, err)),
        }
      }
    })
    .buffer_unordered(args.buffer_size)
    .for_each(|res| {
      builds_bar.inc(1);
      if let Err((build, err)) = res {
        builds_bar.println(format!("Failed to hash {}: {}", build.name, err.to_string()))
      }
      ready(())
    })
    .await;

  writer.close().await;

  progress.remove(&bytes_bar);
  builds_bar.finish_with_message("Done!");
  if !args.skip_timeout {
    sleep(Duration::from_secs(2)).await;
  }
}
