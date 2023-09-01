mod test_utils;

use flexi_logger::{FileSpec, Logger, WriteMode};
use log::*;
use std::path::Path;

#[cfg(feature = "async")]
const COUNT: u8 = 3;
#[cfg(not(feature = "async"))]
const COUNT: u8 = 2;

#[test]
fn test_external_delete() {
    if let Some(value) = test_utils::dispatch(COUNT) {
        work(value)
    }
}

fn work(value: u8) {
    let mut logger = Logger::try_with_str("info").unwrap();
    let file_spec = FileSpec::default()
        .directory(self::test_utils::dir())
        .suppress_timestamp()
        .basename("myprog");
    let file_path = file_spec.as_pathbuf(None);
    logger = logger.log_to_file(file_spec);

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

    let logger = logger
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

    // write some log lines to initialize the file
    info!("XXX 1 AAA");
    info!("XXX 2 AAA");
    info!("XXX 3 AAA");

    // write log lines, and delete the log file intermittently
    for i in 0..100 {
        if i % 25 == 20 {
            logger.flush();
            std::thread::sleep(std::time::Duration::from_millis(100));
            let lines = count_lines(&file_path);
            match std::fs::remove_file(file_path.clone()) {
                Ok(()) => {
                    println!("Removed the log file {file_path:?}, which had {lines} lines");
                    logger.reopen_output().unwrap();
                }
                Err(e) => {
                    panic!("Cannot remove log file {file_path:?}, i = {i}, reason {e:?}")
                }
            }
        }
        info!("YYY {} AAA", i);
    }

    logger.flush();
    assert!(count_lines(&file_path) < 30, "wrong number of lines",);
}

fn count_lines(path: &Path) -> usize {
    match std::fs::read_to_string(path) {
        Ok(s) => s.lines().filter(|line| line.contains("AAA")).count(),
        Err(_e) => 0,
    }
}
