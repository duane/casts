use anyhow::Result;
use commit_store::grpc_interface::{commit_store_client::CommitStoreClient, CommitAttempt, Cursor};
use serde_json::Value;
use tokio_stream::StreamExt;
use tonic::Request;
use uuid::uuid;

#[tokio::main]
async fn main() -> Result<()> {
    assert!(std::env::args().len() == 2);
    let stream_id = uuid!("E6C1E8A3-06EB-4243-9147-0303E059ABE5");
    let mut client = CommitStoreClient::connect("http://127.0.0.1:5059").await?;

    match std::env::args()
        .nth(1)
        .expect("USAGE: grpc-client WRITE|READ|STREAM")
        .to_string()
        .to_lowercase()
        .as_str()
    {
        "write" => {
            let commit_id = uuid::Uuid::new_v4().to_string();
            let events: Value = vec!["only_event".to_owned()].into();

            let request = tonic::Request::new(CommitAttempt {
                stream_id: stream_id.to_string(),
                stream_version: 0,
                commit_id,
                commit_timestamp: None,
                commit_sequence: 0,
                metadata: b"hello".to_vec(),
                events: serde_json::to_vec(&events)?,
                events_count: 1,
            });

            let response = client.append_commit(request).await?;

            println!("RESPONSE={:?}", response);
        }
        "read" => {
            let request = Request::new(Cursor {
                stream_id: stream_id.to_string(),
                min_version: 0,
                max_version: i64::MAX,
            });
            let response = client.get_stream_range(request).await?;
            println!("RESPONSE={:?}", response);
        }
        "stream" => {
            let request = Request::new(Cursor {
                stream_id: stream_id.to_string(),,
                min_version: 0,
                max_version: i64::MAX,
            });
            let mut stream = client
                .stream_stream_range(request)
                .await
                .unwrap()
                .into_inner();
            while let Some(item) = stream.next().await {
                println!("RESPONSE={:?}", item.unwrap());
            }
        }
        _ => panic!(),
    }

    Ok(())
}
