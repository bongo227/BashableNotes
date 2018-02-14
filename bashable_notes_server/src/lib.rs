extern crate env_logger;
#[macro_use]
extern crate log;
extern crate pulldown_cmark;
extern crate serde;
#[macro_use]
extern crate serde_derive;
extern crate serde_json;
extern crate tempdir;
extern crate time;
extern crate ws;
extern crate notify;

mod server;
mod renderer;
mod docker;

use server::{Server, AppMessage};

use notify::{RecommendedWatcher, Watcher, RecursiveMode};
use notify::DebouncedEvent;
use std::sync::mpsc::channel;
use std::time::Duration;
use std::env;
use std::thread;

fn watch(broadcaster: ws::Sender) -> notify::Result<()> {
    let (tx, rx) = channel();

    let mut watcher: RecommendedWatcher = try!(
        Watcher::new(tx, Duration::from_secs(2)));

    try!(watcher.watch(env::current_dir().unwrap(), RecursiveMode::Recursive));

    loop {
        match rx.recv() {
            Ok(event) => {
                match event {
                    DebouncedEvent::Write(path) => {
                        let msg = AppMessage::FileUpdate{path:path.to_str().unwrap().to_string()};
                        let text = serde_json::to_string(&msg).unwrap();
                        broadcaster.send(ws::Message::Text(text)).unwrap();
                    },
                    _ => {},
                }
            },
            Err(e) => println!("watch error: {:?}", e),
        }
    }
}

pub fn start(address: &str) {
    info!("Starting websocket on ws://{}", address);
    
    let socket = ws::WebSocket::new(|out| Server { out: out }).unwrap();

    let broadcaster = socket.broadcaster();
    let watch_handle = thread::spawn(move || {
        watch(broadcaster).unwrap();
    });

    let address = String::from(address);
    let socket_handle = thread::spawn(move || {
        socket.listen(address).unwrap();
    });

    watch_handle.join().unwrap();
    socket_handle.join().unwrap();
}
