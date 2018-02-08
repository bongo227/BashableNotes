extern crate bashable_notes_server;
extern crate env_logger;
extern crate glob;
extern crate include_dir;
extern crate iron;
#[macro_use]
extern crate log;
extern crate mime;
extern crate mime_guess;
extern crate mount;
extern crate phf;

use iron::{status, Iron, IronResult, Request, Response};
use iron::headers::ContentType;
use std::path::Path;
use std::thread;
use std::fs::File;
use std::io::Read;
use assets::STATIC;

fn handler(req: &mut Request) -> IronResult<Response> {
    let mut string_path = req.url.path().join("/");
    info!("request, url path: {:?}", string_path);
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
                return Ok(Response::with(status::NotFound));
            }
        }
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
        info!("starting websocket server");
        bashable_notes_server::start(websocket_address);
    });
    let static_server_handle = thread::spawn(move || {
        info!("starting static server");
        Iron::new(handler).http(static_server_address).unwrap();
    });

    websocket_handle.join().unwrap();
    static_server_handle.join().unwrap();
}

#[allow(dead_code)]
pub mod assets {
    include!(concat!(env!("OUT_DIR"), "/static.rs"));
}
