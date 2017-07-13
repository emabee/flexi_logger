//! Structures and methods that allow supporting multiple FlexiLogger instances in a single process.
use flexi_writer::FlexiWriter;
use log_config::LogConfig;
use log_specification::LogSpecification;
use FlexiLoggerError;
use log;

use std::cell::RefCell;
use std::io::{self, Write};
use std::ops::DerefMut;
use std::sync::Mutex;


/// Does the logging in the background, is normally not used directly.
///
/// This struct is only used explicitly when you want to allow supporting multiple FlexiLogger
/// instances in a single process.
pub struct FlexiLogger {
    log_specification: LogSpecification,
    config: LogConfig,
    // The FlexiWriter has mutable state; since Log.log() requires an unmutable self,
    // we need the internal mutability of RefCell, and we have to wrap it with a Mutex to be
    // thread-safe
    mr_flexi_writer: Mutex<RefCell<FlexiWriter>>,
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

        Ok(log::set_logger(|max_level| {
            max_level.set(max);
            Box::new(flexi_logger)
        })?)
    }

    fn new_internal(spec: LogSpecification, config: LogConfig)
                    -> Result<FlexiLogger, FlexiLoggerError> {
        Ok(FlexiLogger {
            log_specification: spec,
            mr_flexi_writer: Mutex::new(RefCell::new(FlexiWriter::new(&config)?)),
            config: config,
        })
    }

    // Implementation of Log::enabled() with easier testable signature
    fn fl_enabled(&self, level: log::LogLevel, target: &str) -> bool {
        // Search for the longest match, the vector is assumed to be pre-sorted.
        for module_filter in self.log_specification.module_filters().iter().rev() {
            match module_filter.module_name {
                Some(ref module_name) if !target.starts_with(&**module_name) => {}
                Some(..) | None => return level <= module_filter.level_filter,
            }
        }
        false
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
        if !log::Log::enabled(self, record.metadata()) {
            return;
        }

        if let Some(filter) = self.log_specification.text_filter().as_ref() {
            if filter.is_match(&*record.args().to_string()) {
                return;
            }
        }

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
