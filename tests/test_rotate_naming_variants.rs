mod test_utils;

use flexi_logger::{Age, Cleanup, Criterion, Duplicate, FileSpec, Logger, Naming};
use glob::glob;
use log::*;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    ops::Add,
    path::Path,
    time::{Duration, Instant},
};

const COUNT: u8 = 13;

#[test]
fn test_rotate_naming_variants() {
    // work(6)
    if let Some(value) = test_utils::dispatch(COUNT) {
        work(value)
    }
}

fn work(value: u8) {
    let directory = test_utils::dir();

    match value {
        0 => test_variant(
            FileSpec::default().directory(directory.join(value.to_string())),
            Naming::Timestamps,
            Criterion::AgeOrSize(Age::Second, 200),
        ),
        1 => test_variant(
            FileSpec::default().directory(directory.join(value.to_string())),
            Naming::TimestampsDirect,
            Criterion::AgeOrSize(Age::Second, 200),
        ),
        2 => test_variant(
            FileSpec::default().directory(directory.join(value.to_string())),
            Naming::Numbers,
            Criterion::AgeOrSize(Age::Second, 200),
        ),
        3 => test_variant(
            FileSpec::default().directory(directory.join(value.to_string())),
            Naming::NumbersDirect,
            Criterion::AgeOrSize(Age::Second, 200),
        ),

        4 => test_variant(
            FileSpec::default().directory(directory.join(value.to_string())),
            Naming::Timestamps,
            Criterion::Age(Age::Second),
        ),
        5 => test_variant(
            FileSpec::default().directory(directory.join(value.to_string())),
            Naming::TimestampsDirect,
            Criterion::Age(Age::Second),
        ),
        6 => test_variant(
            FileSpec::default().directory(directory.join(value.to_string())),
            Naming::TimestampsCustomFormat {
                current_infix: Some("myCURRENT"),
                format: "%Y-%m-%d",
            },
            Criterion::Age(Age::Second),
        ),
        7 => test_variant(
            FileSpec::default().directory(directory.join(value.to_string())),
            Naming::TimestampsCustomFormat {
                current_infix: Some(""),
                format: "%Y-%m-%d_%H-%M-%S",
            },
            Criterion::Age(Age::Second),
        ),
        8 => test_variant(
            FileSpec::default().directory(directory.join(value.to_string())),
            Naming::Numbers,
            Criterion::Age(Age::Second),
        ),
        9 => test_variant(
            FileSpec::default().directory(directory.join(value.to_string())),
            Naming::NumbersDirect,
            Criterion::Age(Age::Second),
        ),
        10 => test_variant(
            FileSpec::default()
                .directory(directory.join(value.to_string()))
                .suppress_basename()
                .suppress_timestamp()
                .o_discriminant(Option::<String>::None),
            Naming::NumbersDirect,
            Criterion::Age(Age::Second),
        ),
        11 => test_variant(
            FileSpec::default()
                .directory(directory.join(value.to_string()))
                .suppress_basename()
                .suppress_timestamp()
                .o_discriminant(Option::<String>::None),
            Naming::Timestamps,
            Criterion::Age(Age::Second),
        ),
        12 => test_variant(
            FileSpec::default()
                .directory(directory.join(value.to_string()))
                .suppress_basename()
                .suppress_timestamp()
                .o_discriminant(Option::<String>::None),
            Naming::Numbers,
            Criterion::Age(Age::Second),
        ),
        COUNT..=u8::MAX => unreachable!("Wrong dispatch"),
    }
}

fn test_variant(file_spec: FileSpec, naming: Naming, criterion: Criterion) {
    let directory = file_spec.used_directory();
    let _logger = Logger::try_with_str("trace")
        .unwrap()
        .log_to_file(file_spec)
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
    let start = Instant::now();
    let max_runtime = Duration::from_millis(3_000);
    let sleep_time = Duration::from_millis(10);
    while Instant::now() - start < max_runtime {
        trace!("{}", 'a');
        line_count += 1;
        std::thread::sleep(sleep_time);
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
