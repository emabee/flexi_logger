use flexi_logger::Logger;
use log::*;

#[test]
fn test_mods() {
    Logger::try_with_str("trace")
        .unwrap()
        .log_to_stdout()
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message");
    trace!("This is a trace message");
}
