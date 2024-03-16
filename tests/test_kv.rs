mod test_utils;

use flexi_logger::{FileSpec, Logger};
use log::*;

#[test]
fn test_kv() {
    {}
    let logger = Logger::try_with_str("trace")
        .unwrap()
        .log_to_file(FileSpec::default().directory(self::test_utils::dir()))
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

    if cfg!(feature = "kv") {
        #[cfg(feature = "kv")]
        error!(
            a = 1,
            b = "2 beer or not 2 beer";
            "This is an error message {}",
            5
        );
    } else {
        error!("This is an error message {}", 5);
    }
    warn!("This is a warning message {}", 4);
    info!("This is an info message {}", 2);

    if cfg!(feature = "kv") {
        logger.validate_logs(&[
            ("ERROR", "[test_kv] {a=1, b=\"2 beer or not 2 beer\"}", ""),
            ("WARN", "[test_kv]", "is a warning"),
            ("INFO", "[test_kv]", "is an info"),
        ]);
    } else {
        logger.validate_logs(&[
            ("ERROR", "[test_kv]", ""),
            ("WARN", "[test_kv]", "is a warning"),
            ("INFO", "[test_kv]", "is an info"),
        ]);
    }
}
