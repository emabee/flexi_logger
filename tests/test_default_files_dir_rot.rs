extern crate flexi_logger;
#[macro_use]
extern crate log;

use flexi_logger::{default_format, Logger};

#[test]
fn files_dir_rot() {
    assert_eq!((),
               Logger::with_str("info")
                   .format(default_format)
                   .log_to_file()
                   .directory("log_files")
                   .rotate_over_size(2000)
                   .start()
                   .unwrap());

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");
}
