mod test_utils;

#[cfg(feature = "json")]
use flexi_logger::{json_format, FileSpec, Logger};
#[cfg(feature = "json")]
use log::*;

#[cfg(feature = "json")]
#[test]
fn test_json() {
    let logger = Logger::try_with_str("trace")
        .unwrap()
        .log_to_file(FileSpec::default().directory(self::test_utils::dir()))
        .format(json_format)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));
    let duration = std::time::Duration::from_millis(200);
    if cfg!(feature = "kv") {
        #[cfg(feature = "kv")]
        error!(
            a = 1,
            b = "2 beer or not 2 beer";
            "This is an error message {} {:?}",
            5,
            duration
        );
    } else {
        error!("This is an error message {} {:?}", 5, duration);
    }
    warn!("This is a warning message {} {:?}", 4, duration);
    info!("This is an info message {} {:?}", 2, duration);

    if cfg!(feature = "kv") {
        logger.validate_logs(&[
            (
                "{\"level\":\"ERROR\",\"timestamp\"",
                "\"module_path\":\"test_json\"",
                "\"line\":20,\"kv\":{\"a\":1,\"b\":\"2 beer or not 2 beer\"},\
                 \"text\":\"This is an error message 5 200ms\"}",
            ),
            (
                "{\"level\":\"WARN\",\"timestamp\"",
                "\"module_path\":\"test_json\"",
                "\"line\":30,\"text\":\"This is a warning message 4 200ms\"",
            ),
            (
                "{\"level\":\"INFO\",\"timestamp\"",
                "\"module_path\":\"test_json\"",
                "\"line\":31,\"text\":\"This is an info message 2 200ms\"",
            ),
        ]);
    } else {
        logger.validate_logs(&[
            (
                "{\"level\":\"ERROR\",\"timestamp\"",
                "\"module_path\":\"test_json\"",
                "\"line\":28,\"text\":\"This is an error message 5 200ms\"",
            ),
            (
                "{\"level\":\"WARN\",\"timestamp\"",
                "\"module_path\":\"test_json\"",
                "\"line\":30,\"text\":\"This is a warning message 4 200ms\"",
            ),
            (
                "{\"level\":\"INFO\",\"timestamp\"",
                "\"module_path\":\"test_json\"",
                "\"line\":31,\"text\":\"This is an info message 2 200ms\"",
            ),
        ]);
    }
}
