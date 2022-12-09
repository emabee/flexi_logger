mod test_utils;

#[test]
#[cfg(feature = "textfilter")]
fn test_textfilter() {
    use flexi_logger::{default_format, FileSpec, LogSpecification, Logger};
    use log::*;

    let logspec = LogSpecification::parse("info/Hello").unwrap();
    let logger = Logger::with(logspec)
        .format(default_format)
        .print_message()
        .log_to_file(
            FileSpec::default()
                .directory(self::test_utils::dir())
                .suppress_timestamp(),
        )
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");

    error!("Hello, this is an error message");
    warn!("This is a warning! Hello!!");
    info!("Hello, this is an info message! Hello");
    debug!("Hello, this is a debug message - you must not see it!");
    trace!("Hello, this is a trace message - you must not see it!");

    logger.validate_logs(&[
        ("ERROR", "test_textfilter", "Hello, this"),
        ("WARN", "test_textfilter", "! Hello!!"),
        ("INFO", "test_textfilter", "! Hello"),
    ]);
}
