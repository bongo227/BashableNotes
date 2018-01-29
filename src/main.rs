extern crate iron;
extern crate time;
extern crate comrak;
extern crate typed_arena;
extern crate staticfile;
extern crate mount;
extern crate router;

use iron::prelude::*;
use iron::{BeforeMiddleware, AfterMiddleware, typemap};
use iron::headers::ContentType;
use time::precise_time_ns;

mod parser;
use parser::{parse_markdown};

use std::path::Path;
use staticfile::Static;
use mount::Mount;
use router::Router;

struct ResponseTime;

impl typemap::Key for ResponseTime { type Value = u64; }

impl BeforeMiddleware for ResponseTime {
    fn before(&self, req: &mut Request) -> IronResult<()> {
        req.extensions.insert::<ResponseTime>(precise_time_ns());
        Ok(())
    }
}

impl AfterMiddleware for ResponseTime {
    fn after(&self, req: &mut Request, res: Response) -> IronResult<Response> {
        let delta = precise_time_ns() - *req.extensions.get::<ResponseTime>().unwrap();
        println!("Request took: {} ms", (delta as f64) / 1000000.0);
        Ok(res)
    }
}

fn hello_world(_: &mut Request) -> IronResult<Response> {
    // Parse markdown
    let html = parse_markdown();

    // Serve up html
    let resp = Response::with((ContentType::html().0, iron::status::Ok, html));
    Ok(resp)
}

fn main() {
    let mut chain = Chain::new(hello_world);
    chain.link_before(ResponseTime);
    chain.link_after(ResponseTime);
    
    let mut router = Router::new();
    router.get("/", chain, "document");
    
    // Serve the shared JS/CSS at /
    let mut mount = Mount::new();
    mount.mount("/", router);
    mount.mount("res/", Static::new(Path::new("res/")));

    Iron::new(mount).http("localhost:3000").unwrap();
}