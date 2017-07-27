//! Structures and methods that allow supporting multiple FlexiLogger instances in a single process.
use flexi_writer::FlexiWriter;
use log_config::LogConfig;
use log_specification::LogSpecification;
use log_specification::ModuleFilter;
use FlexiLoggerError;
use log;

use regex::Regex;
use std::cell::RefCell;
use std::io::{self, Write};
use std::ops::DerefMut;
use std::sync::{Arc, Mutex, RwLock};

enum LogSpec {
    STATIC(LogSpecification),
    DYNAMIC(Arc<RwLock<LogSpecification>>),
}


/// Does the logging in the background, is normally not used directly.
///
/// This struct is only used explicitly when you want to allow supporting multiple FlexiLogger
/// instances in a single process.
pub struct FlexiLogger {
    log_specification: LogSpec,
    config: LogConfig,
    // The FlexiWriter has mutable state; since Log.log() requires an unmutable self,
    // we need the internal mutability of RefCell, and we have to wrap it with a Mutex to be
    // thread-safe
    mr_flexi_writer: Mutex<RefCell<FlexiWriter>>,
}

/// Allows reconfiguring the logger while it is in use (see Logger::start_reconfigurable()).
///
/// # Example
///
/// The following example shows how to use the reconfigurability feature.
///
/// ```rust
/// extern crate log;
/// extern crate flexi_logger;
/// use flexi_logger::{Logger, LogSpecBuilder};
/// use log::LogLevelFilter;
///
/// fn main() {
///     // Build the initial log specification
///     let mut builder = LogSpecBuilder::new();  // default is LogLevelFilter::Off
///     builder.default(LogLevelFilter::Info);
///     builder.module("karl", LogLevelFilter::Debug);
///
///     // Initialize Logger, keep builder alive
///     let mut logger_reconf_handle = Logger::with(builder.build())
///         // your logger configuration goes here, as usual
///         .start_reconfigurable()
///         .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
///
///     // ...
///
///     // Modify builder and update the logger
///     builder.default(LogLevelFilter::Error);
///     builder.remove("karl");
///     builder.module("emma", LogLevelFilter::Trace);
///
///     logger_reconf_handle.set_new_spec(builder.build());
///
///     // ...
/// }
/// ```
pub struct ReconfigurationHandle {
    spec: Arc<RwLock<LogSpecification>>,
}
impl ReconfigurationHandle {
    fn new(spec: Arc<RwLock<LogSpecification>>) -> ReconfigurationHandle {
        ReconfigurationHandle { spec: spec }
    }

    /// allows specifying a new LogLevelSpecification for the current logger.
    pub fn set_new_spec(&mut self, new_spec: LogSpecification) {
        let mut guard = self.spec.write().unwrap();
        guard.reconfigure(new_spec);
    }
}

impl FlexiLogger {
    /// Configures and starts the flexi_logger.
    pub fn start(config: LogConfig, spec: LogSpecification) -> Result<(), FlexiLoggerError> {
        let max = spec.module_filters()
                      .iter()
                      .map(|d| d.level_filter)
                      .max()
                      .unwrap_or(log::LogLevelFilter::Off);

        let flexi_logger = FlexiLogger::new_internal(spec, config)?;
        log::set_logger(|max_level| {
            max_level.set(max);
            Box::new(flexi_logger)
        })?;
        Ok(())
    }

    /// Configures and starts the flexi_logger, and returns a handle to reconfigure the logger.
    pub fn start_reconfigurable(config: LogConfig, spec: LogSpecification)
                                -> Result<ReconfigurationHandle, FlexiLoggerError> {
        let (flexi_logger, handle) = FlexiLogger::new_internal_reconfigurable(spec, config)?;
        log::set_logger(|max_level| {
            max_level.set(log::LogLevelFilter::Trace); // no optimization possible, because the spec is dynamic, but max is not
            Box::new(flexi_logger)
        })?;
        Ok(handle)
    }

    fn new_internal(spec: LogSpecification, config: LogConfig)
                    -> Result<FlexiLogger, FlexiLoggerError> {
        Ok(FlexiLogger {
            log_specification: LogSpec::STATIC(spec),
            mr_flexi_writer: Mutex::new(RefCell::new(FlexiWriter::new(&config)?)),
            config: config,
        })
    }

    fn new_internal_reconfigurable
        (spec: LogSpecification, config: LogConfig)
         -> Result<(FlexiLogger, ReconfigurationHandle), FlexiLoggerError> {
        let spec = Arc::new(RwLock::new(spec));
        let flexi_logger = FlexiLogger {
            log_specification: LogSpec::DYNAMIC(spec.clone()),
            mr_flexi_writer: Mutex::new(RefCell::new(FlexiWriter::new(&config)?)),
            config: config,
        };
        let handle = ReconfigurationHandle::new(spec.clone());
        Ok((flexi_logger, handle))
    }

    // Implementation of Log::enabled() with easier testable signature
    fn fl_enabled(&self, level: log::LogLevel, target: &str) -> bool {
        fn check_filter(module_filters: &Vec<ModuleFilter>, level: log::LogLevel, target: &str)
                        -> bool {
            // Search for the longest match, the vector is assumed to be pre-sorted.
            for module_filter in module_filters.iter().rev() {
                match module_filter.module_name {
                    Some(ref module_name) if !target.starts_with(&**module_name) => {}
                    Some(..) | None => return level <= module_filter.level_filter,
                }
            }
            false
        }

        match self.log_specification {
            LogSpec::STATIC(ref ls) => check_filter(ls.module_filters(), level, target),
            LogSpec::DYNAMIC(ref locked_ls) => {
                let guard = locked_ls.read();
                check_filter(guard.as_ref().unwrap().module_filters(), level, target)
            }
        }
    }

