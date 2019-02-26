extern crate flexi_logger;
#[macro_use]
extern crate log;

use flexi_logger::Logger;

#[test]
fn you_must_not_see_anything() {
    Logger::with_str("info").do_not_log().start().unwrap();

    error!("This is an error message - you must not see it!");
    warn!("This is a warning - you must not see it!");
    info!("This is an info message - you must not see it!");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");
}
