extern crate flexi_logger;

#[macro_use]
extern crate log;

use flexi_logger::{init, LogConfig};

#[test]
fn you_must_see_exactly_three_messages_above_1_err_1_warn_1_info() {
    init(LogConfig::new(), Some("info".to_string())).unwrap();

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");
}

use flexi_logger::{FlexiLogger, LogLevel};
#[allow(dead_code)]
fn ensure_visibility() {
    let fl = FlexiLogger::new(None, LogConfig::new()).unwrap();
    fl.fl_enabled(LogLevel::Error, "foo");
}
