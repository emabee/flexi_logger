use flexi_logger::{detailed_format, Logger};
use log::{debug, error, info, trace, warn};
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    Logger::with_str("info").format(detailed_format).start()?;

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");

    // example log entry:
    // [2020-10-04 10:19:55.966101 +02:00] ERROR [detailed_log_format] examples/detailed_log_format.rs:8: This is an error message

    Ok(())
}
