//! End-to-end tests for the `Casts` gRPC service.
//!
//! A real `CastsServer` and a real generated client talk over
//! `tokio::io::duplex`: no port to collide on, no sleep waiting for a
//! listener, nothing left behind if a test fails. The backing commit store is
//! an in-memory SQLite instance, so these exercise the whole path -- wire
//! codec, service logic, aggregate hydration, and commit writes -- without
//! any external process.
#![cfg(feature = "grpc")]

use casts::events::FeedAggregateEvent;
use casts::grpc::Grpc2Store;
use casts::grpc_interface::casts_client::CastsClient;
use casts::grpc_interface::casts_server::CastsServer;
use casts::grpc_interface::{
    AddFeedRequest, GetFeedRequest, ListEntriesRequest, ListFeedsRequest, RecordPlaybackRequest,
};
use casts::service::CastsImpl;
use chrono::Utc;
use commit_store::commit::{CommitMetadata, StreamMetadata};
use commit_store::dispatch::DispatchedStore;
use commit_store::sqlite::SqliteStore;
use commit_store::store::Store;
use commit_store::CommitAttempt;
use immutable_feed::{Entry, Feed, FeedMetadata};
use tonic::transport::Channel;
use tonic::Code;
use url::Url;
use uuid::Uuid;

const FEED_URL: &str = "http://example.com/examplecast.xml";

fn test_entry(guid: &str, title: &str) -> Entry {
    Entry {
        guid: String::from(guid),
        url: Url::parse("http://example.com/episode.mp3").unwrap(),
        title: Some(String::from(title)),
        summary: None,
        updated_at: None,
        published_at: None,
        content: None,
        links: vec![],
        media: vec![],
    }
}

fn test_feed(title: &str, entries: Vec<Entry>) -> Feed {
    let url = Url::parse(FEED_URL).unwrap();
    Feed {
        url: Some(url.clone()),
        raw_url: url.to_string(),
        metadata: FeedMetadata {
            title: String::from(title),
            authors: vec![String::from("A Host")],
            logo: None,
            description: Some(String::from("A test feed.")),
        },
        entries,
    }
}

/// A service backed by a fresh in-memory store, plus a client wired to it
/// over an in-process duplex stream. Returns the seeded stream and feed ids.
async fn harness(entries: Vec<Entry>) -> (CastsClient<Channel>, Uuid, Uuid) {
    let mut store = DispatchedStore::new(SqliteStore::with_new_in_memory_connection());
    let stream_id = Uuid::new_v4();
    let feed_id = Uuid::new_v4();

    // Seed one feed by committing the event a real AddFeed would have written.
    let event = FeedAggregateEvent::Added {
        stream_id,
        feed_id,
        url: Url::parse(FEED_URL).unwrap(),
        metadata: test_feed("Examplecast", vec![]).metadata,
        entries,
        do_not_archive: None,
    };
    store
        .commit(&CommitAttempt {
            stream_metadata: StreamMetadata {
                stream_id,
                stream_version: 1,
            },
            commit_metadata: CommitMetadata {
                commit_id: Uuid::new_v4(),
                commit_timestamp: Utc::now(),
                commit_sequence: 1,
            },
            metadata: serde_json::Value::Null,
            events: serde_json::to_value(vec![event]).unwrap(),
            events_count: 1,
        })
        .await
        .expect("seed commit");

    let service = CastsServer::new(CastsImpl::new(Grpc2Store::Sqlite(store)));
    let (client_io, server_io) = tokio::io::duplex(1024 * 1024);

    let _server = tokio::spawn(async move {
        tonic::transport::Server::builder()
            .add_service(service)
            .serve_with_incoming(tokio_stream::once(Ok::<_, std::io::Error>(server_io)))
            .await
            .expect("in-process server");
    });

    // The URI is required by tonic but never dialled: `connect_with_connector`
    // hands back our duplex stream regardless of what is requested.
    let mut client_io = Some(client_io);
    let channel = tonic::transport::Endpoint::try_from("http://[::]:50051")
        .expect("static endpoint")
        .connect_with_connector(tower::service_fn(move |_| {
            let io = client_io.take().expect("connector called more than once");
            async move { Ok::<_, std::io::Error>(io) }
        }))
        .await
        .expect("connect over duplex");

    (CastsClient::new(channel), stream_id, feed_id)
}

#[tokio::test]
async fn list_feeds_returns_the_aggregate_feed_id_and_is_stable() {
    let (mut client, stream_id, feed_id) = harness(vec![]).await;
    let request = || ListFeedsRequest {
        stream_id: stream_id.to_string(),
    };

    let first = client.list_feeds(request()).await.unwrap().into_inner();
    assert_eq!(first.feeds.len(), 1);
    assert_eq!(first.feeds[0].feed_id, feed_id.to_string());
    assert_eq!(first.feeds[0].title, "Examplecast");
    assert_eq!(first.feeds[0].authors, vec![String::from("A Host")]);

    // The id used to be a fresh Uuid::new_v4() per response, so a client saw
    // a different feed on every call.
    let second = client.list_feeds(request()).await.unwrap().into_inner();
    assert_eq!(second.feeds[0].feed_id, first.feeds[0].feed_id);
}

