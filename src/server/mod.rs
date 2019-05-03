use crate::app::AppId;
use std::collections::{HashMap, HashSet};
use std::collections::hash_map::Entry;
use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::sync::mpsc;

use serde::{Deserialize, Serialize};

use rand::{thread_rng, Rng};

use shrinkwraprs::Shrinkwrap;

pub mod messages;
use messages::{Date, Header::*, Msg, MsgId};

pub mod events;
use events::{Event, Events};

use crate::app::events::Event as AppEvent;

#[derive(Shrinkwrap, Clone, Serialize, Deserialize, Debug, PartialEq)]
#[shrinkwrap(mutable)]
pub struct Clock(pub HashMap<AppId, Date>);

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
pub struct Snapshot {
    pub dates : Clock,
    messages : HashMap<AppId,Vec<Msg>>,
}

pub struct Server {
    app_id: AppId,
    clock: Clock,
    sent_messages_ids: HashSet<MsgId>,
    snapshot : Snapshot,
    sent_messages : Vec<Msg>
}

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
                    self.insert(id.to_owned(), date.to_owned());
                }
            }
        }
    }
}


impl Snapshot {
    pub fn new(app_id : AppId) -> Self {
        Snapshot{
            dates : Clock::new(app_id),
            messages : HashMap::new(),
        }
    }
    pub fn add(&mut self, msg : Msg){
        // We store the snapshot sending date from each app
        match self.dates.0.entry(msg.sender_id.to_owned()) {
            Entry::Occupied(_) => panic!("Received snapshot twice from same App"),
            Entry::Vacant(v) => v.insert(msg.clock.get(&msg.sender_id)
                            .expect("Inconsistent message : missing sender date in vector clock").to_owned())
        };
        if let messages::Header::SnapshotResponse(_,messages) = msg.header {
            let mut consistent_msgs = Vec::new();
            for m in messages{
                // We ensure snapshot consistency by removing messages created
                // after snapshot sending date
                let sender_date = m.clock.get(&m.sender_id)
                    .expect("Inconsistent message : missing sender date in vector clock");
                if sender_date <= self.dates.0.entry(m.sender_id.to_owned()).or_insert_with(|| sender_date.to_owned()){
                    consistent_msgs.push(m.to_owned());
                }
            }
            self.messages.insert(msg.sender_id, consistent_msgs);
        }
    }
    pub fn serialize(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }
}

impl Server {
    pub fn new(app_id: AppId) -> Self {
        Server {
            app_id: app_id.clone(),
            clock: Clock::new(app_id.clone()),
            sent_messages_ids: HashSet::new(),
            snapshot : Snapshot::new(app_id.clone()),
            sent_messages : Vec::new(),
        }
    }

    fn get_date(&self) -> Date {
        *self.clock.get(&self.app_id).expect("missing local app_id")
    }

    fn increment_clock(&mut self) {
        let date = self.clock.entry(self.app_id.to_owned()).or_insert(0);
        *date += 1;
    }

    fn send_message(&mut self, msg: &Msg, output_file: &mut File, app_tx: &mpsc::Sender<AppEvent>) {
        if let Ok(msg_str) = msg.serialize() {
            if let Ok(_) = output_file.write_all(format!("{}\n", msg_str).as_bytes()) {
                log::info!(
                    "sent, local date: {}, messsage: {:?}",
                    self.get_date(),
                    msg.header
                );
            } else {
                app_tx
                    .send(AppEvent::ServerMessage("No one can hear you".to_owned()))
                    .unwrap();
                log::error!("Failed to write to output file");
            }
        } else {
            log::error!("Could not serialize `{:?}`", msg);
        }
    }

    fn receive_message(
        &mut self,
        msg: &mut Msg,
        output_file: &mut File,
        app_tx: &mpsc::Sender<AppEvent>,
    ) {
        self.clock.merge(&msg.clock);
        log::info!(
            "received, local date: {}, messsage: {:?}",
            self.get_date(),
            msg.header
        );
        msg.clock = self.clock.clone();
        self.send_message(msg, output_file, app_tx);
    }
}

