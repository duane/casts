use tokio::runtime::Runtime;
use std::fs::rename;

use std::path::Path;
use url::Url;
use std::io::Write;
use futures::StreamExt;
use indicatif::ProgressBar;
use tempfile::NamedTempFile;

pub async fn stream_to_file(url: &Url, file: &Path) -> reqwest::Result<()>  {
  let mut out_file = NamedTempFile::new().unwrap();
  let response = reqwest::get(&url.to_string()).await?;
  let content_length_header = response.headers().get("content-length").map(|content_length_string| {
    content_length_string.to_str().unwrap().parse::<u64>().unwrap()
  });

  let mut stream = response.bytes_stream();

  let bar = match content_length_header {
    Some(size) => ProgressBar::new(size),
    None => ProgressBar::new_spinner(),

  };
  while let Some(item) = stream.next().await {
    let chunk = item?;
    out_file.write_all(chunk.as_ref()).unwrap();
    bar.inc(chunk.len() as u64)
  }

  let path = out_file.into_temp_path();
  rename(&path, &file).unwrap();


  Ok(())
}

pub fn blocking_stream_to_file(url: &Url, file: &Path) {
  Runtime::new()
        .expect("Failed to create Tokio runtime").block_on(stream_to_file(url, file)).unwrap();
}
