extern crate flexi_logger;
#[macro_use]
extern crate log;

use flexi_logger::{detailed_format, Logger};

#[test]
fn test_detailed_files_dscr() {
    let handle = Logger::with_str("info")
        .format(detailed_format)
        .log_to_file()
        .discriminant("foo")
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");
    handle.validate_logs(&[
        ("ERROR", "test_detailed_files_dscr", "error"),
        ("WARN", "test_detailed_files_dscr", "warning"),
        ("INFO", "test_detailed_files_dscr", "info"),
    ]);
}
