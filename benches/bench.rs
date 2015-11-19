#![feature(test)]

extern crate test;
extern crate tinycdb;

use std::fs;
use std::path::{Path, PathBuf};

use test::Bencher;
use tinycdb::Cdb;


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

#[bench]
fn bench_add(b: &mut Bencher) {
    use std::sync::atomic::{AtomicUsize, Ordering};
    let ctr = AtomicUsize::new(0);

    let path = Path::new("add_bench.cdb");
    let _rem = RemovingPath::new(&path);

    let _ = Cdb::new(&path, |creator| {
        b.iter(|| {
            let cnt_str = ctr.fetch_add(1, Ordering::SeqCst).to_string();
            let mut key = "key".to_string();
            key.push_str(cnt_str.as_ref());

            let mut val = "val".to_string();
            val.push_str(cnt_str.as_ref());

            let _ = creator.add(key.as_bytes(), val.as_bytes());
        })
    });
}

#[bench]
fn bench_find(b: &mut Bencher) {
    let path = Path::new("find_bench.cdb");
    let _rem = RemovingPath::new(&path);

    let res = Cdb::new(&path, |creator| {
        let r = creator.add(b"foo", b"bar");
        assert!(r.is_ok());
    });

    let mut c = match res {
        Ok(c) => c,
        Err(why) => panic!("Could not create: {:?}", why),
    };

    b.iter(|| {
        test::black_box(c.find(b"foo"));
    });
}

#[bench]
fn bench_find_mut(b: &mut Bencher) {
    let path = Path::new("find_mut_bench.cdb");
    let _rem = RemovingPath::new(&path);

    let res = Cdb::new(&path, |creator| {
        let r = creator.add(b"foo", b"bar");
        assert!(r.is_ok());
    });

    let mut c = match res {
        Ok(c) => c,
        Err(why) => panic!("Could not create: {:?}", why),
    };

    b.iter(|| {
        test::black_box(c.find_mut(b"foo"));
    });
}

#[bench]
fn bench_exists(b: &mut Bencher) {
    let path = Path::new("exists_bench.cdb");
    let _rem = RemovingPath::new(&path);

    let res = Cdb::new(&path, |creator| {
        let r = creator.add(b"foo", b"bar");
        assert!(r.is_ok());
    });

    let mut c = match res {
        Ok(c) => c,
        Err(why) => panic!("Could not create: {:?}", why),
    };

    b.iter(|| {
        test::black_box(c.exists(b"foo"));
    });
}

