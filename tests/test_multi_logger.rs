#[macro_use]
extern crate log;

use flexi_logger::writers::{FileLogWriter, LogWriter};
use flexi_logger::{detailed_format, Logger, Record};

use std::io;
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
    let log_handle = Logger::with_str("info")
        .format(detailed_format)
        .print_message()
        .log_to_file()
        .add_writer("Sec", sec_writer)
        .add_writer("Alert", alert_logger())
        .start_reconfigurable()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

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

    // Verification:
    #[cfg_attr(rustfmt, rustfmt_skip)]
    log_handle.validate_logs(&[
        ("ERROR", "multi_logger", "a security-relevant alert and log message"),
        ("ERROR", "multi_logger", "another security-relevant alert and log message"),
        ("WARN", "multi_logger", "warning"),
        ("INFO", "multi_logger", "info"),
    ]);
    #[cfg_attr(rustfmt, rustfmt_skip)]
    sec_handle.validate_logs(&[
        ("ERROR", "multi_logger", "security-relevant error"),
        ("ERROR", "multi_logger", "a security-relevant alert"),
        ("ERROR", "multi_logger", "security-relevant alert and log message"),
        ("ERROR", "multi_logger", "another security-relevant alert"),
    ]);
}

struct SecWriter(Arc<FileLogWriter>);

impl SecWriter {
    pub fn new() -> (Box<SecWriter>, Arc<FileLogWriter>) {
        let a_flw = Arc::new(
            FileLogWriter::builder()
                .discriminant("Security")
                .suffix("seclog")
                .print_message()
                .instantiate()
                .unwrap(),
        );
        (Box::new(SecWriter(Arc::clone(&a_flw))), a_flw)
    }
}
impl LogWriter for SecWriter {
    fn write(&self, record: &Record) -> io::Result<()> {
        self.0.write(record)
    }
    fn flush(&self) -> io::Result<()> {
        self.0.flush()
    }
}

pub fn alert_logger() -> Box<FileLogWriter> {
    Box::new(
        FileLogWriter::builder()
            .discriminant("Alert")
            .suffix("alerts")
            .print_message()
            .instantiate()
            .unwrap(),
    )
}
