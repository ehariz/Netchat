use std::io::{self, Write};
use std::iter;
use std::sync::mpsc;

use unicode_width::UnicodeWidthStr;

use termion::cursor::Goto;
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;

use tui::backend::TermionBackend;
use tui::layout::{Constraint, Direction, Layout};
use tui::style::{Color, Style};
use tui::widgets::{Block, Borders, List, Paragraph, Text, Widget};
use tui::Terminal;

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

pub mod events;
use events::{Event, Events};

use crate::server::events::Event as ServerEvent;
use crate::server::messages::Header::*;

pub type AppId = String;

pub enum Message {
    System(String),
    User(String),
}
use Message::*;

impl Message {
    pub fn str(&self) -> &str {
        match self {
            System(s) => s,
            User(s) => s,
        }
    }
}

/// App holds the state of the application
pub struct App {
    //Application id
    pub id: AppId,
    /// History of recorded messages
    pub messages: Vec<Message>,
    /// Current value of the input box
    input: String,
}

impl Default for App {
    fn default() -> App {
        let mut rng = thread_rng();
        App {
            id: iter::repeat(())
                .map(|()| rng.sample(Alphanumeric))
                .take(8)
                .collect(),
            input: String::new(),
            messages: Vec::new(),
        }
    }
}

pub fn run(
    mut app: App,
    server_rx: mpsc::Receiver<Event>,
    server_tx: mpsc::Sender<ServerEvent>,
) -> Result<(), Box<dyn std::error::Error>> {
    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let events = Events::new(server_rx);

    let mut last_private_id = "You".to_owned();
    server_tx
        .send(ServerEvent::UserPublicMessage(
            "joined the chat".to_string(),
        ))
        .expect("failed to send message to the server");

    loop {
        // Draw UI
        terminal.draw(|mut f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints([Constraint::Length(3), Constraint::Min(1)].as_ref())
                .split(f.size());
            Paragraph::new([Text::raw(&app.input)].iter())
                .style(Style::default().fg(Color::Cyan))
                .block(Block::default().borders(Borders::ALL).title("Input"))
                .render(&mut f, chunks[0]);
            let messages = app
                .messages
                .iter()
                .rev()
                .enumerate()
                .map(|(_, m)| Text::raw(format!("{}", m.str())));
            List::new(messages)
                .block(Block::default().borders(Borders::ALL).title("Messages"))
                .render(&mut f, chunks[1]);
        })?;

        // Put the cursor back inside the input box
        write!(
            terminal.backend_mut(),
            "{}",
            Goto(4 + app.input.width() as u16, 4)
        )?;

        // Handle events
        match events.next()? {
            // Input from the user
            Event::UserInput(input) => match input {
                Key::Ctrl('c') => {
                    break;
                }
                Key::Ctrl('h') => {
                    server_tx
                        .send(ServerEvent::GetClock)
                        .expect("failed to send message to the server");
                }
                Key::Ctrl('s') => {
                    server_tx
                        .send(ServerEvent::GetSnapshot)
                        .expect("failed to send message to the server");
                }
                Key::Char('\n') => {
                    server_tx
                        .send(ServerEvent::UserPublicMessage(app.input.clone()))
                        .expect("failed to send message to the server");
                    let message: String = app.input.drain(..).collect();
                    app.messages.push(User(format!("You: {}", message)));
                }
                // set the recipient id for private messages
                Key::Ctrl('r') => {
                    last_private_id = app.input.drain(..).collect();
                    app.messages.push(System(format!(
                        "Private recipient id set to: {}",
                        last_private_id
                    )));
                }
                Key::Ctrl('p') => {
                    server_tx
                        .send(ServerEvent::UserPrivateMessage(
                            last_private_id.clone(),
                            app.input.clone(),
                        ))
                        .expect("failed to send message to the server");
                    let message: String = app.input.drain(..).collect();
                    app.messages
                        .push(User(format!("You to {}: {}", last_private_id, message)));
                }
                Key::Char(c) => {
                    app.input.push(c);
                }
                Key::Backspace => {
                    app.input.pop();
                }
                _ => {}
            },
            // Input from a distant app
            Event::DistantMessage(msg) => match &msg.header {
                Public(content) => {
                    app.messages
                        .push(User(format!("{}: {}", msg.sender_id, content)));
                }
                Private(_, content) => {
                    app.messages
                        .push(User(format!("{} to You: {}", msg.sender_id, content)));
                }
                SnapshotRequest(_) => {},
                SnapshotResponse(_,_) => {},
            },
            Event::Clock(clock) => {
                for (id, date) in clock.0 {
                    app.messages
                        .push(System(format!("App {} date: {}", id, date)));
                }
            }
            Event::ServerMessage(string) => {
                app.messages.push(System(format!("Sever: {}", string)));
            }
            Event::Tick => {}
        }
    }

    server_tx
        .send(ServerEvent::Shutdown)
        .expect("failed to send message to the server");

    Ok(())
}
