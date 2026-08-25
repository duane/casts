// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

//! The `Casts` gRPC service: the read/command surface the native clients
//! talk to.
//!
//! Lives in the library rather than the binary so it can be driven by tests
//! over an in-process transport; `casts-grpc-server` is only the entry point
//! that wires it to a real commit store and a listening socket.

// Every tonic handler returns `Result<_, tonic::Status>` and Status is large;
// the signature is fixed by the generated service trait.
#![allow(clippy::result_large_err)]

use anyhow::Result;
use chrono::{DateTime, Utc};
use commit_store::commit::{CommitMetadata, StreamMetadata};
use commit_store::dispatch::DispatchingAsyncReadWriteStore;
use commit_store::store::{ScanStreamCursor, StoreError};
use commit_store::CommitAttempt;
use futures::lock::Mutex;
use immutable_feed::{Content, Entry, Feed, Image, Link, MediaContent, MediaObject};
use log::info;
use prost_types::Timestamp as GrpcTimestamp;
use std::collections::HashMap;
use std::sync::Arc;
use tonic::transport::ServerTlsConfig;
use tonic::{Request, Response, Status};
use uuid::Uuid;

use crate::{
    aggregate::FeedAggregate,
    applicable::EventApplicable,
    commands::{Command, FeedAggregateCommand, FeedAggregateError},
    events::FeedAggregateEvent,
    grpc::Grpc2Store,
    grpc_interface::{
        AddFeedRequest, AddFeedResponse, Content as GrpcContent, Entries as GrpcEntries,
        Entry as GrpcEntry, Feed as GrpcFeed, Feeds as GrpcFeeds, GetFeedRequest,
        Image as GrpcImage, Link as GrpcLink, ListEntriesRequest, ListFeedsRequest,
        MediaContent as GrpcMediaContent, MediaObject as GrpcMediaObject, RecordPlaybackRequest,
        RecordPlaybackResponse,
    },
};

/// Build the server's TLS configuration.
///
/// Mirrors commit-store's `build_tls_config`, including its refusals: a client
/// CA without a server identity is a hard error rather than a silent downgrade
/// to plaintext, because an operator who asked for client certificates must
/// never be quietly served without them. This service has no authorization of
/// its own, so with neither set, anything that reaches the port owns the
/// library -- hence the warning.
pub fn build_tls_config(
    identity: Option<(String, String)>,
    client_ca: Option<String>,
) -> Result<Option<ServerTlsConfig>> {
    match (identity, client_ca) {
        (Some((cert, key)), client_ca) => {
            let mut cfg =
                ServerTlsConfig::new().identity(tonic::transport::Identity::from_pem(cert, key));
            if let Some(ca) = client_ca {
                cfg = cfg.client_ca_root(tonic::transport::Certificate::from_pem(ca));
                log::info!("TLS enabled, and client certificates are required (mutual TLS).");
            } else {
                log::info!("TLS enabled. Any client that can reach the port is served.");
            }
            Ok(Some(cfg))
        }
        (None, Some(_)) => anyhow::bail!(
            "a client CA is configured but the server certificate and key are not: mutual TLS \
             requires the server to present its own certificate. Configure CASTS_TLS_CERT_PATH \
             and CASTS_TLS_KEY_PATH, or remove the client CA."
        ),
        (None, None) => {
            log::warn!(
                "TLS is disabled and no client certificate is required; \
                 every client that can reach this port has full access to the library"
            );
            Ok(None)
        }
    }
}

pub struct CastsImpl {
    backing_store: Arc<Mutex<Grpc2Store>>,
}

impl CastsImpl {
    /// Serve the `Casts` API from the given backing commit store.
    #[must_use]
    pub fn new(backing_store: Grpc2Store) -> Self {
        CastsImpl {
            backing_store: Arc::new(Mutex::new(backing_store)),
        }
    }
}

/// One page of commits per round trip; the server clamps to its own
/// `MAX_PAGE_SIZE` anyway, so this only bounds client-side memory.
const HYDRATE_PAGE_SIZE: i64 = 1000;

