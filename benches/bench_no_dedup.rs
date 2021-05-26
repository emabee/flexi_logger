#![feature(test)]
extern crate test;

use std::num::NonZeroUsize;

use flexi_logger::{FileSpec, Logger};
use test::Bencher;

#[bench]
fn b10_no_dedup(b: &mut Bencher) {
    Logger::with_str("info")
        .log_to_file(FileSpec::default().directory("log_files"))
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    b.iter(|| {
        for i in 0..100 {
            log::info!("{}", if i != 0 && i % 5 == 0 { "bar" } else { "foo" });
        }
    });
}
