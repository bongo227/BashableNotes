extern crate env_logger;
#[macro_use]
extern crate log;
extern crate ws;
extern crate time;

mod server;

use std::thread;

use server::Server;
use ws::listen;

fn main() {
    env_logger::init();

    let server_handle = thread::spawn(|| {
        info!("websocket thread created");

        let ws_address = "127.0.0.1:3012";
        info!("Starting websocket on ws://{}", ws_address);
        listen(ws_address, |out| Server {
            out: out,
            ping_timeout: None,
            expire_timeout: None,
        }).unwrap();
    });

    server_handle.join().unwrap();
}
