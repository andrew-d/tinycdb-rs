use std;
use std::mem;
use std::os::errno;

use libc;
use libc::{c_int, c_uint, c_void};
use libc::funcs::posix88::fcntl::open;
use libc::consts::os::posix88::{O_CREAT, O_RDONLY, O_RDWR};

use ffi::ffi;

pub struct CdbError {
    code: c_int,
    cause: String,
}

impl CdbError {
    pub fn new(code: c_int) -> CdbError {
        CdbError {
            code: code,
            cause: String::from_str("TODO"),
        }
    }

    #[inline]
    pub fn get_code(&self) -> c_int {
        self.code
    }
}

pub struct Cdb {
    cdb: ffi::cdb,
}

impl Cdb {
    pub fn open(path: &str) -> Result<Cdb, CdbError> {
        let fd = path.with_c_str(|path| unsafe {
            open(path, O_RDONLY, 0)
        });

        if fd < 0 {
            return Err(CdbError::new(errno() as c_int));
        }

        let mut cdb: ffi::cdb = unsafe { std::mem::zeroed() };
        let err = unsafe { ffi::cdb_init(&mut cdb as *mut ffi::cdb, fd) };
        if err < 0 {
            return Err(CdbError::new(errno() as c_int));
        }

        Ok(Cdb{
            cdb: cdb,
        })
    }

    fn cdb_ptr(&self) -> *const ffi::cdb {
        &self.cdb as *const ffi::cdb
    }

    fn cdb_mut_ptr(&mut self) -> *mut ffi::cdb {
        &mut self.cdb as *mut ffi::cdb
    }

    pub fn find(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        let res = unsafe {
            ffi::cdb_find(
                self.cdb_mut_ptr(),
                key.as_ptr() as *const c_void,
                key.len() as c_uint,
            )
        };
        if res <= 0 {
            return None
        }

        let ret = Vec::with_capacity(self.cdb.cdb_datalen() as uint);

        // TODO: Pretty sure this never returns an error...
        unsafe {
            ffi::cdb_read(
                self.cdb_ptr(),
                ret.as_ptr() as *mut c_void,
                self.cdb.cdb_datalen(),
                self.cdb.cdb_datapos()
            )
        };

        Some(ret)
    }
}

#[cfg(test)]
mod tests {
    extern crate flate;
    extern crate serialize;

    use std::io::{File, fs};
    use std::path::Path;
    use self::flate::inflate_bytes;
    use self::serialize::base64::FromBase64;

    use super::*;

    // De-base64s and decompresses
    fn decompress_and_write(input: &[u8], path: &Path) {
        let raw = match input.from_base64() {
            Err(why) => fail!("Could not decode base64: {}", why),
            Ok(val) => val,
        };
        let decomp = match inflate_bytes(raw.as_slice()) {
            None => fail!("Could not inflate bytes: {}"),
            Some(val) => val,
        };

        let mut file = match File::create(path) {
            Err(why) => fail!("Couldn't create {}: {}", path.display(), why),
            Ok(file) => file,
        };

        match file.write(decomp.as_slice()) {
            Err(why) => fail!("Couldn't write to {}: {}", path.display(), why),
            Ok(_) => {},
        };
    }

    fn with_test_file(input: &[u8], name: &str, f: |&str|) {
        let p = Path::new(name);

        decompress_and_write(input, &p);
        f(p.as_str().unwrap());
        match fs::unlink(&p) {
            Err(why) => println!("Couldn't remove temp file: {}", why),
            Ok(_) => {},
        };
    }

    static HelloCDB: &'static [u8] = (
        b"7dIxCoAwDAXQoohCF8/QzdUjuOgdXETMVswiiKNTr20qGdydWn7g8/ghY1xj3nHwt4XY\
          a4cwNeP/DtohhPlbSioJ7zSR9xx7LTlOHpm39aJuCbbV6+/cc7BG9g8="
    );

    #[test]
    fn test_basic_open() {
        with_test_file(HelloCDB, "basic.cdb", |path| {
            let c = match Cdb::open(path) {
                Err(why) => fail!("Could not open CDB"),
                Ok(c) => c,
            };
            let res = match c.find("one".as_bytes()) {
                None => fail!("Could not find 'one' in CDB"),
                Some(val) => val,
            };

            assert!(res.as_slice(), "Hello".as_slice());
        });
    }
}
