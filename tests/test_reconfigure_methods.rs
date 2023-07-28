mod test_utils;

use flexi_logger::{FileSpec, Logger, LoggerHandle};
use log::*;

#[test]
fn test_reconfigure_methods() {
    let mut logger = Logger::try_with_str("info")
        .unwrap()
        .log_to_file(
            FileSpec::default()
                .suppress_timestamp()
                .directory(self::test_utils::dir()),
        )
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

    test_parse_new_spec(&logger);
    test_push_new_spec(&mut logger);
    validate_logs(&logger);
}

fn test_parse_new_spec(logger: &LoggerHandle) {
    error!("1-error message");
    warn!("1-warning");
    info!("1-info message");
    debug!("1-debug message - you must not see it!");
    trace!("1-trace message - you must not see it!");

    logger.parse_new_spec("error").ok();
    error!("1-error message");
    warn!("1-warning - you must not see it!");
    info!("1-info message - you must not see it!");
    debug!("1-debug message - you must not see it!");
    trace!("1-trace message - you must not see it!");

    logger.parse_new_spec("trace").ok();
    error!("1-error message");
    warn!("1-warning");
    info!("1-info message");
    debug!("1-debug message");
    trace!("1-trace message");

    logger.parse_new_spec("info").ok();
}

fn test_push_new_spec(logger: &mut LoggerHandle) {
    error!("2-error message");
    warn!("2-warning");
    info!("2-info message");
    debug!("2-debug message - you must not see it!");
    trace!("2-trace message - you must not see it!");

    logger.parse_and_push_temp_spec("error").ok();
    error!("2-error message");
    warn!("2-warning - you must not see it!");
    info!("2-info message - you must not see it!");
    debug!("2-debug message - you must not see it!");
    trace!("2-trace message - you must not see it!");

    logger.parse_and_push_temp_spec("trace").ok();
    error!("2-error message");
    warn!("2-warning");
    info!("2-info message");
    debug!("2-debug message");
    trace!("2-trace message");

    logger.pop_temp_spec(); // we should be back on error
    error!("2-error message");
    warn!("2-warning - you must not see it!");
    info!("2-info message - you must not see it!");
    debug!("2-debug message - you must not see it!");
    trace!("2-trace message - you must not see it!");

    logger.pop_temp_spec(); // we should be back on info

    error!("2-error message");
    warn!("2-warning");
    info!("2-info message");
    debug!("2-debug message - you must not see it!");
    trace!("2-trace message - you must not see it!");

    logger.pop_temp_spec(); // should be a no-op
}

fn validate_logs(logger: &LoggerHandle) {
    logger.validate_logs(&[
        ("ERROR", "test_reconfigure_methods", "1-error"),
        ("WARN", "test_reconfigure_methods", "1-warning"),
        ("INFO", "test_reconfigure_methods", "1-info"),
        //
        ("ERROR", "test_reconfigure_methods", "1-error"),
        //
        ("ERROR", "test_reconfigure_methods", "1-error"),
        ("WARN", "test_reconfigure_methods", "1-warning"),
        ("INFO", "test_reconfigure_methods", "1-info"),
        ("DEBUG", "test_reconfigure_methods", "1-debug"),
        ("TRACE", "test_reconfigure_methods", "1-trace"),
        // -----
        ("ERROR", "test_reconfigure_methods", "2-error"),
        ("WARN", "test_reconfigure_methods", "2-warning"),
        ("INFO", "test_reconfigure_methods", "2-info"),
        //
        ("ERROR", "test_reconfigure_methods", "2-error"),
        //
        ("ERROR", "test_reconfigure_methods", "2-error"),
        ("WARN", "test_reconfigure_methods", "2-warning"),
        ("INFO", "test_reconfigure_methods", "2-info"),
        ("DEBUG", "test_reconfigure_methods", "2-debug"),
        ("TRACE", "test_reconfigure_methods", "2-trace"),
        //
        ("ERROR", "test_reconfigure_methods", "2-error"),
        //
        ("ERROR", "test_reconfigure_methods", "2-error"),
        ("WARN", "test_reconfigure_methods", "2-warning"),
        ("INFO", "test_reconfigure_methods", "2-info"),
    ]);
}
