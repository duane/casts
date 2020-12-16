extern crate rusqlite;
extern crate anyhow;
extern crate quick_xml;
extern crate base64;
extern crate bytesize;
extern crate casts;
extern crate chrono;
extern crate clap;
extern crate colored;
extern crate futures;
extern crate html2text;
extern crate htmlescape;
extern crate magic;
extern crate notify_rust;
extern crate regex;
extern crate reqwest;
extern crate tempfile;
extern crate url;
extern crate uuid;
extern crate xdg;
extern crate hyper;

use chrono::{DateTime, Duration, Utc};
use std::thread::sleep;

use actix::prelude::*;

#[derive(Message)]
#[rtype(result = "()")]
struct PollFeeds();


impl Handler<PollFeeds> for Clock {
    type Result = ();

    fn handle(&mut self, _msg: PollFeeds, _ctx: &mut Context<Self>) -> Self::Result {
        let ping_duration: Duration = Duration::seconds(1);

        loop {
            let elapsed = Utc::now().signed_duration_since(self.scheduled_clock_trigger);
            self.scheduled_clock_trigger = self.scheduled_clock_trigger.checked_add_signed(ping_duration.checked_sub(&elapsed).unwrap()).unwrap();
            println!("Triggered; next trigger @ {:?}", self.scheduled_clock_trigger);
            sleep(self.scheduled_clock_trigger.signed_duration_since(Utc::now()).to_std().unwrap());
        }
    }
}

struct Clock {
    pub scheduled_clock_trigger: DateTime<Utc>
}

impl Actor for Clock {
    type Context = Context<Self>;
}

#[actix_rt::main] 
async fn main() {
    // start new actor
    let addr = Clock { scheduled_clock_trigger: Utc::now() }.start();

    // send message and get future for result
    let res = addr.send(PollFeeds()).await;
    res.unwrap();

    // handle() returns tokio handle
    //println!("RESULT: {}", res.unwrap() == 20);

    // stop system and exit
    System::current().stop();
}
