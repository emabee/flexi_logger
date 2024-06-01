mod test_utils;

use flexi_logger::{opt_format, FileSpec, Logger, WriteMode};
use log::*;

const COUNT: u8 = 12;

#[test]
fn test_write_modes() {
    if let Some(value) = test_utils::dispatch(COUNT) {
        work(value)
    }
}

fn work(value: u8) {
    let logger = Logger::try_with_str("info").unwrap().format(opt_format);

    let logger = match value {
        0 => {
            println!("stdout, direct");
            logger.log_to_stdout().write_mode(WriteMode::Direct)
        }
        1 => {
            println!("stdout, buffer+flush");
            logger.log_to_stdout().write_mode(WriteMode::BufferAndFlush)
        }
        2 => {
            #[cfg(feature = "async")]
            {
                println!("stdout, async");
                logger.log_to_stdout().write_mode(WriteMode::Async)
            }
            #[cfg(not(feature = "async"))]
            {
                println!("!!! nothing done !!!");
                return;
            }
        }
        3 => {
            println!("stdout, buffer no flush");
            logger
                .log_to_stdout()
                .write_mode(WriteMode::BufferDontFlush)
        }
        4 => {
            println!("stderr, direct");
            logger.log_to_stderr().write_mode(WriteMode::Direct)
        }
        5 => {
            println!("stderr, buffer+flush");
            logger.log_to_stderr().write_mode(WriteMode::BufferAndFlush)
        }
        6 => {
            #[cfg(feature = "async")]
            {
                println!("stderr, async");
                logger.log_to_stderr().write_mode(WriteMode::Async)
            }
            #[cfg(not(feature = "async"))]
            {
                println!("!!! nothing done !!!");
                return;
            }
        }
        7 => {
            println!("stderr, buffer no flush");
            logger
                .log_to_stderr()
                .write_mode(WriteMode::BufferDontFlush)
        }

        8 => {
            println!("file, direct");
            logger
                .log_to_file(
                    FileSpec::default()
                        .suppress_timestamp()
                        .directory(self::test_utils::dir()),
                )
                .write_mode(WriteMode::Direct)
        }
        9 => {
            println!("file, buffer+flush");
            logger
                .log_to_file(
                    FileSpec::default()
                        .suppress_timestamp()
                        .directory(self::test_utils::dir()),
                )
                .write_mode(WriteMode::BufferAndFlush)
        }
        10 => {
            #[cfg(feature = "async")]
            {
                println!("file, async");
                logger
                    .log_to_file(
                        FileSpec::default()
                            .suppress_timestamp()
                            .directory(self::test_utils::dir()),
                    )
                    .write_mode(WriteMode::Async)
            }
            #[cfg(not(feature = "async"))]
            {
                println!("!!! nothing done !!!");
                return;
            }
        }
        11 => {
            println!("file, buffer no flush");
            logger
                .log_to_file(
                    FileSpec::default()
                        .suppress_timestamp()
                        .directory(self::test_utils::dir()),
                )
                .write_mode(WriteMode::BufferDontFlush)
        }
        COUNT..=u8::MAX => {
            unreachable!("got unexpected value {}", value)
        }
    };

    let handle = logger.start().unwrap_or_else(|e| panic!("{e}, {e:?}"));

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");

    handle.validate_logs(&[
        ("ERROR", "test_write_mode", "error"),
        ("WARN", "test_write_mode", "warning"),
        ("INFO", "test_write_mode", "info"),
    ]);
}
