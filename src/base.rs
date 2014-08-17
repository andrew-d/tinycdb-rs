use std;
use std::os::errno;

use libc::{c_int, c_uint, c_void};
use libc::funcs::posix88::fcntl::open;
use libc::funcs::posix88::unistd::close;
use libc::consts::os::posix88::{O_CREAT, O_EXCL, O_RDONLY, O_RDWR};

// Re-export the private enums

pub use ffi::ffi::CdbPutMode;

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
    fd: c_int,
}

impl Cdb {
    /**
     * `open(path)` will open the CDB database at the given file path,
     * returning either the `CDB` struct or an error indicating why the
     * database could not be opened.
     */
    pub fn open(path: &str) -> Result<Box<Cdb>, CdbError> {
        let fd = path.with_c_str(|path| unsafe {
            open(path, O_RDONLY, 0)
        });

        if fd < 0 {
            return Err(CdbError::new(errno() as c_int));
        }

        let mut ret = box Cdb {
            fd: fd,
            cdb: unsafe { std::mem::uninitialized() },
        };

        let err = unsafe { ffi::cdb_init(ret.cdb_mut_ptr(), fd) };
        if err < 0 {
            return Err(CdbError::new(errno() as c_int));
        }

        Ok(ret)
    }

    /**
     * `new(path, cb)` is responsible for creating a new CDB database.  The
     * given closure is called with an instance of a `CdbCreator`, allowing the
     * closure to insert values into the CDB database.  Once the closure
     * returns, the database can no longer be updated.  The now-open database
     * instance is then returned.
     */
    pub fn new(path: &str, create: |&mut CdbCreator|) -> Result<Box<Cdb>, CdbError> {
        // This is its own scope because we want it to be closed before trying
        // to re-open it below.
        {
            // TODO: create as temp file
            let mut creator = match CdbCreator::new(path) {
                Ok(c) => c,
                Err(r) => return Err(r),
            };

            // Call the creation function
            create(&mut *creator);

            // Finalize the database.
            creator.finalize();
        }

        // TODO: rename into place

        // Delegate to the real 'open' function.
        Cdb::open(path)
    }

    #[inline]
    fn cdb_ptr(&self) -> *const ffi::cdb {
        &self.cdb as *const ffi::cdb
    }

    #[inline]
    fn cdb_mut_ptr(&mut self) -> *mut ffi::cdb {
        &mut self.cdb as *mut ffi::cdb
    }

    /**
     * `find(key)` searches the database for the given key, and, if it's found,
     * will return the associated value as a `Vec<u8>`.  Note that, since it is
     * possible to have multiple records with the same key, `find()` will only
     * return the value of the first key.
     */
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

        let mut ret = Vec::with_capacity(self.cdb.cdb_datalen() as uint);

        unsafe {
            // TODO: Pretty sure this never returns an error...
            ffi::cdb_read(
                self.cdb_ptr(),
                ret.as_ptr() as *mut c_void,
                self.cdb.cdb_datalen(),
                self.cdb.cdb_datapos()
            );

            ret.set_len(self.cdb.cdb_datalen() as uint);
        }

        Some(ret)
    }

    /**
     * `exists(key)` returns whether the key exists in the database.  This is
     * essentially the same as the `find(key)` call, except that it does not
     * allocate space for the returned value, and thus may be faster.
     */
    pub fn exists(&mut self, key: &[u8]) -> bool {
        let res = unsafe {
            ffi::cdb_find(
                self.cdb_mut_ptr(),
                key.as_ptr() as *const c_void,
                key.len() as c_uint,
            )
        };
        if res <= 0 {
            false
        } else {
            true
        }
    }
}

impl Drop for Cdb {
    fn drop(&mut self) {
        unsafe { close(self.fd) };
    }
}

pub struct CdbCreator {
    cdbm: ffi::cdb_make,
    fd: c_int,
}

