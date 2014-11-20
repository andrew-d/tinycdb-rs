use std::io::{Command, fs};
use std::os;
use std::path::Path;

const TINYCDB_VERSION: &'static str = "0.78";

fn main() {
    let tinycdb_path = Path::new("deps")
                            .join(format!("tinycdb-{}", TINYCDB_VERSION));

    // Call "make" to build the native C library
    Command::new("make").args(&["-C", tinycdb_path.as_str().unwrap()])
                        .arg("piclib")
                        .status().unwrap();

    // Copy the output file to the expected output directory.
    let built_path = tinycdb_path.join("libcdb_pic.a");
    let output_path = Path::new(os::getenv("OUT_DIR").unwrap()).join("libcdb.a");
    fs::copy(&built_path, &output_path).unwrap();

    // Tell Rust about what we link to
    let out_dir = os::getenv("OUT_DIR").unwrap();
    println!("cargo:rustc-flags=-L {} -l cdb:static", out_dir);
}
