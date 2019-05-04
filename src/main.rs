use std::fs::OpenOptions;
use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use gag::Redirect;
use log;

use structopt::StructOpt;

mod server;
use server::Server;

mod app;
use app::App;

#[derive(StructOpt, Debug)]
#[structopt(name = "netchat")]
/// A fully decentralized (thus inefficient) chat written in rust
///
/// Enter  -> sends the content of the input field to everyone
///
/// Ctrl+c -> exit
///
/// Ctrl+s -> get a snapshot containing every messages sent by every site
///
/// Ctrl+r -> set the private message recipient id to the content of the input field  
///
/// Ctrl+p -> sends the content of the input field to the current private recipient
///
/// Up     -> scroll messages up
///
/// Down   -> scroll messages down
pub struct Opt {
    /// Input file
    #[structopt(short = "i", long = "input", parse(from_os_str))]
    input: PathBuf,

    /// Output file
    #[structopt(short = "o", long = "output", parse(from_os_str))]
    output: PathBuf,

    //Application Identifier
    #[structopt(short = "n", long = "name")]
    id: Option<String>,

    //Application Identifier
    #[structopt(short = "l", long = "logfile")]
    logfile: Option<PathBuf>,
}

fn main() {
    let opt = Opt::from_args();

    // Open a log file
    let logfile = opt.logfile.clone().unwrap_or("/tmp/netchat.log".into());
    let log = OpenOptions::new()
        .truncate(true)
        .read(true)
        .create(true)
        .write(true)
        .open(logfile)
        .unwrap();

    let _stderr_redirect_handle = Redirect::stderr(log).unwrap();
    color_backtrace::install();
    env_logger::init();

    let (app_tx, server_rx) = mpsc::channel(); // server -> app
    let (server_tx, app_rx) = mpsc::channel(); // app    -> server

    // Create default app state
    let mut app = App::default();

    if let Some(id) = opt.id.to_owned() {
        app.id = id;
    }

    app.messages.push(app::Message::System(format!(
        "input : {:?}, output : {:?}, id : {}",
        opt.input, opt.output, app.id
    )));

    let server = Server::new(app.id.to_owned());

    let server_handle = thread::spawn(move || {
        if let Err(e) = server::run(server, app_rx, app_tx, opt.input, opt.output) {
            log::error!("{}", e);
        }
    });

    if let Err(e) = app::run(app, server_rx, server_tx) {
        log::error!("{}", e);
    };

    server_handle
        .join()
        .expect("something went wrong with the server thread");
}
