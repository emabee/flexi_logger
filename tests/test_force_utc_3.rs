mod test_utils;

use flexi_logger::{DeferredNow, Logger};
use log::*;

#[test]
fn test_force_utc_3() {
    DeferredNow::force_utc();
    let _ = Logger::try_with_str("info")
        .unwrap()
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));
    DeferredNow::force_utc();
    info!("must be printed");
}
