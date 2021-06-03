use flexi_logger::writers::{FileLogWriter, LogWriter};
use flexi_logger::{detailed_format, FileSpec, Logger};
use log::*;

#[test]
fn test_default_file_and_writer() {
    let w = FileLogWriter::builder(FileSpec::default().discriminant("bar"))
        .format(detailed_format)
        .try_build()
        .unwrap();

    let handle = Logger::try_with_str("info")
        .unwrap()
        .log_to_file_and_writer(FileSpec::default().discriminant("foo"), Box::new(w))
        .format(detailed_format)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");

    handle.validate_logs(&[
        ("ERROR", "test_default_file_and_writer", "error"),
        ("WARN", "test_default_file_and_writer", "warning"),
        ("INFO", "test_default_file_and_writer", "info"),
    ]);

    let w = FileLogWriter::builder(FileSpec::default().discriminant("bar"))
        .format(detailed_format)
        .append()
        .try_build()
        .unwrap();
    w.validate_logs(&[
        ("ERROR", "test_default_file_and_writer", "error"),
        ("WARN", "test_default_file_and_writer", "warning"),
        ("INFO", "test_default_file_and_writer", "info"),
    ]);
}
