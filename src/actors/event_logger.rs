use log::info;

use actix::prelude::*;
use crate::actors::event_bus::BusMessage;

/// The event bus actor
#[derive(Default)]
pub struct EventLogger {}

impl Handler<BusMessage> for EventLogger {
  type Result = ();

  fn handle(&mut self, msg: BusMessage, _ctx: &mut Context<Self>) -> Self::Result {
    match msg {
      BusMessage(feed_event) => info!("{:#?}", feed_event),
    };
  }
}
impl Actor for EventLogger {
  type Context = Context<Self>;
}