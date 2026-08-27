// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

use std::{pin::Pin, task::Poll};

use crate::events::Event;
use commit_store::{Commit, CommitStream, CommitStreamError};
use futures_core::Stream;

/// A datatype that may be mutated with an event.
pub trait EventApplicable {
    /// The event type used to mutate this data.
    type Event: Event;

    /// The function to produce a new version of this data type with the event applied.
    fn apply(self, event: Self::Event) -> Self;
    fn apply_many(self, events: Vec<Self::Event>) -> Self;
}

/// Continually apply the incoming event to the aggregate, producing a new one of an identical type.
///
pub struct ApplicableStream<E: EventApplicable + Unpin> {
    aggregate: E,
    backing_stream: CommitStream,
    /// Set once an error has been emitted; the stream ends afterwards rather
    /// than folding further commits onto an aggregate with a hole in it.
    failed: bool,
}

impl<E: EventApplicable + Unpin> ApplicableStream<E> {
    pub fn subscribe_aggregate(aggregate: E, backing_stream: CommitStream) -> Self {
        ApplicableStream {
            aggregate,
            backing_stream,
            failed: false,
        }
    }
}

#[derive(Debug)]
pub enum AggregateStreamError {
    CommitStream(CommitStreamError),

    /// A commit's stored events could not be decoded into this aggregate's
    /// event type.
    Decode(String),
}

impl std::fmt::Display for AggregateStreamError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AggregateStreamError::CommitStream(err) => write!(f, "commit stream error: {err}"),
            AggregateStreamError::Decode(err) => write!(f, "could not decode events: {err}"),
        }
    }
}

impl std::error::Error for AggregateStreamError {}

impl<E: EventApplicable + Unpin + Clone> Stream for ApplicableStream<E> {
    type Item = Result<(Commit, E), AggregateStreamError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mself = self.get_mut();
        // A running fold cannot survive a hole. Once a commit is missed or
        // undecodable, every later aggregate is built from an incomplete
        // history and is quietly wrong -- so the stream ends at the first
        // error rather than continuing to emit corrupt state. The consumer
        // re-subscribes, which re-hydrates from the store.
        if mself.failed {
            return Poll::Ready(None);
        }
        let result = match Pin::new(&mut mself.backing_stream).poll_next(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(None) => return Poll::Ready(None),
            Poll::Ready(Some(result)) => result,
        };
        match result {
            Ok(v) => match serde_json::from_value::<Vec<E::Event>>(v.events.clone()) {
                Ok(events) => {
                    // Todo: this clone should be unnecessary. figure out how to modify the aggregate in place.
                    mself.aggregate = mself.aggregate.clone().apply_many(events);
                    Poll::Ready(Some(Ok((v, mself.aggregate.clone()))))
                }
                Err(err) => {
                    mself.failed = true;
                    Poll::Ready(Some(Err(AggregateStreamError::Decode(err.to_string()))))
                }
            },
            Err(err) => {
                mself.failed = true;
                Poll::Ready(Some(Err(AggregateStreamError::CommitStream(err))))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AggregateStreamError, ApplicableStream};
    use crate::aggregate::FeedAggregate;
    use chrono::Utc;
    use commit_store::commit::{CommitAttempt, CommitMetadata, StreamMetadata};
    use commit_store::dispatch::LocalSubscription;
    use commit_store::{Commit, CommitStream};
    use futures::StreamExt;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::sync::Arc;
    use uuid::Uuid;

    fn commit(stream_id: Uuid, version: i64) -> Commit {
        CommitAttempt {
            stream_metadata: StreamMetadata {
                stream_id,
                stream_version: version,
            },
            commit_metadata: CommitMetadata {
                commit_id: Uuid::new_v4(),
                commit_timestamp: Utc::now(),
                commit_sequence: version,
            },
            metadata: serde_json::Value::Null,
            events: serde_json::json!([]),
            events_count: 0,
            snapshot: false,
        }
        .into_commit(version + 1)
        .unwrap()
    }

    /// A gap invalidates the running fold: every aggregate after it would be
    /// built from an incomplete history. The stream must end at the error
    /// rather than keep emitting quietly-wrong state for the subscriber to
    /// render.
    #[tokio::test]
    async fn the_stream_ends_at_the_first_error() {
        let stream_id = Uuid::new_v4();
        let (tx, rx) = tokio::sync::mpsc::channel(10);
        let missed = Arc::new(AtomicU64::new(0));
        let commits = CommitStream::from_subscription(LocalSubscription {
            rx,
            missed: Arc::clone(&missed),
        });
        let mut aggregates =
            ApplicableStream::subscribe_aggregate(FeedAggregate::default(), commits);

        tx.send(Arc::new(commit(stream_id, 0))).await.unwrap();
        assert!(aggregates.next().await.unwrap().is_ok());

        // The dispatcher reports that it dropped a commit for this subscriber.
        missed.store(3, Ordering::Relaxed);
        let err = aggregates.next().await.unwrap().unwrap_err();
        assert!(
            matches!(err, AggregateStreamError::CommitStream(_)),
            "expected the lag to surface: {err}"
        );

        // Even with more commits queued, the stream is over.
        tx.send(Arc::new(commit(stream_id, 1))).await.unwrap();
        assert!(
            aggregates.next().await.is_none(),
            "a fold with a hole in it must not keep emitting aggregates"
        );
    }
}