async fn hydrate_aggregate(
    client: &mut dyn DispatchingAsyncReadWriteStore,
    stream_id: &str,
) -> Result<FeedAggregate, Status> {
    let stream_id: Uuid = Uuid::parse_str(stream_id)
        .map_err(|err| Status::invalid_argument(format!("stream_id is not a UUID: {err}")))?;
    let mut aggregate = FeedAggregate::new();
    let mut cursor = Some(ScanStreamCursor {
        stream_id,
        min_version: 0,
        max_version: None,
        reverse: false,
    });
    while let Some(current) = cursor {
        let (commits, next) = client
            .get_stream_range(current, HYDRATE_PAGE_SIZE)
            .await
            .map_err(|err| Status::unavailable(format!("commit store read failed: {err}")))?;
        for commit in commits {
            let events: Vec<FeedAggregateEvent> =
                serde_json::from_value(commit.events).map_err(|err| {
                    Status::internal(format!("stored events failed to deserialize: {err}"))
                })?;
            aggregate = aggregate.apply_many(events);
        }
        cursor = next;
    }
    Ok(aggregate)
}

#[tonic::async_trait]
impl crate::grpc_interface::casts_server::Casts for CastsImpl {
    async fn list_feeds(
        &self,
        request: Request<ListFeedsRequest>,
    ) -> Result<Response<GrpcFeeds>, Status> {
        info!("list_feeds");
        let mut lock = self.backing_store.lock().await;

        let aggregate = hydrate_aggregate(lock.store(), &request.get_ref().stream_id).await?;
        let feed_result = aggregate_to_feeds(aggregate);

        Ok(Response::new(feed_result))
    }

    async fn list_entries(
        &self,
        request: Request<ListEntriesRequest>,
    ) -> Result<Response<GrpcEntries>, Status> {
        info!("list_entries");
        let mut lock = self.backing_store.lock().await;

        let aggregate = hydrate_aggregate(lock.store(), &request.get_ref().stream_id).await?;
        let playtimes = &aggregate.playtime_by_feed_and_guid;
        let entries: Vec<GrpcEntry> = aggregate
            .feeds_by_feed_id
            .iter()
            .flat_map(|(feed_id, feed)| {
                feed.entries
                    .iter()
                    .map(|entry| entry_to_grpc_entry(entry.clone(), *feed_id, playtimes))
            })
            .collect();

        Ok(Response::new(GrpcEntries { entries }))
    }

    async fn get_feed(
        &self,
        request: Request<GetFeedRequest>,
    ) -> Result<Response<GrpcFeed>, Status> {
        info!("get_feed");
        let feed_id: Uuid = Uuid::parse_str(&request.get_ref().feed_id)
            .map_err(|err| Status::invalid_argument(format!("feed_id is not a UUID: {err}")))?;
        let mut lock = self.backing_store.lock().await;

        let aggregate = hydrate_aggregate(lock.store(), &request.get_ref().stream_id).await?;
        let feed = aggregate
            .feeds_by_feed_id
            .get(&feed_id)
            .ok_or_else(|| Status::not_found(format!("no feed with id {feed_id}")))?;

        Ok(Response::new(feed_to_grpc_feed(
            feed.clone(),
            feed_id,
            &aggregate.playtime_by_feed_and_guid,
        )))
    }

    async fn add_feed(
        &self,
        request: Request<AddFeedRequest>,
    ) -> Result<Response<AddFeedResponse>, Status> {
        info!("add_feed");
        let req = request.get_ref();
        let stream_id: Uuid = Uuid::parse_str(&req.stream_id)
            .map_err(|err| Status::invalid_argument(format!("stream_id is not a UUID: {err}")))?;
        let url = url::Url::parse(&req.url)
            .map_err(|err| Status::invalid_argument(format!("url is not a valid URL: {err}")))?;

        // Fetch before taking the store lock; the fetch is the slow part and
        // needs nothing from the store.
        let feed = immutable_feed::fetch_feed(&url)
            .await
            .map_err(|err| Status::failed_precondition(format!("could not fetch feed: {err}")))?;

        let mut lock = self.backing_store.lock().await;
        let store = lock.store();
        let aggregate = hydrate_aggregate(store, &req.stream_id).await?;

        let feed_id = Uuid::new_v4();
        let command = FeedAggregateCommand::AddFeed {
            stream_id,
            feed_id,
            url,
            feed,
        };
        let events = command
            .async_apply(&aggregate)
            .await
            .map_err(|err| match err {
                FeedAggregateError::DuplicateFeedError => Status::already_exists(err.to_string()),
                _ => Status::failed_precondition(err.to_string()),
            })?;

        let events_count = events.len() as i64;
        let commit_attempt = CommitAttempt {
            stream_metadata: StreamMetadata {
                stream_id,
                stream_version: aggregate.version + 1,
            },
            commit_metadata: CommitMetadata {
                commit_id: Uuid::new_v4(),
                commit_sequence: aggregate.version + 1,
                commit_timestamp: Utc::now(),
            },
            metadata: serde_json::to_value(&command)
                .map_err(|err| Status::internal(format!("could not serialize command: {err}")))?,
            events: serde_json::to_value(events)
                .map_err(|err| Status::internal(format!("could not serialize events: {err}")))?,
            events_count,
        };
        let committed = store
            .commit(&commit_attempt)
            .await
            .map_err(|err| match err {
                StoreError::DuplicateWriteError(conflict) => {
                    Status::aborted(format!("conflicting concurrent write, retry: {conflict}"))
                }
                other => Status::unavailable(format!("commit store write failed: {other}")),
            })?;

        Ok(Response::new(AddFeedResponse {
            feed_id: feed_id.to_string(),
            commit_id: committed.commit_metadata.commit_id.to_string(),
        }))
    }

