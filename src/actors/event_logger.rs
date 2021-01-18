use log::{log, Level};

use actix::prelude::*;
use crate::actors::event_bus::BusMessage;

/// The event bus actor
pub struct EventLogger {
  level: Level
}

impl Default for EventLogger {
  fn default() -> Self {
    EventLogger {
      level: Level::Debug
    }
  }
}

impl Handler<BusMessage> for EventLogger {
  type Result = ();

  fn handle(&mut self, msg: BusMessage, _ctx: &mut Context<Self>) -> Self::Result {
    match msg {
      BusMessage(feed_event) => log!(self.level, "{:#?}", feed_event),
    };
  }
}
impl Actor for EventLogger {
  type Context = Context<Self>;
}