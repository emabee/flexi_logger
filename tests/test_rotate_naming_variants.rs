mod test_utils;

use std::{
    fs::File,
    io::{BufRead, BufReader},
    ops::Add,
    path::Path,
};

use chrono::Local;
use flexi_logger::{Age, Cleanup, Criterion, Duplicate, FileSpec, Logger, Naming};
use glob::glob;
use log::*;

const COUNT: u8 = 10;

#[test]
fn test_rotate_naming_variants() {
    // work(6)
    if let Some(value) = test_utils::dispatch(COUNT) {
        work(value)
    }
}

fn work(value: u8) {
    match value {
        0 => test_variant(Naming::Timestamps, Criterion::AgeOrSize(Age::Second, 200)),
        1 => test_variant(
            Naming::TimestampsDirect,
            Criterion::AgeOrSize(Age::Second, 200),
        ),
        2 => test_variant(Naming::Numbers, Criterion::AgeOrSize(Age::Second, 200)),
        3 => test_variant(
            Naming::NumbersDirect,
            Criterion::AgeOrSize(Age::Second, 200),
        ),

        4 => test_variant(Naming::Timestamps, Criterion::Age(Age::Second)),
        5 => test_variant(Naming::TimestampsDirect, Criterion::Age(Age::Second)),
        6 => test_variant(
            Naming::TimestampsCustomFormat {
                current_infix: Some("myCURRENT"),
                format: "%Y-%m-%d",
            },
            Criterion::Age(Age::Second),
        ),
        7 => test_variant(
            Naming::TimestampsCustomFormat {
                current_infix: Some(""),
                format: "%Y-%m-%d_%H-%M-%S",
            },
            Criterion::Age(Age::Second),
        ),
        8 => test_variant(Naming::Numbers, Criterion::Age(Age::Second)),
        9 => test_variant(Naming::NumbersDirect, Criterion::Age(Age::Second)),
        COUNT..=u8::MAX => unreachable!("Wrong dispatch"),
    }
}

fn test_variant(naming: Naming, criterion: Criterion) {
    let directory = test_utils::dir();

    test_utils::wait_for_start_of_second();

    let _logger = Logger::try_with_str("trace")
        .unwrap()
        .log_to_file(FileSpec::default().directory(&directory))
        .format_for_files(flexi_logger::detailed_format)
        .format_for_stderr(flexi_logger::detailed_format)
        .duplicate_to_stderr(Duplicate::Info)
        .rotate(criterion, naming, Cleanup::Never)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

    info!(
        "test correct rotation by {} with Naming::{naming:?} ",
        match criterion {
            Criterion::Age(_) => "age",
            Criterion::AgeOrSize(_, _) => "age or size",
            Criterion::Size(_) => "size",
        }
    );
    let mut line_count = 1;
    let start = Local::now();
    let duration = chrono::Duration::from_std(std::time::Duration::from_secs(10)).unwrap();
    while Local::now() - start < duration {
        trace!("{}", 'a');
        line_count += 1;
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    verify_logs(&directory, line_count);
}

fn verify_logs(directory: &Path, count: usize) {
    // read all files
    let pattern = directory.display().to_string().add("/*");
    let globresults = match glob(&pattern) {
        Err(e) => panic!("Is this ({pattern}) really a directory? Listing failed with {e}",),
        Ok(globresults) => globresults,
    };
    let mut total_line_count = 0_usize;
    for globresult in globresults.into_iter() {
        let mut line_count = 0_usize;
        let pathbuf = globresult.unwrap_or_else(|e| panic!("Ups - error occured: {e}"));
        let f = File::open(&pathbuf)
            .unwrap_or_else(|e| panic!("Cannot open file {pathbuf:?} due to {e}"));
        let mut reader = BufReader::new(f);
        let mut buffer = String::new();
        while reader.read_line(&mut buffer).unwrap() > 0 {
            line_count += 1;
        }
        total_line_count += line_count;
    }

    assert_eq!(
        total_line_count, count,
        "wrong line count: {total_line_count} instead of {count}"
    );
}
