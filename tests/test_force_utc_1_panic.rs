mod test_utils;

use flexi_logger::{DeferredNow, Logger};
use log::*;

#[test]
#[should_panic]
fn test_force_utc_1_panic() {
    let _ = Logger::try_with_str("info")
        .unwrap()
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
    DeferredNow::force_utc();
    info!("MUST NOT BE REACHED");
}
