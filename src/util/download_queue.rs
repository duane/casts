use crate::util::heap_stream::HeapStream;
use futures::stream::FuturesUnordered;
use futures::Future;
use futures::StreamExt;
use log::info;
use std::fmt;
use std::path::Path;
use std::pin::Pin;
use tokio::task::LocalSet;
use url::Url;

/// A trait representing an object-to-file download.
pub trait DownloadTraitObject: Ord + fmt::Display + Sized + Clone {
  /// The url to retrieve the object from.
  fn url(&self) -> &Url;

  /// The destination path of the file.
  fn path(&self) -> &Path;
}

/// A structure to manage the download queue state.
#[derive(Debug)]
pub struct DownloadQueue<T: DownloadTraitObject> {
  /// The heap-backed asynchronous communication between the worker tasks and the incoming download requests.
  heap_stream: HeapStream<T>,

  /// The set of worker tasks currently in-flight.
  workers: FuturesUnordered<Pin<Box<dyn Future<Output = ()>>>>,

  /// local set
  local_workers: LocalSet,
}

impl<T: DownloadTraitObject> Default for DownloadQueue<T> {
  fn default() -> Self {
    DownloadQueue {
      heap_stream: Default::default(),
      workers: Default::default(),
      local_workers: LocalSet::new(),
    }
  }
}

impl<T: DownloadTraitObject + 'static> DownloadQueue<T> {
  /// Add an item to the stream to be processed by the download workers.
  pub fn push(&mut self, obj: T) {
    self.heap_stream.push(obj);
  }

  /// Kick off the workers in a new local task.
  pub fn start(&mut self) {
    tokio::task::spawn_local(queue_worker(self.heap_stream.clone()));
  }
}

async fn queue_worker<T: DownloadTraitObject>(worker_stream: HeapStream<T>) {
  worker_stream
    .for_each_concurrent(3, move |download_object| download(download_object))
    .await
}

async fn download<T: DownloadTraitObject>(obj: T) {
  let local_obj = obj.clone();
  info!(
    "starting download {} to {}",
    local_obj.url().as_str(),
    local_obj.path().to_str().unwrap()
  );
  let url = obj.url().to_owned();
  let path = obj.path().to_owned();
  crate::net::stream_to_file_owned(url, path).await.unwrap();
  info!(
    "ended download: {} to {}",
    local_obj.url().as_str(),
    local_obj.path().to_str().unwrap()
  );
}
