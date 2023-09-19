mod test_utils;

use std::{
    fs::File,
    io::{BufRead, BufReader},
    ops::Add,
    path::Path,
};

use flexi_logger::{Cleanup, Criterion, Duplicate, FileSpec, Logger, Naming};
use glob::glob;
use log::*;

const COUNT: u8 = 4;

#[test]
fn test_rotate_naming_variants() {
    if let Some(value) = test_utils::dispatch(COUNT) {
        work(value)
    }
}

fn work(value: u8) {
    match value {
        0 => test_variant(Naming::Timestamps, Criterion::Size(800)),
        1 => test_variant(Naming::TimestampsDirect, Criterion::Size(800)),
        2 => test_variant(Naming::Numbers, Criterion::Size(800)),
        3 => test_variant(Naming::NumbersDirect, Criterion::Size(800)),
        COUNT..=u8::MAX => unreachable!("asAS"),
    }
}

fn test_variant(naming: Naming, criterion: Criterion) {
    let directory = test_utils::dir();

    std::thread::sleep(std::time::Duration::from_millis(500));

    test_utils::wait_for_start_of_second();

    let logger = Logger::try_with_str("trace")
        .unwrap()
        .log_to_file(FileSpec::default().directory(&directory))
        .format_for_files(flexi_logger::detailed_format)
        .format_for_stderr(flexi_logger::detailed_format)
        .duplicate_to_stderr(Duplicate::Info)
        .rotate(criterion, naming, Cleanup::Never)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

    info!("test trigger rotation",);
    let mut line_count = 1;

    for i in 0..45 {
        if i == 12 {
            logger.trigger_rotation().unwrap();
        }
        trace!("{}", 'a');
        line_count += 1;
    }

    verify_logs(&directory, line_count, 7);
}

fn verify_logs(directory: &Path, line_count: usize, file_count: usize) {
    // read all files
    let pattern = directory.display().to_string().add("/*");
    let globresults = match glob(&pattern) {
        Err(e) => panic!("Is this ({pattern}) really a directory? Listing failed with {e}",),
        Ok(globresults) => globresults,
    };
    let mut total_line_count = 0_usize;
    let mut total_file_count = 0_usize;
    for globresult in globresults.into_iter() {
        total_file_count += 1;
        let mut current_line_count = 0_usize;
        let pathbuf = globresult.unwrap_or_else(|e| panic!("Ups - error occured: {e}"));
        let f = File::open(&pathbuf)
            .unwrap_or_else(|e| panic!("Cannot open file {pathbuf:?} due to {e}"));
        let mut reader = BufReader::new(f);
        let mut buffer = String::new();
        while reader.read_line(&mut buffer).unwrap() > 0 {
            current_line_count += 1;
        }
        total_line_count += current_line_count;
    }

    assert_eq!(
        total_line_count, line_count,
        "wrong line count: {total_line_count} instead of {line_count}"
    );
    assert_eq!(
        total_file_count, file_count,
        "wrong file count: {total_file_count} instead of {file_count}"
    );
}
