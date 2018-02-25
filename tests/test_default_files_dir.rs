// #![feature(test)]
// extern crate test;

extern crate flexi_logger;
#[macro_use]
extern crate log;

use flexi_logger::Logger;

#[test]
fn test_default_files_dir() {
    let handle = Logger::with_str("info")
        .log_to_file()
        .directory("log_files")
        .start_reconfigurable()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");
    handle.validate_logs(&[
        ("ERROR", "test_default_files_dir", "error"),
        ("WARN", "test_default_files_dir", "warning"),
        ("INFO", "test_default_files_dir", "info"),
    ]);
}
