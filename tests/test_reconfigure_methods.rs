extern crate flexi_logger;
#[macro_use]
extern crate log;

use flexi_logger::{detailed_format, Logger};

#[test]
fn test_reconfigure_methods() {
    assert_eq!((),
               Logger::with_str("info")
                   .format(detailed_format)
                   .o_log_to_file(true)
                   .o_rotate_over_size(Some(2000))
                   .start()
                   .unwrap());

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");
}
