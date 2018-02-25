extern crate flexi_logger;
#[macro_use]
extern crate log;

use flexi_logger::{opt_format, Logger};

#[test]
fn test_opt_files_dir_dscr() {
    let handle = Logger::with_str("info")
        .format(opt_format)
        .log_to_file()
        .directory("log_files")
        .discriminant("foo")
        .start_reconfigurable()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");
    handle.validate_logs(&[
        ("ERROR", "test_opt_files_dir_dscr", "error"),
        ("WARN", "test_opt_files_dir_dscr", "warning"),
        ("INFO", "test_opt_files_dir_dscr", "info"),
    ]);
}
