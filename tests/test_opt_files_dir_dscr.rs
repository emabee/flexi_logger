extern crate flexi_logger;
#[macro_use]
extern crate log;

use flexi_logger::{opt_format, Logger};

#[test]
fn files_dir_dscr() {
    assert_eq!((),
               Logger::with_str("info")
                   .format(opt_format)
                   .log_to_file()
                   .directory("log_files")
                   .discriminant("foo")
                   .start()
                   .unwrap());

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");
}
