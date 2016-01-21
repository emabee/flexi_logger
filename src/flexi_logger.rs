use flexi_writer::FlexiWriter;

use {FlexiLoggerError,LogConfig};
use log::{self,Log,LogLevel,LogLevelFilter,LogMetadata,LogRecord};
use regex::Regex;
use std::cell::RefCell;
use std::env;
use std::io::{self,Write};

use std::ops::DerefMut;
use std::sync::Mutex;


/// Does the logging.
/// Is only made public to support usecases where more than one FlexiLogger instance are required in a single process.
pub struct FlexiLogger{
    directives: Vec<LogDirective>,
    o_filter: Option<Regex>,
    // The FlexiWriter has mutable state; since Log.log() requires an unmutable self,
    // we need the internal mutability of RefCell, and we have to wrap it with a Mutex to be thread-safe
    mr_flexi_writer: Mutex<RefCell<FlexiWriter>>,
    config: LogConfig
}
impl FlexiLogger {
    /// Creates a new FlexiLogger instance based on your configuration and a loglevel specification.
    pub fn new(loglevelspec: Option<String>, config: LogConfig) -> Result<FlexiLogger, FlexiLoggerError> {
        match FlexiLogger::new_int(loglevelspec,config) {
            Ok((_,fl)) => Ok(fl),
            Err(e) => Err(e)
        }
    }

    fn new_int(loglevelspec: Option<String>, config: LogConfig)
    -> Result<(LogLevelFilter,FlexiLogger), FlexiLoggerError> {

        let (mut directives, filter) = match loglevelspec {
            Some(ref llspec) => {let spec: &str = llspec; parse_logging_spec(&spec)},
            None => {
                match env::var("RUST_LOG") {
                    Ok(spec) => parse_logging_spec(&spec),
                    Err(..) => (vec![LogDirective { name: None, level: LogLevelFilter::Error }], None),
                }
            }
        };

        // Sort the provided directives by length of their name,
        // this allows a little more efficient lookup at runtime.
        directives.sort_by(|a, b| {
            let alen = a.name.as_ref().map(|a| a.len()).unwrap_or(0);
            let blen = b.name.as_ref().map(|b| b.len()).unwrap_or(0);
            alen.cmp(&blen)
        });

        let max = directives.iter().map(|d| d.level).max().unwrap_or(LogLevelFilter::Off);
        let flexi_writer = FlexiWriter::new(&config);
        match flexi_writer {
            Ok(flexi_writer) =>  Ok( (max, FlexiLogger {
                                            directives: directives,
                                            o_filter: filter,
                                            mr_flexi_writer: Mutex::new(RefCell::new(flexi_writer)),
                                            config: config
            })),
            Err(e) => Err(e),
        }
    }

    /// Checks if a log line for the specified target and level is to be written really
    pub fn fl_enabled(&self, level: LogLevel, target: &str) -> bool {
        // Search for the longest match, the vector is assumed to be pre-sorted.
        for directive in self.directives.iter().rev() {
            match directive.name {
                Some(ref name) if !target.starts_with(&**name) => {},
                Some(..) | None => {
                    return level <= directive.level
                }
            }
        }
        false
    }
}

impl Log for FlexiLogger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        self.fl_enabled(metadata.level(), metadata.target())
    }

    fn log(&self, record: &LogRecord) {
        if !Log::enabled(self, record.metadata()) {
            return;
        }

        if let Some(filter) = self.o_filter.as_ref() {
            if filter.is_match(&*record.args().to_string()) {
                return;
            }
        }

        let mut msg = (self.config.format)(record);
        if self.config.log_to_file {
            if self.config.duplicate_error && record.level() == LogLevel::Error
            || self.config.duplicate_info  && record.level() == LogLevel::Info {
                println!("{}",&record.args());
            }
            msg.push('\n');
            let msgb = msg.as_bytes();

            let mut mutexguard_refcell_fw = self.mr_flexi_writer.lock().unwrap();  // MutexGuard<RefCell<FlexiWriter>>
            let ref_refcell_fw = mutexguard_refcell_fw.deref_mut();                 // &mut RefCell<FlexiWriter>
            let mut refmut_fw = ref_refcell_fw.borrow_mut();                        // RefMut<FlexiWriter>
            let flexi_writer: &mut FlexiWriter = refmut_fw.deref_mut();

            flexi_writer.write(msgb, &self.config);
        } else {
            let _ = writeln!(&mut io::stderr(), "{}", msg );
        }
    }
}

struct LogDirective {
    name: Option<String>,
    level: LogLevelFilter,
}

