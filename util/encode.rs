/// This is a simple helper utility to encode a cdb file for use in our tests.
/// Pass it a filename as the first argument and it will DEFLATE and then
/// Base64 encode the contents, writing the output to stdout.


extern crate flate;
extern crate serialize;

use std::io::File;
use std::os;
use flate::deflate_bytes;
use serialize::base64::{STANDARD, ToBase64};

// NOTE: we have the attribute here to suppress irritating warnings when using
// Cargo to compile/test/benchmark the remainder of this library.
#[allow(dead_code)]
fn main() {
    let args = os::args();
    let fname = args[1].as_slice();

    let mut file = match File::open(&Path::new(fname)) {
        Err(why) => panic!("Couldn't open {}: {}", fname, why.desc),
        Ok(file) => file,
    };

    let s = match file.read_to_end() {
        Err(why) => panic!("Couldn't read {}: {}", fname, why.desc),
        Ok(s) => s,
    };

    let deflated = match deflate_bytes(s.as_slice()) {
        None => panic!("Error encoding to bytes"),
        Some(v) => v,
    };

    let encoded = deflated.as_slice().to_base64(STANDARD);
    print!("{}", encoded);
}
