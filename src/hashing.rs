use crate::HashingError;
use crate::fill::FillBuild;
use futures::StreamExt;
use indicatif::ProgressBar;
use md5::{Digest, Md5};
use reqwest::Client;
use sha1::Sha1;
use sha2::{Sha256, Sha512};

pub struct HashingResult {
  pub fill_build: FillBuild,
  pub md5: String,
  pub sha1: String,
  pub sha512: String,
}

impl HashingResult {
  fn new(fill_build: FillBuild, md5: String, sha1: String, sha512: String) -> Self {
    Self { fill_build, md5, sha1, sha512 }
  }

  pub async fn hash_build(build: &FillBuild, client: &Client, bytes_bar: &ProgressBar) -> Result<HashingResult, HashingError> {
    let response = client.get(&build.url).send().await;
    let response = match response {
      Ok(ok) => ok,
      Err(err) => return Err(HashingError::DownloadFailed(err.to_string())),
    };

    if !response.status().is_success() {
      return Err(HashingError::BadStatus(response.status().as_u16()));
    }

    let mut sha1_hasher = Sha1::new();
    let mut sha256_hasher = Sha256::new();
    let mut sha512_hasher = Sha512::new();
    let mut md5_hasher = Md5::new();

    let mut bytes_this_attempt: u64 = 0;
    let mut stream = response.bytes_stream();
    while let Some(chunk) = stream.next().await {
      let chunk = match chunk {
        Ok(ok) => ok,
        Err(_) => {
          bytes_bar.dec(bytes_this_attempt);
          return Err(HashingError::DownloadFailedNoBody);
        }
      };
      sha1_hasher.update(&chunk);
      sha256_hasher.update(&chunk);
      sha512_hasher.update(&chunk);
      md5_hasher.update(&chunk);
      bytes_bar.inc(chunk.len() as u64);
      bytes_this_attempt += chunk.len() as u64;
    }

    let sha256 = hex::encode(sha256_hasher.finalize());
    if sha256 != build.sha256 {
      bytes_bar.dec(bytes_this_attempt);
      return Err(HashingError::Sha256Mismatch);
    }

    let sha1 = hex::encode(sha1_hasher.finalize());
    let sha512 = hex::encode(sha512_hasher.finalize());
    let md5 = hex::encode(md5_hasher.finalize());
    Ok(HashingResult::new(build.clone(), md5, sha1, sha512))
  }
}