/// This structure contains methods that can be used while creating a new CDB.
impl CdbCreator {
    // Note: deliberately private
    fn new(path: &str) -> Result<Box<CdbCreator>, CdbError> {
        let fd = path.with_c_str(|path| unsafe {
            // TODO: allow changing this mode
            open(path, O_RDWR|O_CREAT|O_EXCL, 0o644)
        });

        if fd < 0 {
            return Err(CdbError::new(errno() as c_int));
        }

        let mut ret = box CdbCreator {
            fd: fd,
            cdbm: unsafe { std::mem::uninitialized() },
        };

        let err = unsafe {
            ffi::cdb_make_start(ret.cdbm_mut_ptr(), fd)
        };
        if err < 0 {
            return Err(CdbError::new(errno() as c_int));
        }

        Ok(ret)
    }

    /*
    fn cdbm_ptr(&self) -> *const ffi::cdb_make {
        &self.cdbm as *const ffi::cdb_make
    }
    */

    #[inline]
    fn cdbm_mut_ptr(&mut self) -> *mut ffi::cdb_make {
        &mut self.cdbm as *mut ffi::cdb_make
    }

    fn finalize(&mut self) {
        unsafe { ffi::cdb_make_finish(self.cdbm_mut_ptr()); }
    }

    /**
     * `add(key, val)` adds the given key/value pair to the database, silently
     * overwriting any previously-existing value.  It returns whether or not
     * the operation succeeded.  Note that if this call fails, it is unsafe to
     * continue building the database.
     */
    pub fn add(&mut self, key: &[u8], val: &[u8]) -> Result<(), CdbError> {
        let res = unsafe {
            ffi::cdb_make_add(
                self.cdbm_mut_ptr(),
                key.as_ptr() as *const c_void,
                key.len() as c_uint,
                val.as_ptr() as *const c_void,
                val.len() as c_uint,
            )
        };
        match res {
            x if x < 0 => Err(CdbError::new(errno() as c_int)),
            _          => Ok(()),
        }
    }

    /**
     * `exists(key)` checks whether the given key exists within the database.
     * Note that this may slow down creation, as it results in the underlying C
     * library flushing the internal buffer to disk on every call.
     */
    pub fn exists(&mut self, key: &[u8]) -> Result<bool, CdbError> {
        let res = unsafe {
            ffi::cdb_make_exists(
                self.cdbm_mut_ptr(),
                key.as_ptr() as *const c_void,
                key.len() as c_uint,
            )
        };
        match res {
            x if x < 0  => Err(CdbError::new(errno() as c_int)),
            x if x == 0 => Ok(false),
            _           => Ok(true),
        }
    }

    /**
     * `remove(key)` will remove the given key from the database.  If the
     * `zero` parameter is true, then an existing key/value pair will be zeroed
     * out in the database.  This prevents it from being found in normal lookups,
     * but it will still be present in sequential scans of the database.  If
     * the `zero` parameter is false, then the entire record will be removed
     * from the database.  Note, however, that removing a record also involves
     * moving other records in the database, and may take much longer to
     * complete.
     * Note that a `remove()` of the most recently-added key, regardless of the
     * the `zero` parameter, will always remove it from the database entirely.
     * The return value from this function indicates whether or not any keys
     * were removed.
     */
    pub fn remove(&mut self, key: &[u8], zero: bool) -> Result<bool, CdbError> {
        let mode = if zero { ffi::Fill0 } else { ffi::Remove };
        let res = unsafe {
            ffi::cdb_make_find(
                self.cdbm_mut_ptr(),
                key.as_ptr() as *const c_void,
                key.len() as c_uint,
                mode,
            )
        };
        match res {
            x if x < 0  => Err(CdbError::new(errno() as c_int)),
            x if x == 0 => Ok(false),
            _           => Ok(true),
        }
    }

    /**
     * `put(key, val, mode)` will add a new key to the database, with a
     * configurable behaviour if the key already exists.  See the documentation
     * on `CdbPutMode` for more information on the options available.
     * The return value from this function indicates whether or not any existing
     * keys were found in the database during the put operation.
     */
    pub fn put(&mut self, key: &[u8], val: &[u8], mode: CdbPutMode) -> Result<bool, CdbError> {
        let res = unsafe {
            ffi::cdb_make_put(
                self.cdbm_mut_ptr(),
                key.as_ptr() as *const c_void,
                key.len() as c_uint,
                val.as_ptr() as *const c_void,
                val.len() as c_uint,
                mode,
            )
        };
        match res {
            x if x < 0  => Err(CdbError::new(errno() as c_int)),
            x if x == 0 => Ok(false),
            _           => Ok(true),
        }
    }
}