/// Parse a logging specification string (e.g: "crate1,crate2::mod3,crate3::x=error/foo")
/// and return a vector with log directives.
fn parse_logging_spec(spec: &str) -> (Vec<LogDirective>, Option<Regex>) {
    let mut dirs = Vec::new();

    let mut parts = spec.split('/');
    let mods = parts.next();
    let filter = parts.next();
    if parts.next().is_some() {
        print_err!("warning: invalid logging spec '{}', ignoring it (too many '/'s)", spec);
        return (dirs, None);
    }
    mods.map(|m| { for s in m.split(',') {
        if s.len() == 0 { continue }
        let mut parts = s.split('=');
        let (log_level, name) = match (parts.next(), parts.next().map(|s| s.trim()), parts.next()) {
            (Some(part0), None, None) => {
                // if the single argument is a log-level string or number, treat that as a global fallback
                match part0.parse() {
                    Ok(num) => (num, None),
                    Err(_) => (LogLevelFilter::max(), Some(part0)),
                }
            }
            (Some(part0), Some(""), None) => (LogLevelFilter::max(), Some(part0)),
            (Some(part0), Some(part1), None) => {
                match part1.parse() {
                    Ok(num) => (num, Some(part0)),
                    _ => {
                        print_err!("warning: invalid logging spec '{}', ignoring it", part1);
                        continue
                    }
                }
            },
            _ => {
                print_err!("warning: invalid logging spec '{}', ignoring it", s);
                continue
            }
        };
        dirs.push(LogDirective {
            name: name.map(|s| s.to_string()),
            level: log_level,
        });
    }});

    let filter = filter.map_or(None, |filter| {
        match Regex::new(filter) {
            Ok(re) => Some(re),
            Err(e) => {
                print_err!("warning: invalid regex filter - {}", e);
                None
            }
        }
    });

    return (dirs, filter);
}


/// Initializes the flexi_logger to your needs, and the global logger with flexi_logger.
///
/// Note: this should be called early in the execution of a Rust program. The
/// global logger may only be initialized once, subsequent initialization attempts
/// will return an error.
///
/// ## Configuration
///
/// See [LogConfig](struct.LogConfig.html) for most of the initialization options.
///
/// ## Log Level Specification
///
/// Specifying the log levels that you really want to see in a specific program run
/// can be done in the syntax defined by
/// [env_logger -> enabling logging](http://rust-lang.github.io/log/env_logger/#enabling-logging)
/// (from where this functionality was ruthlessly copied).
/// You can hand over the desired log-level-specification as an
/// initialization parameter to flexi_logger, or, if you don't do so,
/// with the environment variable RUST_LOG (as with env_logger).
/// Since using environment variables is on Windows not as comfortable as on linux,
/// you might consider using e.g. a docopt option for specifying the
/// log-Level-specification on the command line of your program.
///
///
/// ## Examples
///
/// ### Use defaults only
///
/// If you initialize flexi_logger with default settings, then it behaves like env_logger:
///
/// ```
/// use flexi_logger::{init,LogConfig};
///
/// init(LogConfig::new(), None).unwrap();
/// ```
///
/// ### Write to files, use a detailed log-line format that contains the module and line number
///
/// Here we configure flexi_logger to write log entries with
/// time and location info into a log file in folder "log_files",
/// and we provide the loglevel-specification programmatically
/// as a ```Some<String>```, which might come in this form from what e.g. [docopt](https://crates.io/crates/docopt)
/// could provide for a respective command-line option:
///
/// ```
/// use flexi_logger::{init,opt_format,LogConfig};
///
/// init( LogConfig { log_to_file: true,
///                   directory: Some("log_files".to_string()),
///                   format: opt_format,
///                   .. LogConfig::new() },
///       Some("myprog=debug,mylib=warn".to_string()) )
/// .unwrap_or_else(|e|{panic!("Logger initialization failed with {}",e)});
/// ```
///
/// # Failures
///
/// Init returns a FlexiLoggerError, if it is supposed to write to an output file
/// but the file cannot be opened, e.g. because of operating system issues.
///
pub fn init(config: LogConfig, loglevelspec: Option<String>) -> Result<(),FlexiLoggerError> {
    match FlexiLogger::new_int(loglevelspec,config) {
        Ok((max,fl)) => {
            log::set_logger( |max_level| {max_level.set(max);Box::new(fl)} )
                 .map_err(|e|{FlexiLoggerError::new(format!("Logger initialization failed due to {}", e))})
        },
        Err(e) => Err(e),
    }
}



