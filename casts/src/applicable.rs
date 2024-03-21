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
}

impl<E: EventApplicable + Unpin> ApplicableStream<E> {
    pub fn subscribe_aggregate(aggregate: E, backing_stream: CommitStream) -> Self {
        ApplicableStream {
            aggregate,
            backing_stream,
        }
    }
}

pub enum AggregateStreamError {
    CommitStream(CommitStreamError),
}

impl<E: EventApplicable + Unpin + Clone> Stream for ApplicableStream<E> {
    type Item = Result<(Commit, E), AggregateStreamError>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> Poll<Option<Self::Item>> {
        let mself = self.get_mut();
        let result = match Pin::new(&mut mself.backing_stream).poll_next(cx) {
            Poll::Pending => return Poll::Pending,
            Poll::Ready(None) => return Poll::Ready(None),
            Poll::Ready(Some(result)) => result,
        };
        match result {
            Ok(v) => {
                let events: Vec<E::Event> =
                    serde_json::from_value(v.events.clone()).expect("failed to decode events json");
                // Todo: this clone should be unnecessary. figure out how to modify the aggregate in place.
                mself.aggregate = mself.aggregate.clone().apply_many(events);
                Poll::Ready(Some(Ok((v, mself.aggregate.clone()))))
            }
            Err(err) => Poll::Ready(Some(Err(AggregateStreamError::CommitStream(err)))),
        }
    }
}
