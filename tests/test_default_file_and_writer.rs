use flexi_logger::writers::{FileLogWriter, LogWriter};
use flexi_logger::{detailed_format, FileSpec, Logger};
use log::*;
mod test_utils;

#[test]
fn test_default_file_and_writer() {
    let file_spec_bar = FileSpec::default()
        .directory(self::test_utils::dir())
        .suppress_timestamp()
        .discriminant("bar");
    let file_spec_foo = file_spec_bar.clone().discriminant("foo");
    let bar_writer = FileLogWriter::builder(file_spec_bar.clone())
        .format(detailed_format)
        .try_build()
        .unwrap();

    {
        let handle = Logger::try_with_str("info")
            .unwrap()
            .log_to_file_and_writer(file_spec_foo, Box::new(bar_writer))
            .format(detailed_format)
            .start()
            .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

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
    }

    let bar_writer = FileLogWriter::builder(file_spec_bar)
        .format(detailed_format)
        .append()
        .try_build()
        .unwrap();
    bar_writer.validate_logs(&[
        ("ERROR", "test_default_file_and_writer", "error"),
        ("WARN", "test_default_file_and_writer", "warning"),
        ("INFO", "test_default_file_and_writer", "info"),
    ]);
}
