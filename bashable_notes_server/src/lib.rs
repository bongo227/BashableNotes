extern crate env_logger;
#[macro_use]
extern crate log;
extern crate ws;
extern crate time;
extern crate pulldown_cmark;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tempdir;

mod server;
mod renderer;
mod docker;

use std::thread;

use server::Server;
use ws::listen;

pub fn start(address: &str) {    
    info!("Starting websocket on ws://{}", address);
    listen(address, |out| Server {
        out: out,
        ping_timeout: None,
        expire_timeout: None,
    }).unwrap();
}
