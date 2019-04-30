extern crate rand;

use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::PathBuf;
use std::collections::HashMap;
use std::iter;

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

use log;

use structopt::StructOpt;

use rand::{Rng, thread_rng};
use rand::distributions::Alphanumeric;

mod utils;
use utils::events::{Event, Events};
use utils::messages::{AppId, Date, Header, Msg};

/// Command line arguments
#[derive(StructOpt, Debug)]
#[structopt(name = "args")]
struct Opt {
    /// Input file
    #[structopt(short = "i", long = "input", parse(from_os_str))]
    input: PathBuf,

    /// Output file
    #[structopt(short = "o", long = "output", parse(from_os_str))]
    output: PathBuf,

    //Application Identifier
    #[structopt(short = "n", long = "name")]
    id: Option<String>,
}

/// App holds the state of the application
struct App {
    //Application id
    id : AppId,
    /// Current value of the input box
    input: String,
    /// History of recorded messages
    messages: Vec<String>,
    //Vector clock
    clock : HashMap<AppId,Date>,
}

impl App {
    fn update_clock(&mut self, clock : HashMap<AppId,Date>) {
        for(id, date) in &clock {
            let local_date = self.clock.get(id);
            if local_date.is_none() || local_date.unwrap() < date {
                self.clock.insert(id.clone(),date.clone());
            }
        }
    }
}

impl Default for App {
    fn default() -> App {
        let mut rng = thread_rng();
        App {
            id : iter::repeat(())
                .map(|()| rng.sample(Alphanumeric))
                .take(8)
                .collect(),
            input: String::new(),
            messages: Vec::new(),
            clock: HashMap::new(),
        }
    }
}

fn main() {
    color_backtrace::install();
    env_logger::init();

    let opt = Opt::from_args();

    if let Err(e) = run(opt) {
        eprintln!("{}", e);
    }
}

fn run(opt: Opt) -> Result<(), Box<dyn std::error::Error>> {
    // Order matter !

    // 1 Setup event handlers
    let events = Events::new(opt.input.to_owned());

    println!("Waiting for others to connect");

    // 2 Open the output pipe,
    // the program will freeze until there is someone at the other end
    let mut output_file = OpenOptions::new()
        .write(true)
        .append(true)
        .open(opt.output.to_owned())
        .unwrap();

    // Terminal initialization
    let stdout = io::stdout().into_raw_mode()?;
    let stdout = MouseTerminal::from(stdout);
    let stdout = AlternateScreen::from(stdout);
    let backend = TermionBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    // Create default app state
    let mut app = App::default();

    if let Some(id) = opt.id.to_owned() {
        app.id = id;
    }

    app.messages.push(format!("input : {:?}, output : {:?}, id : {}", opt.input, opt.output, app.id));

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
                .map(|(_, m)| Text::raw(format!("{}", m)));
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
                    for (id, date) in &app.clock {
                        app.messages.push(format!("App {} date: {}",id.to_owned(),date.to_owned()));
                    }
                }
                Key::Char('\n') => {
                    let date = app.clock.entry(app.id.to_owned()).or_insert(0);
                    *date += 1;
                    log::info!("messsage: {}", app.input);
                    log::info!("local date: {}", app.clock.get(&app.id).expect("Missing local AppId !"));
                    if let Ok(msg) = Msg::new(1, Header::Public, app.input.to_owned(), app.clock.to_owned()).serialize() {
                        output_file
                            .write_all(format!("{}\n", msg).as_bytes())
                            .expect("Failed to write to output file");
                        app.messages.push(app.input.drain(..).collect());
                    } else {
                        log::error!("Could not serialize `{}`", app.input);
                    }
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
            Event::DistantInput(msg) => {
                if let Ok(msg) = Msg::from_str(&msg) {
                    app.messages.push(msg.content);
		            app.update_clock(msg.clock);
                } else {
                    log::error!("Could not decode `{}` as a Msg", msg);
                }
            }
            _ => {}
        }
    }

    Ok(())
}
