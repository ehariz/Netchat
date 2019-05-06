use std::io;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use termion::event::Key;
use termion::input::TermRead;

use crate::server::messages::Msg;
use crate::server::Clock;

pub enum Event {
    /// User input (keypress)
    UserInput(Key),
    /// Message from another app (write in a file)
    DistantMessage(Msg),
    /// Information from the server
    ServerMessage(String),
    /// Periodically send tick a to refresh the UI
    Tick,
    /// Display vector clock
    DisplayClock(Clock),
}

/// A small event handler that wraps termion input and tick events. Each event
/// type is handled in its own thread and returned to a common `Receiver`
pub struct Events {
    rx: mpsc::Receiver<Event>,
    _server_handle: thread::JoinHandle<()>,
    _input_handle: thread::JoinHandle<()>,
    _tick_handle: thread::JoinHandle<()>,
}

#[derive(Debug, Clone, Copy)]
pub struct Config {
    pub exit_key: Key,
    pub tick_rate: Duration,
}

impl Default for Config {
    fn default() -> Config {
        Config {
            exit_key: Key::Ctrl('c'),
            tick_rate: Duration::from_millis(250),
        }
    }
}

impl Events {
    pub fn new(server_rx: mpsc::Receiver<Event>) -> Events {
        Events::with_config(server_rx, Config::default())
    }

    pub fn with_config(server_rx: mpsc::Receiver<Event>, config: Config) -> Events {
        let (tx, rx) = mpsc::channel();

        // listen to the server
        let _server_handle = {
            let tx = tx.clone();
            thread::spawn(move || loop {
                if let Ok(event) = server_rx.recv() {
                    // Forward server events
                    tx.send(event).unwrap();
                } else {
                    log::info!("server disconnected");
                    break;
                }
            })
        };

        // listen to stdin for user events
        let _input_handle = {
            let tx = tx.clone();
            thread::spawn(move || {
                let stdin = io::stdin();
                for evt in stdin.keys() {
                    match evt {
                        Ok(key) => {
                            if let Err(_) = tx.send(Event::UserInput(key)) {
                                return;
                            }
                            if key == config.exit_key {
                                return;
                            }
                        }
                        Err(_) => {}
                    }
                }
            })
        };

        let _tick_handle = {
            let tx = tx.clone();
            thread::spawn(move || loop {
                tx.send(Event::Tick).unwrap();
                thread::sleep(config.tick_rate);
            })
        };

        Events {
            rx,
            _input_handle,
            _tick_handle,
            _server_handle,
        }
    }

    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        self.rx.recv()
    }
}