#[tokio::test]
async fn list_entries_returns_every_entry_in_the_stream() {
    let entries = vec![
        test_entry("guid-1", "Episode 1"),
        test_entry("guid-2", "Episode 2"),
    ];
    let (mut client, stream_id, _feed_id) = harness(entries).await;

    let listed = client
        .list_entries(ListEntriesRequest {
            stream_id: stream_id.to_string(),
        })
        .await
        .unwrap()
        .into_inner();

    let mut guids: Vec<String> = listed.entries.into_iter().map(|e| e.guid).collect();
    guids.sort();
    assert_eq!(guids, vec!["guid-1", "guid-2"]);
}

#[tokio::test]
async fn get_feed_returns_the_named_feed_and_404s_otherwise() {
    let (mut client, stream_id, feed_id) = harness(vec![test_entry("guid-1", "Episode 1")]).await;

    let feed = client
        .get_feed(GetFeedRequest {
            feed_id: feed_id.to_string(),
            stream_id: stream_id.to_string(),
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(feed.feed_id, feed_id.to_string());
    assert_eq!(feed.entries.len(), 1);

    let missing = client
        .get_feed(GetFeedRequest {
            feed_id: Uuid::new_v4().to_string(),
            stream_id: stream_id.to_string(),
        })
        .await
        .unwrap_err();
    assert_eq!(missing.code(), Code::NotFound);
}

#[tokio::test]
async fn record_playback_is_visible_to_the_next_read() {
    let (mut client, stream_id, feed_id) = harness(vec![test_entry("guid-1", "Episode 1")]).await;
    let playback = |seconds| RecordPlaybackRequest {
        stream_id: stream_id.to_string(),
        feed_id: feed_id.to_string(),
        entry_guid: String::from("guid-1"),
        elapsed_seconds: seconds,
        elapsed_nanos: 0,
    };

    let recorded = client
        .record_playback(playback(90))
        .await
        .unwrap()
        .into_inner();
    assert!(
        !recorded.commit_id.is_empty(),
        "a new position must produce a commit"
    );

    let listed = client
        .list_entries(ListEntriesRequest {
            stream_id: stream_id.to_string(),
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(listed.entries[0].elapsed_seconds, 90);

    // The same position again is not a change: no second commit.
    let repeat = client
        .record_playback(playback(90))
        .await
        .unwrap()
        .into_inner();
    assert!(
        repeat.commit_id.is_empty(),
        "an unchanged position must not commit"
    );

    // A later position wins.
    let _ = client.record_playback(playback(120)).await.unwrap();
    let listed = client
        .list_entries(ListEntriesRequest {
            stream_id: stream_id.to_string(),
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(listed.entries[0].elapsed_seconds, 120);
}

#[tokio::test]
async fn record_playback_rejects_unknown_feeds_and_entries() {
    let (mut client, stream_id, feed_id) = harness(vec![test_entry("guid-1", "Episode 1")]).await;

    let unknown_entry = client
        .record_playback(RecordPlaybackRequest {
            stream_id: stream_id.to_string(),
            feed_id: feed_id.to_string(),
            entry_guid: String::from("guid-does-not-exist"),
            elapsed_seconds: 5,
            elapsed_nanos: 0,
        })
        .await
        .unwrap_err();
    assert_eq!(unknown_entry.code(), Code::NotFound);

    let unknown_feed = client
        .record_playback(RecordPlaybackRequest {
            stream_id: stream_id.to_string(),
            feed_id: Uuid::new_v4().to_string(),
            entry_guid: String::from("guid-1"),
            elapsed_seconds: 5,
            elapsed_nanos: 0,
        })
        .await
        .unwrap_err();
    assert_eq!(unknown_feed.code(), Code::NotFound);
}

#[tokio::test]
async fn malformed_ids_are_invalid_argument_and_leave_the_service_usable() {
    let (mut client, stream_id, _feed_id) = harness(vec![]).await;

    let bad_stream = client
        .list_feeds(ListFeedsRequest {
            stream_id: String::from("not-a-uuid"),
        })
        .await
        .unwrap_err();
    assert_eq!(bad_stream.code(), Code::InvalidArgument);

    let bad_url = client
        .add_feed(AddFeedRequest {
            stream_id: stream_id.to_string(),
            url: String::from("not-a-url"),
        })
        .await
        .unwrap_err();
    assert_eq!(bad_url.code(), Code::InvalidArgument);

    // A malformed request must not poison the connection.
    let listed = client
        .list_feeds(ListFeedsRequest {
            stream_id: stream_id.to_string(),
        })
        .await
        .unwrap()
        .into_inner();
    assert_eq!(listed.feeds.len(), 1);
}
