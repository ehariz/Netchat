use std::io::{self, Write};
use std::sync::mpsc;

use unicode_width::UnicodeWidthStr;

use termion::cursor::Goto;
use termion::event::Key;
use termion::input::MouseTerminal;
use termion::raw::IntoRawMode;
use termion::screen::AlternateScreen;

use tui::backend::TermionBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout};
use tui::style::{Color, Modifier, Style};
use tui::widgets::{Block, Borders, List, Paragraph, Text, Widget};
use tui::Terminal;

use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};

pub mod events;
use events::{Event, Events};

use crate::server::events::Event as ServerEvent;
use crate::server::messages::Header::{Private, Public};

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

/// Holds the state of the application
pub struct App {
    //Application id
    pub id: AppId,
    /// History of received/sent messages
    pub messages: Vec<Message>,
    /// Current value of the input box
    input: String,
    /// Id of the first message to be displayed, used for scrolling
    first_display_message_id: usize,
    /// Id of the private message recipient
    private_recipient_id: AppId,
}

impl Default for App {
    fn default() -> App {
        let mut rng = thread_rng();
        App {
            id: (0..8).map(|_| rng.sample(Alphanumeric)).collect(),
            input: String::new(),
            messages: Vec::new(),
            first_display_message_id: 0,
            private_recipient_id: "no one".to_owned(),
        }
    }
}

pub fn send_to_server(msg: ServerEvent, server_tx: &mpsc::Sender<ServerEvent>) {
    server_tx
        .send(msg)
        .expect("Could not send message to the server");
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

    let mut last_private_id = "no one".to_owned();

    let mut msg_list_size: usize = 0;

    loop {
        // Draw UI
        terminal.draw(|mut f| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Length(1),
                        Constraint::Length(3),
                        Constraint::Min(1),
                        Constraint::Length(1),
                    ]
                    .as_ref(),
                )
                .split(f.size());

            msg_list_size = chunks[2].inner(1).height.into();

            Paragraph::new([Text::raw("NetChat")].iter())
                .alignment(Alignment::Center)
                .render(&mut f, chunks[0]);

            Paragraph::new([Text::raw(&app.input)].iter())
                .style(Style::default().fg(Color::Cyan))
                .block(Block::default().borders(Borders::ALL).title(" Input "))
                .render(&mut f, chunks[1]);

            let messages = app
                .messages
                .iter()
                .rev()
                .skip(app.first_display_message_id)
                .map(|m| Text::raw(m.str()));
            List::new(messages)
                .block(Block::default().borders(Borders::ALL).title(" Messages "))
                .render(&mut f, chunks[2]);

            Paragraph::new(
                [
                    Text::styled("^C", Style::default().modifier(Modifier::REVERSED)),
                    Text::raw(" Quit "),
                    Text::styled("^H", Style::default().modifier(Modifier::REVERSED)),
                    Text::raw(" Display clock "),
                    Text::styled("^S", Style::default().modifier(Modifier::REVERSED)),
                    Text::raw(" Snapshot "),
                    Text::styled("^P", Style::default().modifier(Modifier::REVERSED)),
                    Text::raw(" Send private message "),
                    Text::styled("^R", Style::default().modifier(Modifier::REVERSED)),
                    Text::raw(" Set pm recipient "),
                    Text::raw(""),
                ]
                .iter(),
            )
            .render(&mut f, chunks[3]);
        })?;

        // Put the cursor back inside the input box
        write!(
            terminal.backend_mut(),
            "{}",
            Goto(2 + app.input.width() as u16, 3)
        )?;

        // Handle events
        match events.next()? {
            // Input from the user
            Event::UserInput(input) => match input {
                Key::Ctrl('c') => {
                    break;
                }
                Key::Ctrl('h') => {
                    send_to_server(ServerEvent::GetClock, &server_tx);
                }
                Key::Ctrl('s') => {
                    send_to_server(ServerEvent::GetSnapshot, &server_tx);
                }
                Key::Char('\n') => {
                    send_to_server(
                        ServerEvent::UserPublicMessage(app.input.clone()),
                        &server_tx,
                    );
                    let message: String = app.input.drain(..).collect();
                    app.messages.push(User(format!("You: {}", message)));
                }
                // set the recipient id for private messages
                Key::Ctrl('r') => {
                    let private_recipient_id: String = app.input.drain(..).collect();
                    if private_recipient_id.len() > 0 {
                        app.private_recipient_id = private_recipient_id;
                    } else {
                        app.private_recipient_id = last_private_id.clone();
                    }
                    app.messages.push(System(format!(
                        "Private recipient id set to: {}",
                        app.private_recipient_id
                    )));
                }
                Key::Ctrl('p') => {
                    send_to_server(
                        ServerEvent::UserPrivateMessage(
                            app.private_recipient_id.clone(),
                            app.input.clone(),
                        ),
                        &server_tx,
                    );
                    let message: String = app.input.drain(..).collect();
                    app.messages.push(User(format!(
                        "You to {}: {}",
                        app.private_recipient_id, message
                    )));
                }
                Key::Char(c) => {
                    app.input.push(c);
                }
                Key::Backspace => {
                    app.input.pop();
                }
                Key::Up => {
                    app.first_display_message_id = app.first_display_message_id.saturating_sub(1);
                }
                Key::Down => {
                    app.first_display_message_id = app
                        .messages
                        .len()
                        .saturating_sub(msg_list_size)
                        .min(app.first_display_message_id + 1);
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
                    last_private_id = msg.sender_id;
                }
                _ => {}
            },
            Event::DisplayClock(clock) => {
                for (id, date) in clock.0 {
                    app.messages
                        .push(System(format!("App {} date: {}", id, date)));
                }
            }
            Event::ServerMessage(string) => {
                app.messages.push(System(format!("Server: {}", string)));
            }
            Event::Tick => {}
        }
    }

    send_to_server(ServerEvent::Shutdown, &server_tx);

    Ok(())
}
