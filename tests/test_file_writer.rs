mod test_utils;

use flexi_logger::{detailed_format, opt_format, Cleanup, Criterion, FileSpec, Logger, Naming};
use log::*;

const COUNT: u8 = 8;

#[test]
fn test_write_modes() {
    if let Some(value) = test_utils::dispatch(COUNT) {
        work(value)
    }
}

fn work(value: u8) {
    let link_name = "link_to_log".to_string();
    let mut logger = Logger::try_with_str("info").unwrap();

    match value {
        0 => {
            logger = logger.log_to_file(
                FileSpec::default()
                    .directory(self::test_utils::dir())
                    .basename("to_foo_or_not_to_foo"),
            );
        }
        1 => {
            logger = logger
                .log_to_file(
                    FileSpec::default()
                        .suppress_timestamp()
                        .directory(self::test_utils::dir()),
                )
                .rotate(Criterion::Size(2000), Naming::Numbers, Cleanup::Never);
        }
        2 => {
            logger = logger
                .format(detailed_format)
                .log_to_file(
                    FileSpec::default()
                        .directory(self::test_utils::dir())
                        .use_timestamp(false),
                )
                .rotate(Criterion::Size(2000), Naming::Numbers, Cleanup::Never);
        }
        3 => {
            logger = logger
                .format(detailed_format)
                .log_to_file(
                    FileSpec::default()
                        .suppress_timestamp()
                        .directory(self::test_utils::dir()),
                )
                .rotate(Criterion::Size(2000), Naming::Numbers, Cleanup::Never);
        }
        4 => {
            logger = logger
                .format(opt_format)
                .log_to_file(
                    FileSpec::default()
                        .suppress_timestamp()
                        .directory(self::test_utils::dir())
                        .discriminant("foo".to_string()),
                )
                .rotate(Criterion::Size(2000), Naming::Numbers, Cleanup::Never)
                .create_symlink(link_name.clone());
        }
        5 => {
            logger = logger.format(opt_format).log_to_file(
                FileSpec::default()
                    .suppress_timestamp()
                    .directory(self::test_utils::dir())
                    .discriminant("foo"),
            );
        }
        6 => {
            logger = logger.format(opt_format).log_to_file(
                FileSpec::default()
                    .directory(self::test_utils::dir())
                    .suppress_basename(),
            );
        }
        7 => {
            logger = logger.format(opt_format).log_to_file(
                FileSpec::default()
                    .directory(self::test_utils::dir())
                    .suppress_basename()
                    .discriminant("foo"),
            );
        }
        COUNT..=u8::MAX => {
            unreachable!("dtrtgfg")
        }
    };

    let handle = logger
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");

    handle.validate_logs(&[
        ("ERROR", "test_file_writer", "error"),
        ("WARN", "test_file_writer", "warning"),
        ("INFO", "test_file_writer", "info"),
    ]);

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
