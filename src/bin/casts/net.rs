use std::fs::rename;
use tokio::runtime::Runtime;

use futures::StreamExt;
use std::io::Write;
use std::path::Path;
use tempfile::NamedTempFile;
use url::Url;

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

pub fn blocking_stream_to_file(url: &Url, file: &Path) -> usize {
  Runtime::new()
    .expect("Failed to create Tokio runtime")
    .block_on(stream_to_file(url, file))
    .unwrap()
}
