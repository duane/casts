use actix::prelude::*;
use crate::events::FeedEvent;

/// This message is used to deliver a message from the bus.
#[derive(Message)]
#[rtype(result = "()")]
pub struct BusMessage(pub FeedEvent);

/// This message is used to add a subscriber.
#[derive(Message)]
#[rtype(result = "()")]
pub struct Subscribe(pub Recipient<BusMessage>);

/// This message is used to publish events.
#[derive(Message)]
#[rtype(result = "anyhow::Result<()>")]
pub struct Commit(pub FeedEvent);

/// The event bus actor
#[derive(Default)]
pub struct EventBus {
  subscribers: Vec<Recipient<BusMessage>>
}

impl Handler<Subscribe> for EventBus {
  type Result = ();

  fn handle(&mut self, msg: Subscribe, _ctx: &mut Context<Self>) -> Self::Result {
    match msg {
      Subscribe(recipient) => self.subscribers.push(recipient)
    }
  }
}

impl Handler<Commit> for EventBus {
  type Result = anyhow::Result<()>;

  fn handle(&mut self, msg: Commit, _ctx: &mut Context<Self>) -> Self::Result {
    match msg {
      Commit(event) => {
        for subscriber in self.subscribers.iter() {
          subscriber.try_send(BusMessage(event.clone()))?;
        }
        Ok(())
      }
    }
  }
}

impl Actor for EventBus {
  type Context = Context<Self>;
}