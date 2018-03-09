extern crate flexi_logger;
#[macro_use]
extern crate log;

use flexi_logger::{detailed_format, Logger};
use std::{thread, time};

#[test]
fn test_specfile() {
    Logger::with_str("info")
        .format(detailed_format)
        .start_with_specfile("./tests/logspec.toml")
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    let wait_a_sec = time::Duration::from_millis(1000);
    loop {
        thread::sleep(wait_a_sec);
        error!("This is an error message");
        warn!("This is a warning");
        info!("This is an info message");
        debug!("This is a debug message");
        trace!("This is a trace message");
    }
}
