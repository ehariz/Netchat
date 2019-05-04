use std::fs::File;
// BufRead : WHY ????
use crate::app::AppId;
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

pub enum Event {
    /// User public message
    UserPublicMessage(String),
    /// User public message
    UserPrivateMessage(AppId, String),
    /// Input from a distant agent (write in a file)
    DistantInput(String),
    /// Shutdown the server
    Shutdown,
    /// Get Clock
    GetClock,
    /// Request snapshot from other apps
    GetSnapshot,
    /// The wait for the snapshot lasted enough
    SnapshotTimeout,
}

/// A small event handler that wrap termion input and tick events. Each event
/// type is handled in its own thread and returned to a common `Receiver`
pub struct Events {
    rx: mpsc::Receiver<Event>,
    _app_handle: thread::JoinHandle<()>,
    _input_file_handle: thread::JoinHandle<()>,
}

impl Events {
    pub fn new(
        input_file_path: PathBuf,
        app_rx: mpsc::Receiver<Event>,
        server_rx: mpsc::Receiver<Event>,
    ) -> Events {
        let (tx, rx) = mpsc::channel();

        // listen to the app for user commands
        let _app_handle = {
            let tx = tx.clone();
            thread::spawn(move || loop {
                if let Ok(event) = app_rx.recv() {
                    // Forward app events
                    tx.send(event).unwrap();
                } else {
                    log::info!("app disconnected");
                    break;
                }
            })
        };

        // listen to the server for distant events
        let _input_file_handle = {
            let tx = tx.clone();
            thread::spawn(move || loop {
                let input_file = File::open(&input_file_path).expect("Could not open input file");
                let reader = BufReader::new(input_file);
                reader.lines().for_each(|line| {
                    tx.send(Event::DistantInput(
                        line.expect("Could not read from input file"),
                    ))
                    .unwrap();
                })
            })
        };

        // listen to server events to allow to speak to itself asynchronously
        let _server_handle = {
            let tx = tx.clone();
            thread::spawn(move || loop {
                if let Ok(event) = server_rx.recv() {
                    // Forward server events
                    tx.send(event).unwrap();
                }
            })
        };

        Events {
            rx,
            _app_handle,
            _input_file_handle,
        }
    }

    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        self.rx.recv()
    }
}
