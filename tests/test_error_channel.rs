mod test_utils;

#[cfg(feature = "async")]
const COUNT: u8 = 4;

#[cfg(feature = "async")]
#[test]
fn test_error_channels() {
    if let Some(value) = test_utils::dispatch(COUNT) {
        work(value)
    }
}

#[cfg(feature = "async")]
fn work(value: u8) {
    use flexi_logger::{ErrorChannel, FileSpec, Logger, WriteMode};
    use log::*;
    use std::{
        fs::File,
        io::{BufRead, BufReader},
    };

    let mut logger = Logger::try_with_str("info")
        .unwrap()
        .log_to_file(FileSpec::default().directory(&test_utils::dir()));

    {
        logger = logger.write_mode(WriteMode::Async);
    }
    match value {
        0 => {
            logger = logger.error_channel(ErrorChannel::StdErr);
        }
        1 => {
            logger = logger.error_channel(ErrorChannel::StdOut);
        }
        2 => {
            logger = logger.error_channel(ErrorChannel::File(test_utils::file(
                "flexi_logger_error_channel.err",
            )));
        }
        3 => {
            logger = logger.error_channel(ErrorChannel::DevNull);
        }
        COUNT..=u8::MAX => {
            unreachable!("djdjfäfdl")
        }
    };

    {
        let _logger_handle = logger
            .start()
            .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
    }

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");

    if value == 2 {
        std::thread::sleep(std::time::Duration::from_millis(200));
        assert_eq!(
            BufReader::new(File::open(test_utils::file("flexi_logger_error_channel.err")).unwrap())
                .lines()
                .count(),
            6 // two lines per failing error, warn, or info call
        );
    }
}
