extern crate flexi_logger;

#[macro_use]
extern crate log;

use flexi_logger::{init,LogConfig};

#[test]
fn you_must_see_exactly_three_messages_above_1err_1_warn_1info() {
    init(
        LogConfig::new(),
        Some("info".to_string())
    ).unwrap();

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");
}
