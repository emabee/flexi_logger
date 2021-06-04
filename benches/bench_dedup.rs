#![feature(test)]
extern crate test;

#[cfg(feature = "dedup")]
#[bench]
fn b10_dedup(b: &mut test::Bencher) {
    use std::num::NonZeroUsize;

    use flexi_logger::{FileSpec, Logger};

    Logger::try_with_str("info")
        .unwrap()
        .log_to_file(FileSpec::default().directory("log_files"))
        .dedup(NonZeroUsize::new(2).unwrap())
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    b.iter(|| {
        for i in 0..100 {
            log::info!("{}", if i != 0 && i % 5 == 0 { "bar" } else { "foo" });
        }
    });
}
