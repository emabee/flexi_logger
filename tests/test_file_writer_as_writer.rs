mod test_utils;

use flexi_logger::{writers::FileLogWriter, FileSpec, Logger};
use log::*;

const COUNT: u8 = 3;

#[test]
fn test_write_modes() {
    if let Some(value) = test_utils::dispatch(COUNT) {
        work(value)
    }
}

fn work(value: u8) {
    let link_name = "link_to_log".to_string();
    let mut logger = Logger::try_with_str("trace").unwrap();
    let flwb = FileLogWriter::builder(
        FileSpec::default()
            .directory(self::test_utils::dir())
            .basename("to_foo_or_not_to_foo"),
    );
    match value {
        0 => {
            logger = logger.log_to_writer(Box::new(
                flwb.max_level(LevelFilter::Debug).try_build().unwrap(),
            ));
        }
        1 => {
            logger = logger.log_to_writer(Box::new(
                flwb.max_level(LevelFilter::Trace).try_build().unwrap(),
            ));
        }
        2 => {
            logger = logger.log_to_writer(Box::new(flwb.try_build().unwrap()));
        }
        COUNT..=u8::MAX => {
            unreachable!()
        }
    };

    let handle = logger
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message");
    trace!("This is a trace message");

    match value {
        0 => {
            handle.validate_logs(&[
                ("ERROR", "test_file_writer", "error"),
                ("WARN", "test_file_writer", "warning"),
                ("INFO", "test_file_writer", "info"),
                ("DEBUG", "test_file_writer", "debug"),
            ]);
        }
        1 => {
            handle.validate_logs(&[
                ("ERROR", "test_file_writer", "error"),
                ("WARN", "test_file_writer", "warning"),
                ("INFO", "test_file_writer", "info"),
                ("DEBUG", "test_file_writer", "debug"),
                ("TRACE", "test_file_writer", "trace"),
            ]);
        }
        2 => {
            handle.validate_logs(&[
                ("ERROR", "test_file_writer", "error"),
                ("WARN", "test_file_writer", "warning"),
                ("INFO", "test_file_writer", "info"),
                ("DEBUG", "test_file_writer", "debug"),
                ("TRACE", "test_file_writer", "trace"),
            ]);
        }
        COUNT..=u8::MAX => {
            unreachable!()
        }
    }

    if value == 4 {
        self::platform::check_link(&link_name);
    }
}

mod platform {
    #[cfg(target_family = "unix")]
    pub fn check_link(link_name: &str) {
        match std::fs::symlink_metadata(link_name) {
            Err(e) => panic!("error with symlink: {e}"),
            Ok(metadata) => assert!(metadata.file_type().is_symlink(), "not a symlink"),
        }
    }

    #[cfg(not(target_family = "unix"))]
    pub fn check_link(_: &str) {}
}
