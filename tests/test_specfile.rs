// #![feature(test)]
// extern crate test;

extern crate flexi_logger;
#[macro_use]
extern crate log;

use flexi_logger::Logger;

#[test]
fn test_specfile() {
    Logger::with_str("off")
        .log_to_file()
        .directory("log_files")
        .start_with_specfile("./tests/logspec.toml")
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    error!("This is an error message");
    warn!("This is a warning");
    warn!("This warning is filtered out");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");
}