    async fn record_playback(
        &self,
        request: Request<RecordPlaybackRequest>,
    ) -> Result<Response<RecordPlaybackResponse>, Status> {
        info!("record_playback");
        let req = request.get_ref();
        let stream_id: Uuid = Uuid::parse_str(&req.stream_id)
            .map_err(|err| Status::invalid_argument(format!("stream_id is not a UUID: {err}")))?;
        let feed_id: Uuid = Uuid::parse_str(&req.feed_id)
            .map_err(|err| Status::invalid_argument(format!("feed_id is not a UUID: {err}")))?;

        let mut lock = self.backing_store.lock().await;
        let store = lock.store();
        let aggregate = hydrate_aggregate(store, &req.stream_id).await?;

        let command = FeedAggregateCommand::MarkEntryListened {
            stream_id,
            feed_id,
            entry_guid: req.entry_guid.clone(),
            elapsed_seconds: req.elapsed_seconds,
            elapsed_nanos: req.elapsed_nanos,
        };
        let events = command
            .async_apply(&aggregate)
            .await
            .map_err(|err| match err {
                FeedAggregateError::UnknownFeedError | FeedAggregateError::UnknownEntryError => {
                    Status::not_found(err.to_string())
                }
                _ => Status::failed_precondition(err.to_string()),
            })?;
        if events.is_empty() {
            // Position unchanged; nothing to commit.
            return Ok(Response::new(RecordPlaybackResponse {
                commit_id: String::new(),
            }));
        }

        let events_count = events.len() as i64;
        let commit_attempt = CommitAttempt {
            stream_metadata: StreamMetadata {
                stream_id,
                stream_version: aggregate.version + 1,
            },
            commit_metadata: CommitMetadata {
                commit_id: Uuid::new_v4(),
                commit_sequence: aggregate.version + 1,
                commit_timestamp: Utc::now(),
            },
            metadata: serde_json::to_value(&command)
                .map_err(|err| Status::internal(format!("could not serialize command: {err}")))?,
            events: serde_json::to_value(events)
                .map_err(|err| Status::internal(format!("could not serialize events: {err}")))?,
            events_count,
        };
        let committed = store
            .commit(&commit_attempt)
            .await
            .map_err(|err| match err {
                StoreError::DuplicateWriteError(conflict) => {
                    Status::aborted(format!("conflicting concurrent write, retry: {conflict}"))
                }
                other => Status::unavailable(format!("commit store write failed: {other}")),
            })?;

        Ok(Response::new(RecordPlaybackResponse {
            commit_id: committed.commit_metadata.commit_id.to_string(),
        }))
    }
}

// Conversion stuff

fn media_content_to_grpc_media_content(media_content: MediaContent) -> GrpcMediaContent {
    GrpcMediaContent {
        url: media_content.url.map(|s| s.to_string()).unwrap_or_default(),
        content_type: media_content.content_type.unwrap_or_default(),
        width: media_content.width.map(|i| i as i32).unwrap_or_default(),
        height: media_content.height.map(|i| i as i32).unwrap_or_default(),
        size: media_content.size.map(|i| i as i32).unwrap_or_default(),
    }
}

fn media_object_to_grpc_media_object(media_object: MediaObject) -> GrpcMediaObject {
    GrpcMediaObject {
        title: media_object.title.unwrap_or_default(),
        content: media_object
            .content
            .into_iter()
            .map(media_content_to_grpc_media_content)
            .collect(),
        duration_secs: media_object
            .duration
            .map(|i| i.0 as i32)
            .unwrap_or_default(),
        duration_nsecs: media_object
            .duration
            .map(|i| i.1 as i32)
            .unwrap_or_default(),
        description: media_object.description.unwrap_or_default(),
    }
}

