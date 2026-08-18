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

#[derive(clap::Parser)]
pub struct Args {
  /// The max amount of builds to download. Can be used for testing purposes.
  #[arg(long)]
  pub limit: Option<u32>,

  /// The number of concurrent downloads to attempt.
  #[arg(long, default_value = "32")]
  pub buffer_size: usize,

  /// The target *.csv file to write to.
  #[arg(long)]
  pub file_path: Option<String>,

  /// The project to iterate all builds in.
  #[arg(long, short, required = true)]
  pub project: String,

  /// The fill endpoint to use.
  #[arg(long, default_value = "https://fill.papermc.io/v3")]
  pub endpoint: String,
}

#[derive(Debug, Clone)]
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
  let args = Args::parse();

  let mut headers = HeaderMap::default();
  headers.append("User-Agent", HeaderValue::from_static("fill-build-hasher (strokkur.24@gmail.com)"));
  headers.append("accept", HeaderValue::from_static("application/json"));

  let client = Client::builder()
    .timeout(Duration::from_mins(10))
    .default_headers(headers)
    .build()
    .expect("Failed to build reqwest client.");

  let builds = FillBuild::get_all_builds(&args, &client, args.project.clone()).await;

  let total_bytes: u64 = builds.iter().map(|b| b.size).sum();

  let progress = MultiProgress::new();

  let builds_bar = progress.add(ProgressBar::new(builds.len() as u64));
  builds_bar.set_style(ProgressStyle::with_template("{bar:50.blue/cyan} {pos}/{len} ({elapsed}) {msg}").unwrap());
  builds_bar.enable_steady_tick(Duration::from_millis(50));

  let bytes_bar = progress.add(ProgressBar::new(total_bytes));
  bytes_bar.set_style(ProgressStyle::with_template("{bar:50.blue/cyan} {binary_bytes}/{binary_total_bytes} ({binary_bytes_per_sec}, {eta})").unwrap());
  builds_bar.enable_steady_tick(Duration::from_millis(50));

  let writer = CsvWriter::new(&args, args.project.to_string());
  let writer_tx = writer.tx.clone();
  let writer_task = writer.spawn();

  futures::stream::iter(builds)
    .map(|build| {
      let tx = writer_tx.clone();
      let bytes_bar = bytes_bar.clone();
      let client = &client;
      async move {
        let build = HashingResult::hash_build(build, client, bytes_bar).await;
        match build.md5 {
          Ok(_) => match tx.send(HashedFillBuild::from(build.clone())).await {
            Ok(_) => Ok(build),
            Err(_) => Err(build),
          },
          Err(_) => Err(build),
        }
      }
    })
    .buffer_unordered(args.buffer_size)
    .for_each(|res| {
      builds_bar.inc(1);
      if let Err(err) = res {
        builds_bar.println(format!("Failed: {}", err.fill_build.name))
      }
      ready(())
    })
    .await;

  drop(writer_tx);
  writer_task.await.expect("Failed to close writer task.");

  builds_bar.finish_with_message("Done!");
  bytes_bar.finish();
}
