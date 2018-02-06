extern crate iron;
extern crate mount;
extern crate mime;
extern crate phf;
extern crate include_dir;
extern crate glob;
extern crate mime_guess;
extern crate bashable_notes_server;
#[macro_use]
extern crate log;
extern crate env_logger;

use iron::{Request, Response, IronResult, Iron, status};
use iron::headers::{ContentType};
use mount::{Mount};
use mime::{Mime, SubLevel, TopLevel};
use std::path::Path;
use std::thread;
use std::fs::File;
use std::io::Read;

#[allow(dead_code)]
pub mod assets {
    include!(concat!(env!("OUT_DIR"), "/static.rs"));
}

use assets::{DirEntry, STATIC};

fn handler(req: &mut Request) -> IronResult<Response> {
    println!("Running send_hello handler, URL path: {:?}", req.url.path());
    let mut string_path = req.url.path().join("/");
    if string_path == "" {
        string_path = String::from("index.html");
    }
    
    let mut content = String::new();
    let mut path = Path::new(&string_path);
    let data = match STATIC.find(&string_path) {
        Some(file) => file.contents,
        None => {
            path = Path::new(&string_path);
            if path.exists() {
                let mut f = File::open(path).unwrap();
                f.read_to_string(&mut content).unwrap();
                content.as_bytes()
            } else {
                return Ok(Response::with(status::NotFound))
            }
        },
    };
    
    let mut resp = Response::with((status::Ok, data));
    let mime = mime_guess::guess_mime_type(path);
    resp.headers.set(ContentType(mime));
    Ok(resp)
}

fn main() {
    env_logger::init();

    let websocket_address = "127.0.0.1:3012";
    let static_server_address = "127.0.0.1:3000";
    
    let websocket_handle = thread::spawn(move || {
        bashable_notes_server::start(websocket_address);
    });
    let static_server_handle = thread::spawn(move || {
        Iron::new(handler).http("localhost:3000").unwrap();
    });

    websocket_handle.join();
    static_server_handle.join();
}