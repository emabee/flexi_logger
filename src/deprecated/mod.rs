pub mod log_options;

#[allow(deprecated)]
pub use self::log_options::LogOptions;

use {FlexiLogger, FlexiLoggerError, LogConfig, LogSpecification};

///
#[deprecated]
pub fn init(config: LogConfig, loglevelspec: Option<String>) -> Result<(), FlexiLoggerError> {
    let spec = match loglevelspec {
        Some(loglevelspec) => LogSpecification::parse(&loglevelspec),
        None => LogSpecification::env(),
    };
    FlexiLogger::start(config, spec)
}
