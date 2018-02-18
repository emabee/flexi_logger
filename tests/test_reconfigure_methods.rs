extern crate flexi_logger;
#[macro_use]
extern crate log;

use flexi_logger::{detailed_format, Logger};

#[test]
fn test_reconfigure_methods() {
    let mut handle = Logger::with_str("info")
        .format(detailed_format)
        .o_log_to_file(true)
        .o_rotate_over_size(Some(2000))
        .start_reconfigurable()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");

    handle.parse_new_spec("error");
    error!("This is an error message");
    warn!("This is a warning - you must not see it!");
    info!("This is an info message - you must not see it!");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");

    handle.parse_new_spec("trace");
    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message");
    trace!("This is a trace message");

    handle.validate_logs(&[
        ("ERROR", "error"),
        ("WARN", "warning"),
        ("INFO", "info"),
        //
        ("ERROR", "error"),
        //
        ("ERROR", "error"),
        ("WARN", "warning"),
        ("INFO", "info"),
        ("DEBUG", "debug"),
        ("TRACE", "trace"),
    ]);
}
