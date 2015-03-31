use std::env;
use std::fs;
use std::path::Path;
use std::process::Command;

const TINYCDB_VERSION: &'static str = "0.78";

fn main() {
    let out_dir = env::var("OUT_DIR").unwrap();
    let tinycdb_folder = format!("tinycdb-{}", TINYCDB_VERSION);
    let tinycdb_path = Path::new("deps")
                            .join(&tinycdb_folder);

    // Clean the build directory first.
    Command::new("make").arg("-C")
                        .arg(&tinycdb_path)
                        .arg("clean")
                        .status().unwrap();

    // Call "make" to build the native C library
    Command::new("make").arg("-C")
                        .arg(&tinycdb_path)
                        .arg("piclib")
                        .status().unwrap();

    // Copy the output file to the expected output directory.
    let built_path = tinycdb_path.join("libcdb_pic.a");
    let output_path = Path::new(&out_dir).join("libcdb.a");
    fs::copy(&built_path, &output_path).unwrap();

    // Tell Rust about what we link to
    println!("cargo:rustc-flags=-L {} -l static=cdb", out_dir);
}
