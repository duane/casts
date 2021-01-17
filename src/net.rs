use std::fs::rename;
use tokio::runtime::Runtime;

use futures::{Future, StreamExt};
use std::io::Write;
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;
use futures::future::Map;
use url::Url;
use std::time::Duration;

/// Download the file asynchronously.
pub async fn stream_to_file(url: &Url, file: &Path) -> reqwest::Result<usize> {
  let mut out_file = NamedTempFile::new().unwrap();
  let response = reqwest::get(&url.to_string()).await?;

  let mut stream = response.bytes_stream();

  let mut total_size: usize = 0;

  while let Some(item) = stream.next().await {
    let chunk = item?;
    out_file.write_all(chunk.as_ref()).unwrap();
    total_size += chunk.len();
  }

  let path = out_file.into_temp_path();
  rename(&path, &file).unwrap();

  Ok(total_size)
}

/// blah
pub async fn stream_to_file_owned(url: Url, file: PathBuf) -> reqwest::Result<usize> {
  let mut out_file = NamedTempFile::new().unwrap();
  let response = reqwest::get(&url.to_string()).await?;

  let mut stream = response.bytes_stream();

  let mut total_size: usize = 0;

  while let Some(item) = stream.next().await {
    let chunk = item?;
    out_file.write_all(chunk.as_ref()).unwrap();
    total_size += chunk.len();
  }

  let path = out_file.into_temp_path();
  rename(&path, &file).unwrap();

  Ok(total_size)
}

/*
/// Download the file asynchronously.
pub fn stream_to_file_best(url: url::Url, file: PathBuf) -> Map<(), ()> {
  let client = Client::builder()
    .pool_idle_timeout(Duration::from_secs(30))
    .build_http();
  let mut request = Request::builder()
    .uri(url.as_str().parse().unwrap())
    .header("User-Agent", "my-awesome-agent/1.0");
  
}*/

/// Block until the file is completely downloaded.
pub fn blocking_stream_to_file(url: &Url, file: &Path) -> usize {
  Runtime::new()
    .expect("Failed to create Tokio runtime")
    .block_on(stream_to_file(url, file))
    .unwrap()
}
