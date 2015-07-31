extern crate flexi_logger;

#[macro_use]
extern crate log;

use flexi_logger::{init,LogConfig};

#[test]
fn test_envlogger_style() {
    init(
        LogConfig::new(),
        Some("info".to_string())
    ).unwrap();

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message");
    trace!("This is a trace message");
}
