extern crate flexi_logger;
#[macro_use]
extern crate log;

use flexi_logger::LogOptions;

#[test]
fn you_must_see_exactly_three_messages_above_1_err_1_warn_1_info() {
    LogOptions::new()
        .init(Some("info".to_string()))
        .unwrap();

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");
}

use flexi_logger::{FlexiLogger, LogConfig};
#[allow(dead_code)]
fn ensure_visibility() {
    let _ = FlexiLogger::new(None, LogConfig::new()).unwrap();
}