pub fn run(
    mut server: Server,
    app_rx: mpsc::Receiver<Event>,
    app_tx: mpsc::Sender<AppEvent>,
    input_file_path: PathBuf,
    output_file_path: PathBuf,
) -> Result<(), Box<dyn std::error::Error>> {
    // Order matter !

    // 1 Setup event handlers
    let events = Events::new(input_file_path.to_owned(), app_rx);

    // 2 Open the output pipe,
    // the program will freeze until there is someone at the other end
    let mut output_file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(output_file_path.to_owned())
        .expect("failed to open output file");

    let mut rng = thread_rng();

    loop {
        // Handle events
        match events.next()? {
            // Input from a distant app
            Event::DistantInput(msg) => {
                if let Ok(mut msg) = Msg::from_str(&msg) {
                    // If we receive this message for the first time
                    if server.sent_messages_ids.insert(msg.id.clone()) {
                        server.increment_clock();
                        server.receive_message(&mut msg, &mut output_file, &app_tx);
                        match &msg.header {
                            Public(_) => {
                                app_tx.send(AppEvent::DistantMessage(msg)).unwrap();
                            }
                            Private(app_id, _) if *app_id == server.app_id => {
                                app_tx.send(AppEvent::DistantMessage(msg)).unwrap();
                            }
                            Private(_, _) => {}
                            SnapshotRequest(app_id) => {
                                let msg_id: MsgId = rng.gen();
                                server.sent_messages_ids.insert(msg_id.clone());
                                server.increment_clock();
                                let msg = Msg::new(
                                    msg_id,
                                    server.app_id.clone(),
                                    SnapshotResponse(app_id.clone(),
                                                     server.sent_messages.clone()),
                                    server.clock.clone(),
                                );
                                server.send_message(&msg, &mut output_file, &app_tx);
                            }
                            SnapshotResponse(app_id,_) if *app_id == server.app_id => {
                                server.snapshot.add(msg);
                                if server.snapshot.dates.0.len() == server.clock.0.len(){
                                    // We have received a snapshot from every App instance we know of
                                    // Writing snapshot to file
                                    let mut snapshot_file = OpenOptions::new().write(true)
                                                                        .create(true)
                                                                        .truncate(true)
                                                                        .open("snapshot.json")
                                                                        .expect("Failed to create snapshot file");
                                    if let Ok(snapshot_str) = server.snapshot.serialize() {
                                        snapshot_file
                                            .write_all(format!("{}\n", snapshot_str).as_bytes())
                                            .expect("Failed to write to output file");
                                        log::info!(
                                            "Snapshot saved to file, local date: {}",
                                            server.get_date()
                                        );
                                        //emptying local snapshot save
                                        server.snapshot = Snapshot::new(server.app_id.clone());
                                    } else {
                                        log::error!("Could not serialize snapshot");
                                    }
                                }
                            },
                            SnapshotResponse(_, _) => {},
                        }
                    }
                } else {
                    log::error!("Could not decode `{}` as a Msg", msg);
                }
            }
            Event::UserPublicMessage(message) => {
                let msg_id: MsgId = rng.gen();
                server.sent_messages_ids.insert(msg_id.clone());
                server.increment_clock();
                let msg = Msg::new(
                    msg_id,
                    server.app_id.clone(),
                    Public(message),
                    server.clock.clone(),
                );
                server.send_message(&msg, &mut output_file, &app_tx);
                server.sent_messages.push(msg.clone());
            }
            Event::UserPrivateMessage(app_id, message) => {
                let msg_id: MsgId = rng.gen();
                server.sent_messages_ids.insert(msg_id.clone());
                server.increment_clock();
                let msg = Msg::new(
                    msg_id,
                    server.app_id.clone(),
                    Private(app_id, message),
                    server.clock.clone(),
                );
                server.send_message(&msg, &mut output_file, &app_tx);
                server.sent_messages.push(msg.clone());
            }
            Event::GetClock => {
                app_tx
                    .send(AppEvent::Clock(server.clock.clone()))
                    .expect("failed to send message to the app");
            }
            Event::Shutdown => {
                let msg_id: MsgId = rng.gen();
                server.sent_messages_ids.insert(msg_id.clone());
                server.increment_clock();
                let msg = Msg::new(
                    msg_id,
                    server.app_id.clone(),
                    Public("left the chat".to_owned()),
                    server.clock.clone(),
                );
                server.send_message(&msg, &mut output_file, &app_tx);
                break;
            }
            Event::GetSnapshot => {
                let msg_id: MsgId = rng.gen();
                server.sent_messages_ids.insert(msg_id.clone());
                server.increment_clock();
                let msg = Msg::new(
                    msg_id,
                    server.app_id.clone(),
                    SnapshotRequest(server.app_id.to_owned()),
                    server.clock.clone(),
                );
                server.send_message(&msg, &mut output_file, &app_tx);
                server.sent_messages.push(msg.clone());
                // Adding local messages and clock to snapshot
                server.snapshot.dates.insert(server.app_id.clone(),
                   *server.clock.0.get(&server.app_id).expect("Missing server date !"));
                server.snapshot.messages.insert(server.app_id.clone(), server.sent_messages.clone());
            }
        }
    }

    Ok(())
}
