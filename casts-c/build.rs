extern crate cbindgen;

use std::env;

fn main() {
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    println!("{}", crate_dir);

    cbindgen::generate(crate_dir)
        .expect("Unable to generate bindings")
        .write_to_file("libcasts-c.h");
}
