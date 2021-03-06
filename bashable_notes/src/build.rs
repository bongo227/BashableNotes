extern crate include_dir;
extern crate tempdir;

use std::process::Command;
use std::path::Path;
use std::env;
use include_dir::include_dir;

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let out_file = Path::new(&out_dir).join("static.rs");
    let in_dir = Path::new(&out_dir).join("static/");

    // build bashable_notes_client
    Command::new("parcel")
        .arg("build")
        .arg("../bashable_notes_client/src/index.html")
        .arg("--no-cache")
        .arg("--out-dir")
        .arg(in_dir.as_os_str())
        .arg("--public-url")
        .arg("./")
        .output()
        .unwrap();

    std::thread::sleep_ms(1000);

    include_dir(in_dir.to_str().unwrap())
    // include_dir(&in_dir.canonicalize().unwrap().to_string_lossy())
        .as_variable("STATIC")
        .to_file(out_file)
        .unwrap();
}
