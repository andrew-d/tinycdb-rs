/*!
 * Rust bindings to the [TinyCDB](http://www.corpit.ru/mjt/tinycdb.html)
 * library.
 *
 * TinyCDB is a very fast and simple package for creating and reading constant
 * databases, as introduced by [Dan Bernstein](http://cr.yp.to/djb.html) in his
 * [cdb](http://cr.yp.to/cdb.html) package.
 *
 * CDB is a constant database, that is, it cannot be updated at a runtime, only
 * rebuilt. Rebuilding is atomic operation and is very fast - much faster than
 * of many other similar packages. Once created, CDB may be queried, and a
 * query takes very little time to complete.
 *
 */
#![crate_name = "tinycdb"]
#![crate_type = "lib"]
#![warn(missing_docs)]
#![warn(non_upper_case_globals)]
#![warn(unused_qualifications)]

extern crate libc;
extern crate tinycdb_sys as ffi;

use std::borrow::Cow;
use std::convert::Into;
use std::ffi::CString;
use std::path::Path;
use std::slice;

use libc::{c_int, c_uint, c_void};
use libc::{open, close};
use libc::{O_CREAT, O_EXCL, O_RDONLY, O_RDWR};

// Re-export the private enums
pub use ffi::CdbPutMode;

/// Kinds of errors that can be encountered.
#[derive(Debug)]
pub enum CdbErrorKind {
    /// An error resulting from an underlying I/O error.
    IoError(std::io::Error),

    // TODO: Split up actual I/O errors from errors that TinyCDB will return
    // in errno.
}

/// Our error type
#[derive(Debug)]
pub struct CdbError {
    kind: CdbErrorKind,
    message: Cow<'static, str>,
}

impl CdbError {
    /**
     * Create a new CdbError from the given kind and message.
     */
    pub fn new<T>(msg: T, kind: CdbErrorKind) -> CdbError
    where T: Into<Cow<'static, str>>
    {
        CdbError {
            kind: kind,
            message: msg.into(),
        }
    }

    /**
     * Create a new CdbError from the current errno.
     * Note: deliberately not public.
     */
    fn new_from_errno<T>(msg: T) -> CdbError
    where T: Into<Cow<'static, str>>
    {
        CdbError::new(msg, CdbErrorKind::IoError(std::io::Error::last_os_error()))
    }
}

/// A specialized Result type that might contain a CdbError.
pub type CdbResult<T> = Result<T, CdbError>;

/// A `CdbIterator` allows iterating over all the keys in a CDB database.
pub struct CdbIterator<'a> {
    underlying: &'a mut Cdb,
    cptr: c_uint,
}

// TODO: Move these into the Cdb struct.  Can't do that now because I ran into
// some lifetime errors.
impl<'a> CdbIterator<'a> {
    unsafe fn get_key_slice(&self) -> &'a [u8] {
        let len = self.underlying.cdb.cdb_keylen();
        let ptr = ffi::cdb_get(
            self.underlying.cdb_ptr(),
            len,
            self.underlying.cdb.cdb_keypos(),
        ) as *const u8;

        slice::from_raw_parts(ptr, len as usize)
    }

    unsafe fn get_data_slice(&self) -> &'a [u8] {
        let len = self.underlying.cdb.cdb_datalen();
        let ptr = ffi::cdb_get(
            self.underlying.cdb_ptr(),
            len,
            self.underlying.cdb.cdb_datapos(),
        ) as *const u8;

        slice::from_raw_parts(ptr, len as usize)
    }
}

impl<'a> Iterator for CdbIterator<'a> {
    type Item = (&'a [u8], &'a [u8]);

    fn next(&mut self) -> Option<(&'a [u8], &'a [u8])> {
        let ret = unsafe {
            ffi::cdb_seqnext(
                &mut self.cptr,
                self.underlying.cdb_mut_ptr(),
            )
        };

        // TODO: should distinguish error condition from end-of-iteration
        if ret <= 0 {
            return None
        }

        let v = unsafe { (self.get_key_slice(), self.get_data_slice()) };
        Some(v)
    }

}

// Convert a Path instance to a C-style string
fn path_as_c_str<T, F>(path: &Path, f: F) -> T
    where F: Fn(*const i8) -> T
{
    // Convert to an OsStr
    let ostr = path.as_os_str();
    let str = ostr.to_str().unwrap();
    let cstring = CString::new(str).unwrap();

    f(cstring.as_ptr())
}