fn link_to_grpc_link(link: Link) -> GrpcLink {
    GrpcLink {
        title: link.title.unwrap_or_default(),
        href: link.href,
        href_lang: link.href_lang.unwrap_or_default(),
        length: link.length.map(|i| i as i32).unwrap_or_default(),
        media_type: link.media_type.unwrap_or_default(),
        rel: link.rel.unwrap_or_default(),
    }
}

fn image_to_grpc_image(image: Image) -> GrpcImage {
    GrpcImage {
        title: image.title.unwrap_or_default(),
        url: image.raw_url,
        link: image.link.map(link_to_grpc_link),
        width: image.width.map(|w| w as i32).unwrap_or_default(),
        height: image.height.map(|h| h as i32).unwrap_or_default(),
        description: image.description.unwrap_or_default(),
    }
}

fn content_to_grpc_content(content: Content) -> GrpcContent {
    GrpcContent {
        body: content.body.unwrap_or_default(),
        content_type: content.content_type,
        length: content.length.map(|i| i as i32).unwrap_or_default(),
        src: content.src.map(link_to_grpc_link),
    }
}

type PlaytimeByFeedAndGuid = HashMap<(String, String), (i64, i32)>;

fn entry_to_grpc_entry(
    entry: Entry,
    feed_id: Uuid,
    playtimes: &PlaytimeByFeedAndGuid,
) -> GrpcEntry {
    let (elapsed_seconds, elapsed_nanos) = playtimes
        .get(&(feed_id.to_string(), entry.guid.clone()))
        .copied()
        .unwrap_or((0, 0));
    GrpcEntry {
        guid: entry.guid,
        title: entry.title.unwrap_or_default(),
        summary: entry.summary.unwrap_or_default(),
        links: entry.links.into_iter().map(link_to_grpc_link).collect(),
        media: entry
            .media
            .into_iter()
            .map(media_object_to_grpc_media_object)
            .collect(),
        published: entry.published_at.map(domain_to_grpc_ts),
        content: entry.content.map(content_to_grpc_content),
        elapsed_seconds,
        elapsed_nanos,
    }
}

fn domain_to_grpc_ts(datetime: DateTime<Utc>) -> GrpcTimestamp {
    let seconds = datetime.timestamp();
    let onanos_i64 = datetime.timestamp_nanos_opt().expect("cannot fetch nanos");
    let nanos_i64 = onanos_i64 - seconds * 1_000_000_000;
    let nanos = i32::try_from(nanos_i64).expect("nanos is too large to downcast to i32");
    GrpcTimestamp { seconds, nanos }
}

fn feed_to_grpc_feed(feed: Feed, feed_id: Uuid, playtimes: &PlaytimeByFeedAndGuid) -> GrpcFeed {
    let mut entries: Vec<GrpcEntry> = feed
        .entries
        .into_iter()
        .map(|entry| entry_to_grpc_entry(entry, feed_id, playtimes))
        .collect();
    entries.sort_by(|a, b| match (&a.published, &b.published) {
        (Some(a_ts), Some(b_ts)) => b_ts.seconds.cmp(&a_ts.seconds),
        (None, Some(_)) => std::cmp::Ordering::Less,
        (Some(_), None) => std::cmp::Ordering::Greater,
        (None, None) => std::cmp::Ordering::Equal,
    });
    GrpcFeed {
        feed_id: feed_id.to_string(),
        title: feed.metadata.title,
        authors: feed.metadata.authors,
        description: feed.metadata.description.unwrap_or_default(),
        logo: feed.metadata.logo.map(image_to_grpc_image),
        entries,
    }
}

fn aggregate_to_feeds(aggregate: FeedAggregate) -> GrpcFeeds {
    // The aggregate's key IS the feed id; inventing one per response gave
    // clients a different id for the same feed on every call.
    let playtimes = &aggregate.playtime_by_feed_and_guid;
    let feeds: Vec<GrpcFeed> = aggregate
        .feeds_by_feed_id
        .iter()
        .map(|(feed_id, feed)| feed_to_grpc_feed(feed.clone(), *feed_id, playtimes))
        .collect();
    GrpcFeeds { feeds }
}
