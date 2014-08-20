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
#![comment = "Bindings to TinyCDB"]
#![license = "MIT"]
#![crate_type = "lib"]
#![warn(missing_doc)]
#![warn(non_uppercase_statics)]
#![warn(managed_heap_memory)]
#![warn(unnecessary_qualification)]
#![feature(globs)]
#![feature(unsafe_destructor)]

extern crate libc;

mod ffi;

/// The module containing the basic CDB interface.
pub mod base;
