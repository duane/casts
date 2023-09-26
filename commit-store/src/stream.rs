use crate::commit::Commit;
use crate::grpc::grpc_to_domain_commit;
use crate::grpc_interface;
use std::fmt;
use std::fmt::Display;
use std::pin::Pin;
use tokio_stream::StreamExt;
use tokio_stream::{wrappers::ReceiverStream, Stream};
use tonic::Streaming;

/// Wraps
pub enum CommitStream {
    ReceiverBacked(ReceiverStream<Commit>),
    GrpcBacked(Streaming<grpc_interface::Commit>),
}

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum CommitStreamError {
    GrpcError,
}

impl Display for CommitStreamError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CommitStreamError::GrpcError")
    }
}

impl Stream for CommitStream {
    type Item = Result<Commit, CommitStreamError>;

    fn poll_next(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Option<Self::Item>> {
        match self.get_mut() {
            &mut CommitStream::ReceiverBacked(ref mut rx) => {
                Pin::new(rx).poll_next(cx).map(|r| r.map(Ok))
            }
            &mut CommitStream::GrpcBacked(ref mut grpc) => {
                let mut mapped_stream = grpc.map(|r| match r {
                    Ok(commit) => Ok(grpc_to_domain_commit(&commit)),
                    Err(_status) => Err(CommitStreamError::GrpcError),
                });
                Pin::new(&mut mapped_stream).poll_next(cx)
            }
        }
    }
}
