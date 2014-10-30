# tinycdb-rs

[![Build Status](https://travis-ci.org/andrew-d/tinycdb-rs.svg?branch=master)](https://travis-ci.org/andrew-d/tinycdb-rs)

This project consists of Rust bindings to [tinycdb](http://www.corpit.ru/mjt/tinycdb.html),
a small library for creating and reading constant key-value databases.

# Example

Add this to your `Cargo.toml`:

```
[dependencies.tinycdb]

git = "https://github.com/andrew-d/tinycdb-rs"
```

Then, in your crate:

```rust
extern crate tinycdb;

use tinycdb::base::Cdb;
```

Reading a database:

```rust
let path = Path::new("test.cdb");

let mut db = match Cdb::open(&path) {
    Ok(db) => db,
    Err(why) => panic!("Could not open CDB: {}", why),
};

match db.find(b"foo") {
    Some(val) => println!("Value of 'foo' key is: {}", val),
    None      => println!("'foo' key was not found"),
};
```

Creating a database:

```rust
let path = Path::new("created.cdb");

let res = Cdb::new(&path, |creator| {
    let r = creator.add(b"foo", b"bar");
    assert!(r.is_ok());
});

let mut db = match res {
    Ok(db)   => db,
    Err(why) => panic!("Could not create database: {}", why),
};

// Now, use 'db' as normal...
```

# License

MIT (the original code of TinyCDB is in the public domain)
