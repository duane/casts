use futures::task::Context;
use futures::task::Poll;
use futures::Stream;
use log::debug;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::{collections::BinaryHeap, task::Waker};

/// A structure to provide a heap-backed stream.
#[derive(Debug)]
pub struct HeapStream<T: Ord> {
  state: Arc<Mutex<HeapStreamState<T>>>,
}

impl<T: Ord> Clone for HeapStream<T> {
  fn clone(&self) -> Self {
    HeapStream {
      state: self.state.clone(),
    }
  }
}

impl<T: Ord> Default for HeapStream<T> {
  fn default() -> Self {
    HeapStream {
      state: Arc::new(Mutex::new(Default::default())),
    }
  }
}

/// The internal state.
#[derive(Debug)]
struct HeapStreamState<T: Ord> {
  /// The actual unbounded storage of the stream thus far.
  heap: BinaryHeap<T>,

  /// The waker used to communicate the heap has been updated.
  waker: Option<Waker>,
}

impl<T: Ord> Default for HeapStreamState<T> {
  fn default() -> Self {
    HeapStreamState {
      heap: BinaryHeap::new(),
      waker: None,
    }
  }
}

impl<T: Ord> Stream for HeapStream<T> {
  type Item = T;

  fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
    let mut state = self.state.lock().unwrap();
    match state.heap.pop() {
      Some(obj) => {
        debug!("ready");
        Poll::Ready(Some(obj))
      }
      None => {
        debug!("pending");
        state.waker = Some(cx.waker().clone());
        Poll::Pending
      }
    }
  }
}

impl<T: Ord> HeapStream<T> {
  /// Add an item to the heap stream.
  pub fn push(&mut self, item: T) {
    let mut state = self.state.lock().unwrap();
    state.heap.push(item);
    if let Some(waker) = state.waker.take() {
      waker.wake()
    }
  }
}
