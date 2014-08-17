#[allow(dead_code)]
#[allow(non_camel_case_types)]
pub mod ffi {
    use libc::{c_int, c_uchar, c_uint, c_void};

    pub struct cdb {
        // File descriptor
        pub cdb_fd: c_int,

        // Datafile size
        cdb_fsize: c_uint,

        // End of data ptr
        cdb_dend: c_uint,

        // mmap'ed file memory
        cdb_mem: *const c_uchar,

        // Found data
        cdb_vpos: c_uint,
        cdb_vlen: c_uint,

        // Found key
        cdb_kpos: c_uint,
        cdb_klen: c_uint,
    }

    // Macros defined in C
    impl cdb {
        #[inline]
        pub fn cdb_datapos(&self) -> c_uint {
            self.cdb_vpos
        }

        #[inline]
        pub fn cdb_datalen(&self) -> c_uint {
            self.cdb_vlen
        }

        #[inline]
        pub fn cdb_keypos(&self) -> c_uint {
            self.cdb_kpos
        }

        #[inline]
        pub fn cdb_keylen(&self) -> c_uint {
            self.cdb_klen
        }
    }

    pub struct cdb_find {
        cdb_cdbp: *mut cdb,
        cdb_hval: c_uint,
        cdb_htp: *const c_uchar,
        cdb_htab: *const c_uchar,
        cdb_htend: *const c_uchar,
        cdb_httodo: c_uint,
        cdb_key: *const c_void,
        cdb_klen: c_uint,
    }

    pub struct cdb_make {
        // File descriptor
        pub cdb_fd: c_int,

        // Data position so far
        cdb_dpos: c_uint,

        // Record count so far
        cdb_rcnt: c_uint,

        // Write buffer
        cdb_buf: [c_uchar, ..4096],

        // Current buf position
        cdb_bpos: *mut c_uchar,

        // List of arrays of record infos
        // OLD: cdb_rl*
        cdb_rec: [*mut c_void, ..256],
    }

    type cdb_put_mode = c_uint;

    pub static CDB_PUT_ADD: cdb_put_mode = 0;
    pub static CDB_PUT_REPLACE: cdb_put_mode = 1;
    pub static CDB_PUT_INSERT: cdb_put_mode = 2;
    pub static CDB_PUT_WARN: cdb_put_mode = 3;
    pub static CDB_PUT_REPLACE0: cdb_put_mode = 4;

    pub static CDB_FIND: c_uint = 0;            // == CDB_PUT_ADD
    pub static CDB_FIND_REMOVE: c_uint = 1;     // == CDB_PUT_REPLACE
    pub static CDB_FIND_FILL0: c_uint = 4;      // == CDB_PUT_REPLACE0


    #[link(name = "cdb", kind = "static")]
    extern "C" {
        pub fn cdb_init(cdbp: *mut cdb, fd: c_int) -> c_int;
        pub fn cdb_free(cdbp: *mut cdb);
        pub fn cdb_read(cdbp: *const cdb, buf: *mut c_void, len: c_uint, pos: c_uint) -> c_int;
        pub fn cdb_get(cdbp: *const cdb, len: c_uint, pos: c_uint) -> *const c_void;

        pub fn cdb_find(cdbp: *mut cdb, key: *const c_void, klen: c_uint) -> c_int;
        pub fn cdb_findinit(cdbfp: *mut cdb_find, cdb: *mut cdb, key: *const c_void, klen: c_uint) -> c_int;
        pub fn cdb_findnext(cdbfp: *mut cdb_find) -> c_int;

        pub fn cdb_make_start(cdbmp: *mut cdb_make, fd: c_int) -> c_int;
        pub fn cdb_make_add(cdbmp: *mut cdb_make, key: *const c_void, klen: c_uint, val: *const c_void, vlen: c_uint) -> c_int;
        pub fn cdb_make_exists(cdbmp: *mut cdb_make, key: *const c_void, klen: c_uint) -> c_int;
        pub fn cdb_make_find(cdbmp: *mut cdb_make, key: *const c_void, klen: c_uint, mode: cdb_put_mode) -> c_int;
        pub fn cdb_make_put(cdbmp: *mut cdb_make, key: *const c_void, klen: c_uint, val: *const c_void, vlen: c_uint, mode: cdb_put_mode) -> c_int;
        pub fn cdb_make_finish(cdbmp: *mut cdb_make) -> c_int;
    }
}
