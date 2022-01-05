mod test_utils;

#[cfg(feature = "external_rotation")]
use flexi_logger::{FileSpec, Logger, WriteMode};
#[cfg(feature = "external_rotation")]
use log::*;

use std::path::Path;

#[cfg(feature = "external_rotation")]
#[cfg(feature = "async")]
const COUNT: u8 = 3;
#[cfg(feature = "external_rotation")]
#[cfg(not(feature = "async"))]
const COUNT: u8 = 2;

#[cfg(feature = "external_rotation")]
#[test]
fn test_external_file_remover() {
    if let Some(value) = test_utils::dispatch(COUNT) {
        work(value)
    }
}

#[cfg(feature = "external_rotation")]
fn work(value: u8) {
    let mut logger = Logger::try_with_str("info").unwrap();
    let file_spec = FileSpec::default()
        .directory(self::test_utils::dir())
        .suppress_timestamp()
        .basename("myprog");
    logger = logger.log_to_file(file_spec.clone());

    // ToDo: test with all write modes, with and without rotation
    match value {
        0 => {
            logger = logger.write_mode(WriteMode::Direct);
        }
        1 => {
            logger = logger.write_mode(WriteMode::BufferAndFlush);
        }
        #[cfg(feature = "async")]
        2 => {
            logger = logger.write_mode(WriteMode::Async);
        }
        COUNT..=u8::MAX => {
            unreachable!("dtrtgfg")
        }
    };

    let _handle = logger
        .watch_external_rotations()
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    // write some log lines to initialize the file
    info!("XXX 1 AAA");
    info!("XXX 2 AAA");
    info!("XXX 3 AAA");

    let log_file = file_spec.as_pathbuf(None);
    // write log lines in a slow loop, and delete the log file intermittently
    for i in 1..200 {
        std::thread::sleep(std::time::Duration::from_millis(20));
        info!("YYY {} AAA", i);
        if i % 50 == 0 {
            let lines = count_lines(&log_file);
            match std::fs::remove_file(log_file.clone()) {
                Ok(()) => {
                    println!(
                        "Removed the log file {:?}, which had {} lines",
                        log_file, lines
                    )
                }
                Err(e) => {
                    panic!("Cannot remove log file {:?}, due to {:?}", log_file, e)
                }
            }
        }
    }
    assert_eq!(count_lines(&log_file), 49);
}

#[cfg(feature = "external_rotation")]
fn count_lines(path: &Path) -> usize {
    std::fs::read_to_string(path)
        .unwrap()
        .lines()
        .filter(|line| line.contains("AAA"))
        .count()
}
