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

#[allow(dead_code)]
pub mod assets {
    include!(concat!(env!("OUT_DIR"), "/static.rs"));
}

use assets::{DirEntry, STATIC};

fn handler(req: &mut Request) -> IronResult<Response> {
    println!("Running send_hello handler, URL path: {:?}", req.url.path());
    let mut file_request = "";
    if req.url.path().len() == 1 && req.url.path()[0] == "" {
        file_request = "index.html";
    } else {
        file_request = req.url.path()[0]
    }
    
    let data = match STATIC.find(file_request) {
        Some(file) => file.contents,
        None => return Ok(Response::with(status::NotFound)),
    };
    
    let mut resp = Response::with((status::Ok, data));
    
    // resp.headers.set(ContentType(Mime(TopLevel::Text, SubLevel::Html, vec![])));
    let mime = mime_guess::guess_mime_type(Path::new(&file_request));
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