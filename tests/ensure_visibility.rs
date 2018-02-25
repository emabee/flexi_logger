extern crate flexi_logger;

use flexi_logger::deprecated::{FlexiLogger, LogConfig};

#[allow(dead_code)]
#[test]
fn ensure_visibility() {
    let _ = FlexiLogger::new(None, LogConfig::new()).unwrap();
}
