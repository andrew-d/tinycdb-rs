/// This is a simple helper utility to encode a cdb file for use in our tests.
/// Pass it a filename as the first argument and it will DEFLATE and then
/// Base64 encode the contents, writing the output to stdout.


extern crate lz4;
extern crate "rustc-serialize" as serialize;

use std::fs::File;
use std::io::{Read, Write};
use std::path::Path;

use serialize::base64::{STANDARD, ToBase64};

// NOTE: we have the attribute here to suppress irritating warnings when using
// Cargo to compile/test/benchmark the remainder of this library.
fn main() {
    let fname = std::env::args().skip(1).next().unwrap();

    let mut file = match File::open(&Path::new(&*fname)) {
        Err(why) => panic!("Couldn't open {}: {:?}", fname, why),
        Ok(file) => file,
    };

    let mut buf = Vec::new();
    match file.read_to_end(&mut buf) {
        Err(why) => panic!("Couldn't read {}: {:?}", fname, why),
        Ok(_) => {},
    };

    let mut compressed = Vec::new();
    {
        let mut encoder = lz4::Encoder::new(&mut compressed, 0).unwrap();
        match encoder.write_all(&*buf) {
            Err(why) => panic!("Could not compress: {:?}", why),
            Ok(_) => {},
        };

        match encoder.finish() {
            (_, res) => res.unwrap(),
        };
    }

    let encoded = (&*compressed).to_base64(STANDARD);
    print!("{}", encoded);
}