#[cfg(test)]
mod tests {
    use log::{LogLevel,LogLevelFilter};
    use LogConfig;
    use super::{FlexiLogger,parse_logging_spec};

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
        let logger = make_logger("crate2=info,crate2::mod=debug,crate1::mod1=warn");
        assert!(logger.fl_enabled(LogLevel::Debug, "crate2::mod1"));
        assert!(!logger.fl_enabled(LogLevel::Debug, "crate2"));
    }

    #[test]
    fn match_default() {
        let logger = make_logger("info,crate1::mod1=warn");
        assert!(logger.fl_enabled(LogLevel::Warn, "crate1::mod1"));
        assert!(logger.fl_enabled(LogLevel::Info, "crate2::mod2"));
    }

    #[test]
    fn zero_level() {
        let logger = make_logger("info,crate1::mod1=off");
        assert!(!logger.fl_enabled(LogLevel::Error, "crate1::mod1"));
        assert!(logger.fl_enabled(LogLevel::Info, "crate2::mod2"));
    }

    #[test]
    fn parse_logging_spec_valid() {
        let (dirs, filter) = parse_logging_spec("crate1::mod1=error,crate1::mod2,crate2=debug");
        assert_eq!(dirs.len(), 3);
        assert_eq!(dirs[0].name, Some("crate1::mod1".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::Error);

        assert_eq!(dirs[1].name, Some("crate1::mod2".to_string()));
        assert_eq!(dirs[1].level, LogLevelFilter::max());

        assert_eq!(dirs[2].name, Some("crate2".to_string()));
        assert_eq!(dirs[2].level, LogLevelFilter::Debug);
        assert!(filter.is_none());
    }

    #[test]
    fn parse_logging_spec_invalid_crate() {
        // test parse_logging_spec with multiple = in specification
        let (dirs, filter) = parse_logging_spec("crate1::mod1=warn=info,crate2=debug");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::Debug);
        assert!(filter.is_none());
    }

    #[test]
    fn parse_logging_spec_invalid_log_level() {
        // test parse_logging_spec with 'noNumber' as log level
        let (dirs, filter) = parse_logging_spec("crate1::mod1=noNumber,crate2=debug");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::Debug);
        assert!(filter.is_none());
    }

    #[test]
    fn parse_logging_spec_string_log_level() {
        // test parse_logging_spec with 'warn' as log level
        let (dirs, filter) = parse_logging_spec("crate1::mod1=wrong,crate2=warn");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::Warn);
        assert!(filter.is_none());
    }

    #[test]
    fn parse_logging_spec_empty_log_level() {
        // test parse_logging_spec with '' as log level
        let (dirs, filter) = parse_logging_spec("crate1::mod1=wrong,crate2=");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::max());
        assert!(filter.is_none());
    }

    #[test]
    fn parse_logging_spec_global() {
        // test parse_logging_spec with no crate
        let (dirs, filter) = parse_logging_spec("warn,crate2=debug");
        assert_eq!(dirs.len(), 2);
        assert_eq!(dirs[0].name, None);
        assert_eq!(dirs[0].level, LogLevelFilter::Warn);
        assert_eq!(dirs[1].name, Some("crate2".to_string()));
        assert_eq!(dirs[1].level, LogLevelFilter::Debug);
        assert!(filter.is_none());
    }

    #[test]
    fn parse_logging_spec_valid_filter() {
        let (dirs, filter) = parse_logging_spec("crate1::mod1=error,crate1::mod2,crate2=debug/abc");
        assert_eq!(dirs.len(), 3);
        assert_eq!(dirs[0].name, Some("crate1::mod1".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::Error);

        assert_eq!(dirs[1].name, Some("crate1::mod2".to_string()));
        assert_eq!(dirs[1].level, LogLevelFilter::max());

        assert_eq!(dirs[2].name, Some("crate2".to_string()));
        assert_eq!(dirs[2].level, LogLevelFilter::Debug);
        assert!(filter.is_some() && filter.unwrap().to_string() == "abc");
    }

    #[test]
    fn parse_logging_spec_invalid_crate_filter() {
        let (dirs, filter) = parse_logging_spec("crate1::mod1=error=warn,crate2=debug/a.c");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::Debug);
        assert!(filter.is_some() && filter.unwrap().to_string() == "a.c");
    }

    #[test]
    fn parse_logging_spec_empty_with_filter() {
        let (dirs, filter) = parse_logging_spec("crate1/a*c");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate1".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::max());
        assert!(filter.is_some() && filter.unwrap().to_string() == "a*c");
    }
}
