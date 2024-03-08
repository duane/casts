use amqprs::{
    channel::{BasicPublishArguments, Channel, QueueDeclareArguments},
    connection::Connection,
    BasicProperties,
};
use async_trait::async_trait;

use crate::{dispatch::Dispatcher, Commit};

pub struct RabbitMQDispatchBuilder {
    connection: Connection,
    queue_name: String,
    channel_id: u16,
}

impl RabbitMQDispatchBuilder {
    pub fn new(
        connection: Connection,
        queue_name: String,
        channel_id: u16,
    ) -> RabbitMQDispatchBuilder {
        RabbitMQDispatchBuilder {
            connection,
            queue_name,
            channel_id,
        }
    }

    pub async fn finish(self) -> RabbitMQDispatcher {
        let channel = self
            .connection
            .open_channel(Some(self.channel_id))
            .await
            .unwrap();
        let args: QueueDeclareArguments = QueueDeclareArguments::default()
            .queue(self.queue_name.clone())
            .finish();
        let _queue = channel.queue_declare(args).await.unwrap().unwrap();
        RabbitMQDispatcher { channel }
    }
}

pub struct RabbitMQDispatcher {
    channel: Channel,
    // queue: (String, u32, u32),
}

#[async_trait]
impl Dispatcher for RabbitMQDispatcher {
    async fn dispatch(&self, commit: Commit) {
        let props: BasicProperties = Default::default();
        let content = serde_json::to_string(&commit).unwrap().as_bytes().to_vec();
        let args: BasicPublishArguments = BasicPublishArguments::default();
        self.channel
            .basic_publish(props, content, args)
            .await
            .unwrap();
    }
}
