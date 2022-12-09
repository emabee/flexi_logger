mod test_utils;

use flexi_logger::{detailed_format, FileSpec, Logger, LoggerHandle};
use log::*;

#[test]
fn test_mods_off() {
    let handle: LoggerHandle = Logger::try_with_env_or_str("info, test_mods_off::mymod1=off")
        .unwrap()
        .format(detailed_format)
        .log_to_file(
            FileSpec::default()
                .suppress_timestamp()
                .directory(self::test_utils::dir()),
        )
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

    error!("This is an error message");
    warn!("This is a warning");
    mymod1::test_traces();
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");

    handle.validate_logs(&[
        ("ERROR", "test_mods", "error"),
        ("WARN", "test_mods", "warning"),
        ("INFO", "test_mods", "info"),
    ]);
}

mod mymod1 {
    use log::*;
    pub fn test_traces() {
        error!("This is an error message");
        warn!("This is a warning");
        info!("This is an info message");
        debug!("This is a debug message");
        trace!("This is a trace message - you must not see it!");

        self::mysubmod::test_traces();
    }
    mod mysubmod {
        use log::*;
        pub fn test_traces() {
            error!("This is an error message - you must not see it!");
            warn!("This is a warning - you must not see it!");
            info!("This is an info message - you must not see it!");
            debug!("This is a debug message - you must not see it!");
            trace!("This is a trace message - you must not see it!");
        }
    }
}