impl Drop for CdbCreator {
    fn drop(&mut self) {
        unsafe { close(self.fd) };
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

    // Helper to remove test files after a test is finished, even if the test
    // fail!()s
    struct RemovingPath {
        underlying: Path,
    }

    impl RemovingPath {
        pub fn new(p: Path) -> RemovingPath {
            RemovingPath {
                underlying: p,
            }
        }

        pub fn as_str(&self) -> &str {
            // Want to fail here, if we're in a test
            self.underlying.as_str().unwrap()
        }
    }

    impl Drop for RemovingPath {
        fn drop(&mut self) {
            match fs::unlink(&self.underlying) {
                Err(why) => println!("Couldn't remove temp file: {}", why),
                Ok(_) => {},
            };
        }
    }

    fn with_remove_file(name: &str, f: |&str|) {
        let p = RemovingPath::new(Path::new(name));
        f(p.as_str());
    }

    fn with_test_file(input: &[u8], name: &str, f: |&str|) {
        with_remove_file(name, |name| {
            let p = Path::new(name);
            decompress_and_write(input, &p);
            f(name);
        });
    }

    // Simple compressed/base64'd CDB that contains the key/values:
    //      "one" --> "Hello"
    //      "two" --> "Goodbye"
    static HelloCDB: &'static [u8] = (
        b"7dIxCoAwDAXQoohCF8/QzdUjuOgdXETMVswiiKNTr20qGdydWn7g8/ghY1xj3nHwt4XY\
          a4cwNeP/DtohhPlbSioJ7zSR9xx7LTlOHpm39aJuCbbV6+/cc7BG9g8="
    );

    #[test]
    fn test_basic_find() {
        let mut ran = false;

        with_test_file(HelloCDB, "basic.cdb", |path| {
            let mut c = match Cdb::open(path) {
                Err(why) => fail!("Could not open CDB: {}", why.get_code()),
                Ok(c) => c,
            };
            let res = match c.find(b"one") {
                None => fail!("Could not find 'one' in CDB"),
                Some(val) => val,
            };

            assert_eq!(res.as_slice(), b"Hello");
            ran = true;
        });

        assert!(ran);
    }

    #[test]
    fn test_find_not_found() {
        with_test_file(HelloCDB, "notfound.cdb", |path| {
            let mut c = match Cdb::open(path) {
                Err(why) => fail!("Could not open CDB: {}", why.get_code()),
                Ok(c) => c,
            };
            match c.find("bad".as_bytes()) {
                None => {}
                Some(val) => fail!("Found unexpected value: {}", val),
            };
        });
    }

    #[test]
    fn test_simple_create() {
        let mut ran = false;

        let path = "simple_create.cdb";
        let _rem = RemovingPath::new(Path::new(path));

        let c = Cdb::new(path, |_creator| {
            ran = true;
        });

        match c {
            Ok(_) => {},
            Err(why) => fail!("Could not create: {}", why.get_code()),
        }

        assert!(ran);
    }

    #[test]
    fn test_add_and_exists() {
        let path = "add.cdb";
        let _rem = RemovingPath::new(Path::new(path));

        let res = Cdb::new(path, |creator| {
            let r = creator.add(b"foo", b"bar");
            assert!(r.is_ok());

            match creator.exists(b"foo") {
                Ok(v) => assert!(v),
                Err(why) => fail!("Could not check: {}", why.get_code()),
            }

            match creator.exists(b"notexisting") {
                Ok(v) => assert!(!v),
                Err(why) => fail!("Could not check: {}", why.get_code()),
            }
        });

        let mut c = match res {
            Ok(c) => c,
            Err(why) => fail!("Could not create: {}", why.get_code()),
        };

        let res = match c.find(b"foo") {
            None => fail!("Could not find 'foo' in CDB"),
            Some(val) => val,
        };

        assert_eq!(res.as_slice(), b"bar");
    }
}
