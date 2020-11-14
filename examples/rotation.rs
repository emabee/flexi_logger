use flexi_logger::{Age, Cleanup, Criterion, Logger, Naming};
use log::{debug, error, info, trace, warn};
use std::env;
use std::error::Error;

fn main() -> Result<(), Box<dyn Error>> {
    Logger::with_str("info")
        .directory(env::temp_dir())
        .log_to_file()
        .rotate(
            Criterion::Age(Age::Day), // Every day new log file is created.
            Naming::Timestamps,       // Each file has timestamp in it's name.
            Cleanup::KeepLogFiles(3), // Keep max of 3 log files.
        )
        .start()?;

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");

    Ok(())
}
