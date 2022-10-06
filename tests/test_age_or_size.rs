mod test_utils;

use flexi_logger::{Age, Cleanup, Criterion, Duplicate, FileSpec, Logger, Naming};
use glob::glob;
use log::*;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::ops::Add;
use std::path::Path;

#[test]
fn test_age_or_size() {
    let directory = test_utils::dir();
    Logger::try_with_str("trace")
        .unwrap()
        .log_to_file(FileSpec::default().directory(&directory))
        .duplicate_to_stderr(Duplicate::Info)
        .rotate(
            Criterion::AgeOrSize(Age::Second, 75),
            Naming::Numbers,
            Cleanup::Never,
        )
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
    // info!("test correct rotation by age or size");

    write_log_lines();

    verify_logs(&directory);
}

fn write_log_lines() {
    trace!("{}", 'A');
    // wait to enforce a rotation
    std::thread::sleep(std::time::Duration::from_millis(1100));

    // Fill first three files by size
    trace!("{}", 'a');
    trace!("{}", 'b');
    trace!("{}", 'c');

    trace!("{}", 'd');
    trace!("{}", 'e');
    trace!("{}", 'f');

    trace!("{}", 'g');
    trace!("{}", 'h');
    trace!("{}", 'i');

    trace!("{}", 'j');

    // now wait to enforce a rotation with a smaller file
    std::thread::sleep(std::time::Duration::from_secs(2));
    trace!("{}", 'k');

    // now wait to enforce a rotation with a smaller file
    std::thread::sleep(std::time::Duration::from_secs(2));
    trace!("{}", 'l');

    // then again fill a file by size
    trace!("{}", 'm');
    trace!("{}", 'n');

    // and do the final rotation:
    trace!("{}", 'o');
}

fn verify_logs(directory: &Path) {
    let mut error_detected = false;
    let expected_line_counts = [1, 3, 3, 3, 1, 1, 3, 1];
    // read all files
    let pattern = directory.display().to_string().add("/*");
    let globresults = match glob(&pattern) {
        Err(e) => panic!(
            "Is this ({}) really a directory? Listing failed with {}",
            pattern, e
        ),
        Ok(globresults) => globresults,
    };
    let mut no_of_log_files = 0;
    let mut total_line_count = 0_usize;
    for (index, globresult) in globresults.into_iter().enumerate() {
        let mut line_count = 0_usize;
        let pathbuf = globresult.unwrap_or_else(|e| panic!("Ups - error occured: {}", e));
        let f = File::open(&pathbuf)
            .unwrap_or_else(|e| panic!("Cannot open file {:?} due to {}", pathbuf, e));
        no_of_log_files += 1;
        let mut reader = BufReader::new(f);
        let mut buffer = String::new();
        while reader.read_line(&mut buffer).unwrap() > 0 {
            line_count += 1;
        }
        println!("file {:?}:\n{}", pathbuf, buffer);
        if line_count != expected_line_counts[index] {
            error_detected = true;
        }
        total_line_count += line_count;
    }

    if no_of_log_files != 8 {
        println!("wrong file count: {} instead of 8", no_of_log_files);
        error_detected = true;
    }
    if total_line_count != 16 {
        println!("wrong line count: {} instead of 16", total_line_count);
        error_detected = true;
    };

    assert!(!error_detected);
}
