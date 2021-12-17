mod test_utils;

use flexi_logger::{DeferredNow, Logger};
use log::*;

#[test]
fn test_force_utc_4() {
    let _ = Logger::try_with_str("info")
        .unwrap()
        .use_utc()
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
    DeferredNow::force_utc();
    info!("must be printed");
}
