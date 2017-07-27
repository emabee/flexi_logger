extern crate flexi_logger;
#[macro_use]
extern crate log;

use flexi_logger::{default_format, Logger};

#[test]
fn files_dir_rot() {
    Logger::with_str("info")
        .format(default_format)
        .log_to_file()
        .directory("log_files")
        .rotate_over_size(2000)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");
}
