mod test_utils;

use flexi_logger::{
    detailed_format,
    writers::{FileLogWriter, LogWriter},
    DeferredNow, FileSpec, Logger,
};
use log::*;
use std::sync::Arc;

#[macro_use]
mod macros {
    #[macro_export]
    macro_rules! sec_alert_error {
        ($($arg:tt)*) => (
            error!(target: "{Sec,Alert,_Default}", $($arg)*);
        )
    }
}

#[test]
fn test() {
    // more complex just to support validation:
    let (sec_writer, sec_handle) = SecWriter::new();
    let logger = Logger::try_with_str("info, fantasy = trace")
        .unwrap()
        .format(detailed_format)
        .print_message()
        .log_to_file(
            FileSpec::default()
                .suppress_timestamp()
                .directory(self::test_utils::dir()),
        )
        .add_writer("Sec", sec_writer)
        .add_writer("Alert", alert_logger())
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

    // Explicitly send logs to different loggers
    error!(target : "{Sec}", "This is a security-relevant error message");
    error!(target : "{Sec,Alert}", "This is a security-relevant alert message");
    error!(target : "{Sec,Alert,_Default}", "This is a security-relevant alert and log message");
    error!(target : "{Alert}", "This is an alert");

    // Nicer: use explicit macros
    sec_alert_error!("This is another security-relevant alert and log message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");

    trace!(target: "phantasia", "this is a trace you should not see");
    trace!(target: "fantasy", "this is a trace you should see");

    // Switching off logging has no effect on non-default targets
    logger.parse_new_spec("Off").unwrap();
    sec_alert_error!("This is a further security-relevant alert and log message");

    // Verification:
    #[rustfmt::skip]
    logger.validate_logs(&[
        ("ERROR", "multi_logger", "a security-relevant alert and log message"),
        ("ERROR", "multi_logger", "another security-relevant alert and log message"),
        ("WARN", "multi_logger", "warning"),
        ("INFO", "multi_logger", "info"),
        ("TRACE", "multi_logger", "this is a trace you should see"),
    ]);
    #[rustfmt::skip]
    sec_handle.validate_logs(&[
        ("ERROR", "multi_logger", "security-relevant error"),
        ("ERROR", "multi_logger", "a security-relevant alert"),
        ("ERROR", "multi_logger", "security-relevant alert and log message"),
        ("ERROR", "multi_logger", "another security-relevant alert"),
        ("ERROR", "multi_logger", "a further security-relevant alert"),
    ]);
}

struct SecWriter(Arc<FileLogWriter>);

impl SecWriter {
    pub fn new() -> (Box<SecWriter>, Arc<FileLogWriter>) {
        let a_flw = Arc::new(
            FileLogWriter::builder(
                FileSpec::default()
                    .directory(self::test_utils::dir())
                    .discriminant("Security")
                    .suffix("seclog"),
            )
            .print_message()
            .try_build()
            .unwrap(),
        );
        (Box::new(SecWriter(Arc::clone(&a_flw))), a_flw)
    }
}
impl LogWriter for SecWriter {
    fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        self.0.write(now, record)
    }
    fn flush(&self) -> std::io::Result<()> {
        self.0.flush()
    }
    fn max_log_level(&self) -> log::LevelFilter {
        log::LevelFilter::Error
    }
}

pub fn alert_logger() -> Box<FileLogWriter> {
    Box::new(
        FileLogWriter::builder(
            FileSpec::default()
                .directory(self::test_utils::dir())
                .discriminant("Alert")
                .suffix("alerts"),
        )
        .print_message()
        .try_build()
        .unwrap(),
    )
}