    /// Creates a new FlexiLogger instance based on your configuration and a loglevel specification.
    /// Only needed in special setups.
    pub fn new(loglevelspec: Option<String>, config: LogConfig)
               -> Result<FlexiLogger, FlexiLoggerError> {
        let spec = match loglevelspec {
            Some(loglevelspec) => LogSpecification::parse(&loglevelspec),
            None => LogSpecification::env(),
        };
        FlexiLogger::new_internal(spec, config)
    }
}

impl log::Log for FlexiLogger {
    fn enabled(&self, metadata: &log::LogMetadata) -> bool {
        self.fl_enabled(metadata.level(), metadata.target())
    }

    fn log(&self, record: &log::LogRecord) {
        if !self.enabled(record.metadata()) {
            return;
        }

        fn check_text_filter(text_filter: &Option<Regex>, record: &log::LogRecord) -> bool {
            if let Some(filter) = text_filter.as_ref() {
                filter.is_match(&*record.args().to_string())
            } else {
                true
            }
        }
        match self.log_specification {
            LogSpec::STATIC(ref ls) => check_text_filter(ls.text_filter(), &record),
            LogSpec::DYNAMIC(ref locked_ls) => {
                let guard = locked_ls.read();
                check_text_filter(guard.as_ref().unwrap().text_filter(), &record)
            }
        };


        let mut msg = (self.config.format)(record);
        if self.config.log_to_file {
            if self.config.duplicate_error && record.level() == log::LogLevel::Error ||
               self.config.duplicate_info &&
               (record.level() == log::LogLevel::Error || record.level() == log::LogLevel::Warn ||
                record.level() == log::LogLevel::Info) {
                println!("{}", &record.args());
            }
            msg.push('\n');
            let msgb = msg.as_bytes();

            // MutexGuard<RefCell<FlexiWriter>>:
            let mut mutexguard_refcell_fw = self.mr_flexi_writer.lock().unwrap();
            // &mut RefCell<FlexiWriter>:
            let ref_refcell_fw = mutexguard_refcell_fw.deref_mut();
            // RefMut<FlexiWriter>:
            let mut refmut_fw = ref_refcell_fw.borrow_mut();
            let flexi_writer: &mut FlexiWriter = refmut_fw.deref_mut();

            flexi_writer.write(msgb, &self.config);
        } else {
            let _ = writeln!(&mut io::stderr(), "{}", msg);
        }
    }
}

#[cfg(test)]
mod tests {
    use LogLevel;
    use LogConfig;
    use super::FlexiLogger;

    fn make_logger(loglevelspec: &'static str) -> FlexiLogger {
        FlexiLogger::new(Some(loglevelspec.to_string()), LogConfig::new()).unwrap()
    }

    #[test]
    fn match_full_path() {
        let logger = make_logger("crate2=info,crate1::mod1=warn");
        assert!(logger.fl_enabled(LogLevel::Warn, "crate1::mod1"));
        assert!(!logger.fl_enabled(LogLevel::Info, "crate1::mod1"));
        assert!(logger.fl_enabled(LogLevel::Info, "crate2"));
        assert!(!logger.fl_enabled(LogLevel::Debug, "crate2"));
    }

    #[test]
    fn no_match() {
        let logger = make_logger("crate2=info,crate1::mod1=warn");
        assert!(!logger.fl_enabled(LogLevel::Warn, "crate3"));
    }

    #[test]
    fn match_beginning() {
        let logger = make_logger("crate2=info,crate1::mod1=warn");
        assert!(logger.fl_enabled(LogLevel::Info, "crate2::mod1"));
    }

    #[test]
    fn match_beginning_longest_match() {
        let logger = make_logger("abcd = info, abcd::mod1 = error, klmn::mod = debug, klmn = info");
        assert!(logger.fl_enabled(LogLevel::Error, "abcd::mod1::foo"));
        assert!(!logger.fl_enabled(LogLevel::Warn, "abcd::mod1::foo"));
        assert!(logger.fl_enabled(LogLevel::Warn, "abcd::mod2::foo"));
        assert!(!logger.fl_enabled(LogLevel::Debug, "abcd::mod2::foo"));

        assert!(!logger.fl_enabled(LogLevel::Debug, "klmn"));
        assert!(!logger.fl_enabled(LogLevel::Debug, "klmn::foo::bar"));
        assert!(logger.fl_enabled(LogLevel::Info, "klmn::foo::bar"));
    }

    #[test]
    fn match_default() {
        let logger = make_logger("info,abcd::mod1=warn");
        assert!(logger.fl_enabled(LogLevel::Warn, "abcd::mod1"));
        assert!(logger.fl_enabled(LogLevel::Info, "crate2::mod2"));
    }

    #[test]
    fn zero_level() {
        let logger = make_logger("info,crate1::mod1=off");
        assert!(!logger.fl_enabled(LogLevel::Error, "crate1::mod1"));
        assert!(logger.fl_enabled(LogLevel::Info, "crate2::mod2"));
    }

}
