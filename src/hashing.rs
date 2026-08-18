use crate::HashingError;
use crate::fill::FillBuild;
use futures::StreamExt;
use indicatif::ProgressBar;
use md5::{Digest, Md5};
use reqwest::Client;
use sha2::Sha256;

#[derive(Clone)]
pub struct HashingResult {
  pub fill_build: FillBuild,
  pub md5: Result<String, HashingError>,
}

impl HashingResult {
  fn err(fill_build: FillBuild, err: HashingError) -> Self {
    Self { fill_build, md5: Err(err) }
  }

  fn ok(fill_build: FillBuild, md5: String) -> Self {
    Self { fill_build, md5: Ok(md5) }
  }

  pub async fn hash_build(build: FillBuild, client: &Client, bytes_bar: ProgressBar) -> HashingResult {
    let response = client.get(&build.url).send().await;
    let response = match response {
      Ok(ok) => ok,
      Err(e) => return HashingResult::err(build, HashingError::DownloadFailed(e.to_string())),
    };

    if !response.status().is_success() {
      return HashingResult::err(build, HashingError::BadStatus(response.status().as_u16()));
    }

    let mut sha256_hasher = Sha256::new();
    let mut md5_hasher = Md5::new();

    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
      let chunk = match chunk {
        Ok(ok) => ok,
        Err(_) => return HashingResult::err(build, HashingError::DownloadFailedNoBody),
      };
      sha256_hasher.update(&chunk);
      md5_hasher.update(&chunk);
      bytes_bar.inc(chunk.len() as u64);
    }

    let sha256 = hex::encode(sha256_hasher.finalize());
    if sha256 != build.sha256 {
      return HashingResult::err(build, HashingError::Sha256Mismatch);
    }

    let md5 = hex::encode(md5_hasher.finalize());
    HashingResult::ok(build, md5)
  }
}
