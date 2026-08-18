use crate::Args;
use crate::hashing::HashingResult;
use csv::Writer;
use serde::{Deserialize, Serialize};
use std::fs::File;
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
  pub fn spawn(args: &Args, project: String) -> Self {
    let (tx, mut rx) = tokio::sync::mpsc::channel::<HashedFillBuild>(128);
    let path = args.file_path.clone();
    let path = path.unwrap_or(format!("{}.csv", project));

    let handle = tokio::spawn(async move {
      let file = File::create(&path).expect(format!("Failed to create file: {}", path).as_str());
      let mut writer = Writer::from_writer(file);
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
