use std::path::PathBuf;
use std::sync::mpsc;
use std::thread;

use log;

use structopt::StructOpt;

mod server;
use server::Server;

mod app;
use app::App;

/// Command line arguments
#[derive(StructOpt, Debug)]
#[structopt(name = "args")]
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
}

fn main() {
    color_backtrace::install();
    env_logger::init();

    let opt = Opt::from_args();

    let (app_tx, server_rx) = mpsc::channel(); // server -> app
    let (server_tx, app_rx) = mpsc::channel(); // app    -> server

    // Create default app state
    let mut app = App::default();

    if let Some(id) = opt.id.to_owned() {
        app.id = id;
    }

    app.messages.push(format!(
        "input : {:?}, output : {:?}, id : {}",
        opt.input, opt.output, app.id
    ));

    let server = Server::new(app.id.to_owned());

    let server_handle = thread::spawn(move || {
        if let Err(e) = server::run(server, app_rx, app_tx, opt) {
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