/// The `Cdb` struct represents an open instance of a CDB database.
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
    pub fn open(path: &Path) -> CdbResult<Box<Cdb>> {
        let fd = path_as_c_str(path, |path| unsafe {
            open(path, O_RDONLY, 0)
        });

        if fd < 0 {
            return Err(CdbError::new_from_errno("Error opening file"));
        }

        let mut ret = Box::new(Cdb {
            fd: fd,
            cdb: unsafe { std::mem::uninitialized() },
        });

        let err = unsafe { ffi::cdb_init(ret.cdb_mut_ptr(), fd) };
        if err < 0 {
            return Err(CdbError::new_from_errno("Error initializing CDB"));
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
    pub fn new<F>(path: &Path, mut create: F) -> CdbResult<Box<Cdb>>
        where F: FnMut(&mut CdbCreator)
    {
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
    unsafe fn cdb_ptr(&self) -> *const ffi::cdb {
        &self.cdb
    }

    #[inline]
    unsafe fn cdb_mut_ptr(&mut self) -> *mut ffi::cdb {
        &mut self.cdb
    }

    /**
     * `find(key)` searches the database for the given key, and, if it's found,
     * will return the associated value as an immutable byte slice.  Note that,
     * since it is possible to have multiple records with the same key, `find`
     * will only return the value of the first key.
     */
    pub fn find(&mut self, key: &[u8]) -> Option<&[u8]> {
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

        let len = self.cdb.cdb_datalen();
        let ptr = unsafe {
            ffi::cdb_get(
                self.cdb_ptr(),
                len,
                self.cdb.cdb_datapos(),
            ) as *const u8
        };

        unsafe {
            Some(slice::from_raw_parts(ptr, len as usize))
        }
    }

    /**
     * `find_mut(key)` searches the database for the given key, and, if it's
     * found, will return the associated value as a `Vec<u8>`.  Note that,
     * since it is possible to have multiple records with the same key,
     * `find_mut` will only return the value of the first key.
     */
    pub fn find_mut(&mut self, key: &[u8]) -> Option<Vec<u8>> {
        match self.find(key) {
            Some(val) => Some(val.to_vec()),
            None      => None,
        }
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

    /**
     * `iter()` returns an iterator over all the keys in the database.  Only
     * one iterator for a database can be active at a time.
     */
    pub fn iter<'i>(&'i mut self) -> CdbIterator<'i> {
        // Need to get around the fact that we're borrowing self as mutable
        // twice - specifically, once for the CdbIterator, and once to pass to
        // cdb_seqinit.
        let cdbp = unsafe { self.cdb_mut_ptr() };

        let mut iter = CdbIterator {
            underlying: self,
            cptr: 0,
        };

        unsafe {
            ffi::cdb_seqinit(&mut iter.cptr, cdbp);
        }

        iter
    }
}

impl Drop for Cdb {
    fn drop(&mut self) {
        unsafe { close(self.fd) };
    }
}

unsafe impl Send for Cdb {}

/// The `CdbCreator` struct is used while building a new CDB instance.
pub struct CdbCreator {
    cdbm: ffi::cdb_make,
    fd: c_int,
}

impl CdbCreator {
    // Note: deliberately private
    fn new(path: &Path) -> CdbResult<Box<CdbCreator>> {
        let fd = path_as_c_str(path, |path| unsafe {
            // TODO: allow changing this mode
            open(path, O_RDWR|O_CREAT|O_EXCL, 0o644)
        });

        if fd < 0 {
            return Err(CdbError::new_from_errno("Error creating file"));
        }

        let mut ret = Box::new(CdbCreator {
            fd: fd,
            cdbm: unsafe { std::mem::uninitialized() },
        });

        let err = unsafe {
            ffi::cdb_make_start(ret.cdbm_mut_ptr(), fd)
        };
        if err < 0 {
            return Err(CdbError::new_from_errno("Error starting to make CDB"));
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
        &mut self.cdbm
    }

    fn finalize(&mut self) {
        unsafe { ffi::cdb_make_finish(self.cdbm_mut_ptr()); }
    }

    /**
     * `add(key, val)` adds the given key/value pair to the database, silently
     * overwriting any previously-existing value.  It returns whether or not
     * the operation succeeded.  Note that if this call panics, it is unsafe to
     * continue building the database.
     */
    pub fn add(&mut self, key: &[u8], val: &[u8]) -> CdbResult<()> {
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
            x if x < 0 => Err(CdbError::new_from_errno("Error adding key/value")),
            _          => Ok(()),
        }
    }

    /**
     * `exists(key)` checks whether the given key exists within the database.
     * Note that this may slow down creation, as it results in the underlying C
     * library flushing the internal buffer to disk on every call.
     */
    pub fn exists(&mut self, key: &[u8]) -> CdbResult<bool> {
        let res = unsafe {
            ffi::cdb_make_exists(
                self.cdbm_mut_ptr(),
                key.as_ptr() as *const c_void,
                key.len() as c_uint,
            )
        };
        match res {
            x if x < 0  => Err(CdbError::new_from_errno("Error checking if key exists")),
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
    pub fn remove(&mut self, key: &[u8], zero: bool) -> CdbResult<bool> {
        let mode = if zero { ffi::CdbFindMode::Fill0 } else { ffi::CdbFindMode::Remove };
        let res = unsafe {
            ffi::cdb_make_find(
                self.cdbm_mut_ptr(),
                key.as_ptr() as *const c_void,
                key.len() as c_uint,
                mode,
            )
        };
        match res {
            x if x < 0  => Err(CdbError::new_from_errno("Error removing key")),
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
    pub fn put(&mut self, key: &[u8], val: &[u8], mode: CdbPutMode) -> CdbResult<bool> {
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
            x if x < 0  => Err(CdbError::new_from_errno("Error putting key/value")),
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
    extern crate lz4;
    extern crate rustc_serialize as serialize;

    use std::borrow::ToOwned;
    use std::convert::AsRef;
    use std::fs::{self, File};
    use std::io::{Read, Write};
    use std::path::{Path, PathBuf};

    use self::serialize::base64::FromBase64;

    use super::Cdb;
    use super::ffi;

    // De-base64s and decompresses
    fn decompress_and_write(input: &[u8], path: &Path) {
        let raw = match input.from_base64() {
            Err(why) => panic!("Could not decode base64: {:?}", why),
            Ok(val) => val,
        };

        let mut decomp = Vec::new();
        let mut decoder = lz4::Decoder::new(&*raw).unwrap();
        match decoder.read_to_end(&mut decomp) {
            Err(why) => panic!("Could not decompress bytes: {:?}", why),
            Ok(_) => {},
        };

        let mut file = match File::create(path) {
            Err(why) => panic!("Couldn't create {}: {:?}", path.display(), why),
            Ok(file) => file,
        };

        match file.write(decomp.as_ref()) {
            Err(why) => panic!("Couldn't write to {}: {:?}", path.display(), why),
            Ok(_) => {},
        };
    }

    // Helper to remove test files after a test is finished, even if the test
    // panic!()s
    struct RemovingPath {
        underlying: PathBuf,
    }

    impl RemovingPath {
        pub fn new(p: &Path) -> RemovingPath {
            RemovingPath {
                underlying: p.to_owned(),
            }
        }
    }

    impl Drop for RemovingPath {
        fn drop(&mut self) {
            match fs::remove_file(&self.underlying) {
                Err(why) => println!("Couldn't remove temp file: {:?}", why),
                Ok(_) => {},
            };
        }
    }

    fn with_remove_file<F>(path: &Path, mut f: F)
        where F: FnMut(&Path)
    {
        let _p = RemovingPath::new(path);
        f(path);
    }

    fn with_test_file<F>(input: &[u8], name: &str, mut f: F)
        where F: FnMut(&Path)
    {
        let path = Path::new(name);
        with_remove_file(&path, |path| {
            decompress_and_write(input, path);
            f(path);
        });
    }

    // Simple compressed/base64'd CDB that contains the key/values:
    //      "one" --> "Hello"
    //      "two" --> "Goodbye"
    static HELLO_CDB: &'static [u8] = (
        b"BCJNGERAXl4AAAAxIggAAQAPCAD/MlMCAAAAMlABDwgA//+jAMACE0LAAg8IAP///9jw\
          AQMAAAAFAAAAb25lSGVsbG8QAPMEBwAAAHR3b0dvb2RieWUpYIcLEBYECAIAgIFbhwsA\
          CAAAAAAAAAXW/+Q="
    );

    #[test]
    fn test_basic_find() {
        let mut ran = false;

        with_test_file(HELLO_CDB, "basic.cdb", |path| {
            let mut c = match Cdb::open(path) {
                Err(why) => panic!("Could not open CDB: {:?}", why),
                Ok(c) => c,
            };

            let res = match c.find_mut(b"one") {
                None => panic!("Could not find 'one' in CDB (find_mut)"),
                Some(val) => val,
            };
            assert_eq!(&*res, b"Hello");

            let res = match c.find(b"one") {
                None => panic!("Could not find 'one' in CDB (find)"),
                Some(val) => val,
            };
            assert_eq!(res, b"Hello");

            ran = true;
        });

        assert!(ran);
    }

    #[test]
    fn test_find_not_found() {
        with_test_file(HELLO_CDB, "notfound.cdb", |path| {
            let mut c = match Cdb::open(path) {
                Err(why) => panic!("Could not open CDB: {:?}", why),
                Ok(c) => c,
            };
            match c.find("bad".as_bytes()) {
                None => {}
                Some(val) => panic!("Found unexpected value: {:?}", val),
            };
        });
    }

    #[test]
    fn test_iteration() {
        with_test_file(HELLO_CDB, "iter.cdb", |path| {
            let mut c = match Cdb::open(path) {
                Err(why) => panic!("Could not open CDB: {:?}", why),
                Ok(c) => c,
            };

            // Uncommenting this should cause compilation to panic, since we
            // can't have two iterators, both with mutable borrows, at the same
            // time.
            // let it1 = c.iter();

            let kvs: Vec<(&[u8], &[u8])> = c.iter().collect();

            assert_eq!(kvs.len(), 2);

            assert_eq!(kvs[0].0, b"one");
            assert_eq!(kvs[0].1, b"Hello");

            assert_eq!(kvs[1].0, b"two");
            assert_eq!(kvs[1].1, b"Goodbye");
        });
    }

    #[test]
    fn test_simple_create() {
        let mut ran = false;

        let path = Path::new("simple_create.cdb");
        let _rem = RemovingPath::new(&path);

        let c = Cdb::new(&path, |_creator| {
            ran = true;
        });

        match c {
            Ok(_) => {},
            Err(why) => panic!("Could not create: {:?}", why),
        }

        assert!(ran);
    }

    #[test]
    fn test_add_and_exists() {
        let path = Path::new("add.cdb");
        let _rem = RemovingPath::new(&path);

        let res = Cdb::new(&path, |creator| {
            let r = creator.add(b"foo", b"bar");
            assert!(r.is_ok());

            match creator.exists(b"foo") {
                Ok(v) => assert!(v),
                Err(why) => panic!("Could not check: {:?}", why),
            }

            match creator.exists(b"notexisting") {
                Ok(v) => assert!(!v),
                Err(why) => panic!("Could not check: {:?}", why),
            }
        });

        let mut c = match res {
            Ok(c) => c,
            Err(why) => panic!("Could not create: {:?}", why),
        };

        let res = match c.find(b"foo") {
            None => panic!("Could not find 'foo' in CDB"),
            Some(val) => val,
        };

        assert_eq!(res, b"bar");
    }

    #[test]
    fn test_remove() {
        let path = Path::new("remove.cdb");
        let _rem = RemovingPath::new(&path);

        let res = Cdb::new(&path, |creator| {
            let r = creator.add(b"foo", b"bar");
            assert!(r.is_ok());

            match creator.exists(b"foo") {
                Ok(v) => assert!(v),
                Err(why) => panic!("Could not check: {:?}", why),
            }

            let r = creator.remove(b"foo", false);
            assert!(r.is_ok());

            match creator.exists(b"foo") {
                Ok(v) => assert!(!v),
                Err(why) => panic!("Could not check: {:?}", why),
            }
        });

        let mut c = match res {
            Ok(c) => c,
            Err(why) => panic!("Could not create: {:?}", why),
        };

        match c.find(b"foo") {
            None => {},
            Some(val) => panic!("Found value for 'foo' when not expected: {:?}", val),
        };
    }

    #[test]
    fn test_put() {
        let path = Path::new("put.cdb");
        let _rem = RemovingPath::new(&path);

        let res = Cdb::new(&path, |creator| {
            let r = creator.add(b"foo", b"bar");
            assert!(r.is_ok());

            match creator.exists(b"foo") {
                Ok(v) => assert!(v),
                Err(why) => panic!("Could not check: {:?}", why),
            }

            let r = creator.put(b"foo", b"baz", ffi::CdbPutMode::Insert);
            assert!(r.is_ok());

            match creator.exists(b"foo") {
                Ok(v) => assert!(v),
                Err(why) => panic!("Could not check: {:?}", why),
            }
        });

        let mut c = match res {
            Ok(c) => c,
            Err(why) => panic!("Could not create: {:?}", why),
        };

        // The 'insert' operation should have only inserted if it didn't exist,
        // and since it did, the value is 'bar'
        match c.find(b"foo") {
            None => panic!("Could not find 'foo' in CDB"),
            Some(val) => assert_eq!(&*val, b"bar"),
        };
    }

    #[test]
    fn test_send() {
        use std::thread::spawn;

        let path = Path::new("send.cdb");
        let _rem = RemovingPath::new(&path);

        let res = Cdb::new(&path, |creator| {
            let r = creator.add(b"foo", b"bar");
            assert!(r.is_ok());
        });

        let mut c = match res {
            Ok(c) => c,
            Err(why) => panic!("Could not create: {:?}", why),
        };

        let t = spawn(move || {
            match c.find(b"foo") {
                None => panic!("Could not find 'foo' in CDB"),
                Some(val) => assert_eq!(&*val, b"bar"),
            };
        });

        t.join().unwrap();
    }
}
