extern crate iron;
extern crate mount;
extern crate mime;
extern crate phf;
extern crate include_dir;
extern crate glob;
extern crate mime_guess;

use iron::{Request, Response, IronResult, Iron, status};
use iron::headers::{ContentType};
use mount::{Mount};
use mime::{Mime, SubLevel, TopLevel};
use std::path::Path;

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
    Iron::new(handler).http("localhost:3000").unwrap();
}