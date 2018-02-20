extern crate flexi_logger;
#[macro_use]
extern crate log;

use flexi_logger::{detailed_format, Logger};

#[test]
fn test_additional_writers() {
    let handle = Logger::with_str("info")
        .format(detailed_format)
        .log_to_file()
        .add_writer("Sec", sec_writer)
        .add_writer("dummy", dummy_writer)
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");
    handle.validate_logs(&[("ERROR", "error"), ("WARN", "warning"), ("INFO", "info")]);
}
