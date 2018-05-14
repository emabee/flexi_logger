extern crate flexi_logger;
#[macro_use]
extern crate log;

use flexi_logger::{detailed_format, Logger};

fn main() {
    Logger::with_str("info")
        .format(detailed_format)
        .print_message()
        .log_to_file()
        .rotate_over_size(1_000)
        .append()
        .suppress_timestamp()
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");
}
