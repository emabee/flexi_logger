extern crate flexi_logger;
#[macro_use]
extern crate log;

use flexi_logger::{detailed_format, Logger};

#[test]
fn simple() {
    Logger::with_env_or_str("info, test_simple::mymod1=info, test_simple::mymod2=error")
        .format(detailed_format)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");

    mymod1::test_traces();
    mymod2::test_traces();
    println!("Done");
}

mod mymod1 {
    pub fn test_traces() {
        error!("This is an error message");
        warn!("This is a warning");
        info!("This is an info message");
        debug!("This is a debug message - you must not see it!");
        trace!("This is a trace message - you must not see it!");
    }
}
mod mymod2 {
    pub fn test_traces() {
        error!("This is an error message");
        warn!("This is a warning");
        info!("This is an info message");
        debug!("This is a debug message - you must not see it!");
        trace!("This is a trace message - you must not see it!");
    }
}
