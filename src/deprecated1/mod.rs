pub mod log_options;

#[allow(deprecated)]
pub use self::log_options::LogOptions;

use {FlexiLoggerError, LogSpecification};
use log_config::LogConfig;
use deprecated::FlexiLogger;
///
#[deprecated]
pub fn init(config: LogConfig, loglevelspec: Option<String>) -> Result<(), FlexiLoggerError> {
    let spec = match loglevelspec {
        Some(loglevelspec) => LogSpecification::parse(&loglevelspec),
        None => LogSpecification::env(),
    };
    FlexiLogger::start(config, spec)
}
