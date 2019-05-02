use crate::app::AppId;
use std::collections::{HashMap, HashSet};
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::sync::mpsc;

use serde::{Deserialize, Serialize};

use rand::{thread_rng, Rng};

use shrinkwraprs::Shrinkwrap;

pub mod messages;
use messages::{Date, Header, Msg, MsgId};

pub mod events;
use events::{Event, Events};

use crate::app::events::Event as AppEvent;

pub struct Server {
    app_id: AppId,
    clock: Clock,
    sent_messages_ids: HashSet<MsgId>,
}

#[derive(Shrinkwrap, Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Clock(pub HashMap<AppId, Date>);

impl Clock {
    fn new(app_id: AppId) -> Self {
        let mut map = HashMap::new();
        map.insert(app_id.to_owned(), 0);
        Clock(map)
    }

    fn merge(&mut self, clock: &Self) {
        for (id, date) in &clock.0 {
            match self.get(id) {
                // Do not update the clock if it contains a more recent date
                Some(local_date) if local_date >= date => {}
                _ => {
                    self.0.insert(id.to_owned(), date.to_owned());
                }
            }
        }
    }
}

impl Server {
    pub fn new(app_id: AppId) -> Self {
        Server {
            app_id: app_id.clone(),
            clock: Clock::new(app_id),
            sent_messages_ids: HashSet::new(),
        }
    }

    fn get_date(&self) -> Date {
        *self.clock.get(&self.app_id).expect("missing local app_id")
    }

    fn increment_clock(&mut self) {
        let date = self.clock.0.entry(self.app_id.to_owned()).or_insert(0);
        *date += 1;
    }

    fn send_message(&mut self, msg: &Msg, output_file: &mut File) {
        if let Ok(msg_str) = msg.serialize() {
            output_file
                .write_all(format!("{}\n", msg_str).as_bytes())
                .expect("Failed to write to output file");
            log::info!(
                "sent, local date: {}, messsage: {:?}",
                self.get_date(),
                msg.content
            );
        } else {
            log::error!("Could not serialize `{}`", msg.content);
        }
    }

    fn receive_message(&mut self, msg: &mut Msg, output_file: &mut File) {
        self.clock.merge(&msg.clock);
            log::info!(
                "received, local date: {}, messsage: {:?}",
                self.get_date(),
                msg.content
            );
            msg.clock = self.clock.clone();
            self.send_message(msg, output_file);
        }
    }

pub fn run(
    mut server: Server,
    app_rx: mpsc::Receiver<Event>,
    app_tx: mpsc::Sender<AppEvent>,
    opt: crate::Opt,
) -> Result<(), Box<dyn std::error::Error>> {
    // Order matter !

    // 1 Setup event handlers
    let events = Events::new(opt.input.to_owned(), app_rx);

    // 2 Open the output pipe,
    // the program will freeze until there is someone at the other end
    let mut output_file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(opt.output.to_owned())
        .expect("failed to open output file");

    let mut rng = thread_rng();

    loop {
        // Handle events
        match events.next()? {
            // Input from a distant app
            Event::DistantInput(msg) => {
                if let Ok(mut msg) = Msg::from_str(&msg) {
                    if server.sent_messages_ids.insert(msg.id.clone()) {
                        server.increment_clock();
                        server.receive_message(&mut msg, &mut output_file);
                        app_tx.send(AppEvent::DistantMessage(msg)).unwrap();
                    }
                } else {
                    log::error!("Could not decode `{}` as a Msg", msg);
                }
            }
            Event::UserPublicMessage(message) => {
                let msg_id: MsgId = rng.gen();
                server.sent_messages_ids.insert(msg_id.clone());
                server.increment_clock();
                let msg = Msg::new(msg_id, server.app_id.clone(), Header::Public, message, server.clock.clone());
                server.send_message(&msg, &mut output_file);
            }
            Event::GetClock => {
                app_tx
                    .send(AppEvent::Clock(server.clock.clone()))
                    .expect("failed to send message to the app");
            }
            Event::Shutdown => break,
        }
    }

    Ok(())
}
