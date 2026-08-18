use crate::Args;
use crate::hashing::HashingResult;
use csv::{Reader, WriterBuilder};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs::{File, OpenOptions, metadata};
use std::io::ErrorKind::NotFound;
use tokio::sync::mpsc::Sender;
use tokio::task::JoinHandle;

#[derive(Deserialize, Serialize, Clone)]
pub struct HashedFillBuild {
  project: String,
  version: String,
  build: u16,
  file_name: String,
  md5: String,
}

impl From<HashingResult> for HashedFillBuild {
  fn from(result: HashingResult) -> Self {
    HashedFillBuild {
      project: result.fill_build.project,
      version: result.fill_build.version,
      build: result.fill_build.id,
      file_name: result.fill_build.name,
      md5: result.md5,
    }
  }
}

pub struct CsvWriter {
  pub tx: Sender<HashedFillBuild>,
  pub handle: JoinHandle<()>,
}

impl CsvWriter {
  fn path(args: &Args, project: &str) -> String {
    args.file_path.clone().unwrap_or(format!("{}.csv", project))
  }

  pub fn read_existing(args: &Args, project: &str) -> HashSet<(String, String, u16)> {
    let path = Self::path(args, project);
    let file = match File::open(&path) {
      Ok(file) => file,
      Err(err) if err.kind() == NotFound => return HashSet::new(),
      Err(err) => panic!("Failed to open existing csv file {path}: {err}"),
    };

    Reader::from_reader(file)
      .deserialize::<HashedFillBuild>()
      .filter_map(|val| val.ok().map(|v| (v.project, v.version, v.build)))
      .collect()
  }

  pub fn spawn(args: &Args, project: &str) -> Self {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<HashedFillBuild>(128);
    let path = CsvWriter::path(args, project);
    let has_existing_rows = metadata(&path).map(|m| m.len() > 0).unwrap_or(false);

    let handle = tokio::spawn(async move {
      let file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)
        .expect(format!("Failed to create file: {}", path).as_str());
      let mut writer = WriterBuilder::new().has_headers(!has_existing_rows).from_writer(file);
      while let Some(hashed) = rx.recv().await {
        writer
          .serialize(&hashed)
          .expect(format!("Failed to write csv record for: {}", hashed.file_name).as_str());
        writer.flush().expect(format!("Failed to flush csv record for: {}", hashed.file_name).as_str());
      }
    });
    Self { tx, handle }
  }

  pub async fn close(self) {
    drop(self.tx);
    self.handle.await.expect("CSV writer task failed.");
  }
}
