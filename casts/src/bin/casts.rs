use anyhow::Result;
use casts::{
    aggregate::FeedAggregate,
    applicable::EventApplicable,
    commands::{Command, FeedAggregateCommand},
    config::{
        ClientConfig, ClientConfigBuilder, ServerConfig, ServerConfigBuilder,
        DEFAULT_COMMIT_STORE_URL,
    },
    events::FeedAggregateEvent,
};
use chrono::Utc;
use commit_store::{
    commit::{CommitMetadata, StreamMetadata},
    domain_to_grpc_commit_attempt,
    grpc_interface::{commit_store_client::CommitStoreClient, Commits, StreamCursor},
    grpc_to_domain_commit, CommitAttempt,
};
use immutable_feed::fetch_feed;
use log::info;
use prost::Message;
use tonic::transport::Channel;
use url::Url;
use uuid::{uuid, Uuid};

use futures::StreamExt;

fn usage() {
    println!("USAGE: casts add FEED_URL");
    println!("USAGE: casts show FEED_ID");
    println!("USAGE: casts mark-listened FEED_ID ENTRY_GUID ELAPSED_SECONDS");
    std::process::exit(1);
}

async fn mark_listened(
    mut client: CommitStoreClient<tonic::transport::Channel>,
    stream_id: Uuid,
    feed_id: Uuid,
    entry_guid: String,
    elapsed_seconds: i64,
) -> Result<()> {
    let aggregate = hydrated_client(&mut client, &stream_id).await?;
    let command = FeedAggregateCommand::MarkEntryListened {
        stream_id,
        feed_id,
        entry_guid,
        elapsed_seconds,
        elapsed_nanos: 0,
    };
    let events = command.async_apply(&aggregate).await?;
    if events.is_empty() {
        println!("position unchanged; nothing committed");
        return Ok(());
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
        metadata: serde_json::to_value(&command)?,
        events: serde_json::to_value(events)?,
        events_count,
    };
    let commit_result = client
        .append_commit(domain_to_grpc_commit_attempt(&commit_attempt))
        .await?;
    let commit = grpc_to_domain_commit(commit_result.get_ref());
    println!("commit: {:?}", commit.commit_metadata.commit_id);
    Ok(())
}

const STREAM_ID: Uuid = uuid!["34c3c08d-0d55-4af4-a126-170c1380f07e"];

pub async fn get_client() -> CommitStoreClient<Channel> {
    let grpc_client: CommitStoreClient<Channel> =
        CommitStoreClient::connect("http://127.0.0.1:7392")
            .await
            .expect("Failed to connect to backend commit store");
    grpc_client
}

async fn add(
    mut client: CommitStoreClient<tonic::transport::Channel>,
    stream_id: &Uuid,
    feed_url: Url,
) -> Result<()> {
    let aggregate = hydrated_client(&mut client, stream_id).await?;
    // Fetch and parse before issuing the command; command application is pure.
    let feed = fetch_feed(&feed_url).await?;
    let add_feed_cmd: FeedAggregateCommand = FeedAggregateCommand::AddFeed {
        stream_id: STREAM_ID,
        feed_id: Uuid::new_v4(),
        url: feed_url,
        feed,
    };

    let events = add_feed_cmd.async_apply(&aggregate).await?;
    let events_count = events.len() as i64;
    let commit_attempt = CommitAttempt {
        stream_metadata: StreamMetadata {
            stream_id: STREAM_ID,
            stream_version: aggregate.version + 1,
        },
        commit_metadata: CommitMetadata {
            commit_id: Uuid::new_v4(),
            commit_sequence: aggregate.version + 1,
            commit_timestamp: chrono::Utc::now(),
        },
        metadata: serde_json::to_value(&add_feed_cmd)?,
        events: serde_json::to_value(events)?,
        events_count,
    };

    let commit_result = client
        .append_commit(domain_to_grpc_commit_attempt(&commit_attempt))
        .await?;
    let commit = grpc_to_domain_commit(commit_result.get_ref());
    println!("commit: {:?}", commit);
    Ok(())
}

