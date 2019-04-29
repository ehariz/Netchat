use std::fs::File;
use std::io;
// BufRead : WHY ????
use std::io::{BufRead, BufReader};
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use termion::event::Key;
use termion::input::TermRead;

pub enum Event {
    /// User input (keypress)
    UserInput(Key),
    /// Input from a distant agent (write in a file)
    DistantInput(String),
    /// Periodically send tick a to refresh the UI
    Tick,
}

/// A small event handler that wrap termion input and tick events. Each event
/// type is handled in its own thread and returned to a common `Receiver`
pub struct Events {
    rx: mpsc::Receiver<Event>,
    _input_handle: thread::JoinHandle<()>,
    _input_file_handle: thread::JoinHandle<()>,
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
    pub fn new(input_file: PathBuf) -> Events {
        Events::with_config(input_file, Config::default())
    }

    pub fn with_config(input_file: PathBuf, config: Config) -> Events {
        let (tx, rx) = mpsc::channel();

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

        // listen to a file for distant events
        let _input_file_handle = {
            let tx = tx.clone();
            thread::spawn(move || {
                let input_file = File::open(input_file).expect("Could not open input file");
                let reader = BufReader::new(input_file);

                reader.lines().for_each(|line| {
                    tx.send(Event::DistantInput(
                        line.expect("Could not read from input file"),
                    ))
                    .unwrap();
                })
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
            _input_file_handle,
        }
    }

    pub fn next(&self) -> Result<Event, mpsc::RecvError> {
        self.rx.recv()
    }
}
