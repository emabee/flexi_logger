mod test_utils;

use std::{
    fs::File,
    io::{BufRead, BufReader},
};

use flexi_logger::{Cleanup, Criterion, FileSpec, LogfileSelector, Logger, Naming};
use log::*;

const COUNT: u8 = 2;

#[test]
fn test_write_modes() {
    if let Some(value) = test_utils::dispatch(COUNT) {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        work(value)
    }
}

fn work(value: u8) {
    let directory = test_utils::dir();
    let file_spec = FileSpec::default()
        .directory(directory)
        .o_suffix(match value {
            0 => Some("log".to_string()),
            1 => None,
            COUNT..=u8::MAX => {
                unreachable!("got unexpected value {}", value)
            }
        });

    let logger = Logger::try_with_str("debug")
        .unwrap()
        .log_to_file(file_spec)
        .rotate(Criterion::Size(100), Naming::Timestamps, Cleanup::Never)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

    for i in 0..100 {
        error!("This is error message {i}");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    let mut contents = String::new();

    assert_eq!(
        100,
        logger
            .existing_log_files(&LogfileSelector::default().with_r_current())
            .unwrap()
            .into_iter()
            .filter(|pb| {
                let extension = pb.extension().map(|s| s.to_string_lossy().into_owned());
                match value {
                    0 => Some("log".to_string()) == extension,
                    1 => extension.is_none() || extension.unwrap().starts_with("restart"),
                    COUNT..=u8::MAX => {
                        unreachable!("got unexpected value {}", value)
                    }
                }
            })
            .map(|path| {
                let mut buf_reader = BufReader::new(File::open(path).unwrap());
                let mut line_count = 0;
                while buf_reader.read_line(&mut contents).unwrap() > 0 {
                    line_count += 1;
                }
                line_count
            })
            .sum::<u32>()
    );
}
