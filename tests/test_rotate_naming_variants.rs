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
use regex::Regex;

const COUNT: u8 = 11;

#[test]
fn test_rotate_naming_variants() {
    // work(10);
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
        10 => test_issue_176(),
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

fn test_issue_176() {
    let directory = test_utils::dir();
    test_utils::wait_for_start_of_second();

    let file_spec = FileSpec::default()
        .directory(&directory)
        .suppress_basename()
        .suppress_timestamp()
        .o_suffix(Option::<String>::None)
        .suffix("log");
    let _logger = Logger::try_with_str("trace")
        .unwrap()
        .log_to_file(file_spec)
        .format_for_files(flexi_logger::detailed_format)
        .format_for_stderr(flexi_logger::detailed_format)
        .duplicate_to_stderr(Duplicate::Info)
        .rotate(
            flexi_logger::Criterion::Size(1),
            flexi_logger::Naming::TimestampsCustomFormat {
                current_infix: Some("rCURRENT"),
                format: "%Y-%m-%d_%H-%M-%S",
            },
            Cleanup::Never,
        )
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

    info!(
        "test correct rotation by 0 with Naming::TimestampsCustomFormat(without infix underscore)"
    );
    info!(
        "test correct rotation by 1 with Naming::TimestampsCustomFormat(without infix underscore)"
    );
    info!(
        "test correct rotation by 2 with Naming::TimestampsCustomFormat(without infix underscore)"
    );

    let pattern = directory.display().to_string().add("/*");
    let globresults = match glob(&pattern) {
        Err(e) => panic!("Is this ({pattern}) really a directory? Listing failed with {e}",),
        Ok(globresults) => globresults,
    };
    let re = Regex::new(r"^(rCURRENT|\d{4}-\d{2}-\d{2}_\d{2}-\d{2}-\d{2})\b").unwrap();
    for globresult in globresults.into_iter() {
        let mut total_line_count = 0_usize;
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
        assert_eq!(
            total_line_count, 1,
            "wrong line count: {total_line_count} instead of 1"
        );

        let n = pathbuf.file_name().unwrap().to_str().unwrap();
        assert!(re.is_match(&n), "Log file {} does not match the required format: {}", n, re.as_str());
    }
}
