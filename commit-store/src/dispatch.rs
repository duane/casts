// Copyright (c) 2023 Duane R. Bailey. All rights reserved.

use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};
use tokio::sync::mpsc::{channel, Receiver, Sender};

use async_trait::async_trait;
use uuid::Uuid;

use crate::{
    commit::{Commit, CommitAttempt},
    AsyncReadWriteStore, CommitStream, DispatchingAsyncReadWriteStore, StoreError,
};

#[async_trait]
pub trait Dispatcher {
    async fn dispatch(&self, commit: Commit);
}

#[derive(Clone, Debug)]
pub struct LocalSubscriber {
    tx: Sender<Commit>,
    // commit_number: i64,
}

#[derive(Clone, Debug)]
pub struct LocalDispatcher {
    pub global_subscriptions: Arc<Mutex<Vec<LocalSubscriber>>>,
    stream_subscriptions: Arc<Mutex<HashMap<Uuid, Vec<LocalSubscriber>>>>,
}

impl LocalDispatcher {
    pub fn new() -> LocalDispatcher {
        LocalDispatcher {
            global_subscriptions: Arc::new(Mutex::new(Vec::new())),
            stream_subscriptions: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn subscribe_all_commits(&self) -> Receiver<Commit> {
        let (tx, rx) = channel(100);
        {
            let mut global_subscriptions = self.global_subscriptions.lock().unwrap();
            (&mut *global_subscriptions).push(LocalSubscriber { tx });
        }
        rx
    }

    pub fn subscribe_stream(&self, stream_id: &Uuid) -> Receiver<Commit> {
        let (tx, rx) = channel(100);
        let mut subscriptions = self.stream_subscriptions.lock().unwrap();
        match subscriptions.get_mut(&stream_id) {
            Some(list) => {
                list.push(LocalSubscriber {
                    tx,
                    // commit_number: 0,
                });
            }
            None => {
                _ = subscriptions.insert(
                    stream_id.clone(),
                    vec![LocalSubscriber {
                        tx,
                        // commit_number: 0,
                    }],
                );
            }
        }
        rx
    }
}

#[derive(Clone, Debug)]
pub struct DispatchedStore<S: AsyncReadWriteStore + Send> {
    store: S,
    pub dispatcher: LocalDispatcher,
}

#[async_trait]
impl<S: AsyncReadWriteStore + Send> DispatchingAsyncReadWriteStore for DispatchedStore<S> {
    async fn subscribe_all_commits(&mut self) -> CommitStream {
        CommitStream::ReceiverBacked(self.dispatcher.subscribe_all_commits().into())
    }

    async fn subscribe_stream(&mut self, stream_id: &Uuid) -> CommitStream {
        CommitStream::ReceiverBacked(self.dispatcher.subscribe_stream(stream_id).into())
    }
}

impl<S: AsyncReadWriteStore + Send> DispatchedStore<S> {
    pub fn new(s: S) -> DispatchedStore<S> {
        DispatchedStore {
            store: s,
            dispatcher: LocalDispatcher::new(),
        }
    }

    pub fn subscribe_all_commits_async(&self) -> Receiver<Commit> {
        self.dispatcher.subscribe_all_commits()
    }

    pub fn subscribe_stream_async(&self, stream_id: String) -> Receiver<Commit> {
        self.dispatcher
            .subscribe_stream(&Uuid::parse_str(&stream_id).unwrap())
    }
}

#[async_trait]
impl<S: AsyncReadWriteStore + Send> AsyncReadWriteStore for DispatchedStore<S> {
    async fn commit(
        &mut self,
        commit_attempt: &CommitAttempt,
    ) -> Result<Commit, Box<dyn StoreError>> {
        let commit = self.store.commit(&commit_attempt).await?;
        self.dispatcher.dispatch(commit.clone()).await;
        Ok(commit)
    }

    async fn get_stream_range(
        &mut self,
        stream_id: Uuid,
        min_version: i64,
        max_version: i64,
    ) -> Result<Vec<Commit>, Box<dyn StoreError>> {
        self.store
            .get_stream_range(stream_id, min_version, max_version)
            .await
    }

    async fn get_range(
        &mut self,
        min_commit_number: i64,
        max_commit_number: i64,
    ) -> Result<Vec<Commit>, Box<dyn StoreError>> {
        self.store
            .get_range(min_commit_number, max_commit_number)
            .await
    }

    async fn get_commit(&mut self, commit_id: &Uuid) -> Result<Commit, Box<dyn StoreError>> {
        self.store.get_commit(commit_id).await
    }
}

#[async_trait]
impl Dispatcher for LocalDispatcher {
    async fn dispatch(&self, commit: Commit) {
        let global_subscribers = self.global_subscriptions.lock().unwrap().clone();
        for subscriber in global_subscribers {
            subscriber.tx.send(commit.clone()).await.unwrap();
        }


        let stream_subscribers = {
            if let Some(subscribers) = self
                .stream_subscriptions
                .lock()
                .unwrap()
                .get(&commit.stream_id)
            {
                subscribers.clone()
            } else {
                return;
            }
        };

        for subscriber in stream_subscribers {
            let commit = commit.clone();
            subscriber.tx.send(commit).await.unwrap();
        }
    }
}