async fn poll(stream_id: Uuid) -> Result<()> {
    println!("HALLO");
    let min_version: i64 = 0;
    let max_version = i64::MAX;

    let mut grpc_client: CommitStoreClient<tonic::transport::Channel> =
        CommitStoreClient::connect("http://127.0.0.1:7392")
            .await
            .expect("Failed to connect to backend commit store");

    let commits: Commits = grpc_client
        .get_stream_range(StreamCursor {
            stream_id: stream_id.to_string(),
            min_version,
            max_version,
            limit: i64::MAX,
        })
        .await
        .unwrap()
        .get_mut()
        .to_owned();

    let mut aggregate = FeedAggregate::default();
    for commit in commits.commits.iter() {
        aggregate = aggregate.apply_many(serde_json::from_slice(commit.events.as_slice()).unwrap());
    }
    for (feed_id, existing_feed) in aggregate.feeds_by_feed_id.clone().into_iter() {
        let observed_feed = fetch_feed(&existing_feed.url.clone().expect("requires url")).await?;
        let diff = existing_feed.diff(&observed_feed);
        println!("found {} changes", diff.len());
        let observed_feed_cmd = FeedAggregateCommand::ObserveFeed {
            stream_id,
            feed_id,
            feed: observed_feed,
        };
        let events = observed_feed_cmd
            .async_apply(&aggregate)
            .await
            .expect("Failed to apply feed observation");
        let events_count = events.len() as i64;
        if events_count == 0 {
            continue;
        }
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
            metadata: serde_json::to_value(observed_feed_cmd).unwrap(),
            events: serde_json::to_value(events.clone()).unwrap(),
            events_count,
        };
        let commit_result = grpc_to_domain_commit(
            grpc_client
                .append_commit(domain_to_grpc_commit_attempt(&commit_attempt))
                .await
                .unwrap()
                .get_ref(),
        );
        let committed_events: Vec<FeedAggregateEvent> =
            serde_json::from_value(commit_result.events).unwrap();
        aggregate = aggregate.apply_many(committed_events);
    }
    Ok(())
}

async fn hydrated_client(
    grpc_client: &mut CommitStoreClient<tonic::transport::Channel>,
    stream_id: &Uuid,
) -> Result<FeedAggregate> {
    let min_version = 0;
    let max_version = i64::MAX;

    let commits: Commits = grpc_client
        .get_stream_range(StreamCursor {
            stream_id: stream_id.to_string(),
            min_version,
            max_version,
            limit: i64::MAX,
        })
        .await
        .unwrap()
        .get_mut()
        .to_owned();

    let mut aggregate = FeedAggregate::default();
    for commit in commits.commits.iter() {
        aggregate = aggregate.apply_many(serde_json::from_slice(commit.events.as_slice()).unwrap());
    }
    Ok(aggregate)
}

fn client_config() -> ClientConfig {
    ClientConfigBuilder::default().build().unwrap()
}

fn server_config() -> ServerConfig {
    ServerConfigBuilder::default().build().unwrap()
}

#[tokio::main]
async fn main() -> Result<()> {
    pretty_env_logger::init();

    let stream_id = Uuid::parse_str("34c3c08d-0d55-4af4-a126-170c1380f07e")?;

    let args = std::env::args().collect::<Vec<String>>();
    info!(
        "connecting to backend server: {:?}",
        DEFAULT_COMMIT_STORE_URL
    );
    info!(
        "Connecting to backend server: {:?}",
        DEFAULT_COMMIT_STORE_URL
    );
    let grpc_client: CommitStoreClient<tonic::transport::Channel> =
        CommitStoreClient::connect(DEFAULT_COMMIT_STORE_URL)
            .await
            .expect("Failed to connect to backend commit store");

    match args[1].as_str() {
        "server-config" => {
            println!("Server config: {:?}", server_config());
        }
        "client-config" => {
            let server_config = server_config();
            println!("Server config: {:?}", server_config);
            println!("Client config: {:?}", client_config());
            panic!("don't know what to do")
        }
        "add" => {
            if std::env::args().len() != 3 {
                usage();
            }
            add(grpc_client, &stream_id, Url::parse(&args[2])?).await?
        }
        "mark-listened" => {
            if args.len() != 5 {
                usage();
            }
            mark_listened(
                grpc_client,
                stream_id,
                Uuid::parse_str(&args[2])?,
                args[3].clone(),
                args[4].parse()?,
            )
            .await?
        }
        "list-commits-events-json" => {
            let mut grpc_client: CommitStoreClient<tonic::transport::Channel> =
                CommitStoreClient::connect("http://127.0.0.1:7392")
                    .await
                    .expect("Failed to connect to backend commit store")
                    .max_encoding_message_size(8 * 1024 * 1024);

            let response_result = grpc_client
                .get_stream_range(StreamCursor {
                    stream_id: stream_id.to_string(),
                    min_version: 0,
                    max_version: i64::MAX,
                    limit: i64::MAX,
                })
                .await;

            let mut response: tonic::Response<Commits> = response_result.unwrap();
            let commits = response.get_mut().to_owned();

            println!("[");
            for commit in commits.commits.iter() {
                let domain_commit = grpc_to_domain_commit(commit);
                println!("{},", serde_json::to_string(&domain_commit.events).unwrap());
            }
            println!("]");
        }
        "list-commits" => {
            let mut grpc_client: CommitStoreClient<tonic::transport::Channel> =
                CommitStoreClient::connect("http://127.0.0.1:7392")
                    .await
                    .expect("Failed to connect to backend commit store");

            println!("Fetching commits");
            let response_result = grpc_client
                .stream_stream_range(StreamCursor {
                    stream_id: stream_id.to_string(),
                    min_version: 0,
                    max_version: i64::MAX,
                    limit: i64::MAX,
                })
                .await;

            let mut response = response_result.unwrap();
            while let Some(rpc_commit_result) = response.get_mut().next().await {
                let rpc_commit = rpc_commit_result.unwrap();
                let rpc_commit_as_msg: &dyn Message = &rpc_commit;
                let domain_commit = grpc_to_domain_commit(&rpc_commit);
                println!(
                    "{}\t{}\t{}\t{:-#}",
                    domain_commit.stream_metadata.stream_version,
                    domain_commit.commit_metadata.commit_id,
                    domain_commit.commit_metadata.commit_timestamp,
                    byte_unit::Byte::from_i64_with_unit(
                        rpc_commit_as_msg.encoded_len() as i64,
                        byte_unit::Unit::B
                    )
                    .unwrap()
                );
            }
        }
        "poll" => {
            if args.len() == 2 {
                let min_version = 0;
                let max_version = i64::MAX;

                let mut grpc_client: CommitStoreClient<tonic::transport::Channel> =
                    CommitStoreClient::connect("http://127.0.0.1:7392")
                        .await
                        .expect("Failed to connect to backend commit store")
                        // .max_encoding_message_size(8 * 1024 * 1024);
                        .max_decoding_message_size(8 * 1024 * 1024);

                let mut commit_stream_response = grpc_client
                    .stream_stream_range(StreamCursor {
                        stream_id: stream_id.to_string(),
                        min_version,
                        max_version,
                        limit: i64::MAX,
                    })
                    .await
                    .unwrap()
                    .into_inner();
                let mut aggregate = FeedAggregate::default();
                while let Some(Ok(commit)) = commit_stream_response.next().await {
                    aggregate = aggregate
                        .apply_many(serde_json::from_slice(commit.events.as_slice()).unwrap());
                }
                for (feed_id, existing_feed) in aggregate.feeds_by_feed_id.clone().into_iter() {
                    let observed_feed =
                        fetch_feed(&existing_feed.url.clone().expect("requires url")).await?;
                    let diff = existing_feed.diff(&observed_feed);
                    println!("found {} changes", diff.len());
                    let observed_feed_cmd = FeedAggregateCommand::ObserveFeed {
                        stream_id,
                        feed_id,
                        feed: observed_feed,
                    };
                    let events = observed_feed_cmd
                        .async_apply(&aggregate)
                        .await
                        .expect("Failed to apply feed observation");
                    let events_count = events.len() as i64;
                    if events_count == 0 {
                        continue;
                    }
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
                        metadata: serde_json::to_value(observed_feed_cmd).unwrap(),
                        events: serde_json::to_value(events.clone()).unwrap(),
                        events_count,
                    };
                    let commit_result = grpc_to_domain_commit(
                        grpc_client
                            .append_commit(domain_to_grpc_commit_attempt(&commit_attempt))
                            .await
                            .unwrap()
                            .get_ref(),
                    );
                    let committed_events: Vec<FeedAggregateEvent> =
                        serde_json::from_value(commit_result.events).unwrap();
                    aggregate = aggregate.apply_many(committed_events);
                }
            } else {
                let poll_arg_o = Uuid::parse_str(&args[2]).ok();

                poll(
                    poll_arg_o.expect("You should pass a valid feed id if you want a nice result"),
                )
                .await?;
            }
        }
        _ => usage(),
    }

    Ok(())
}
