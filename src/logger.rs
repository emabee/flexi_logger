use writers::LogWriter;
use log::Record;
use log_config::LogConfig;
use LogSpecification;
use flexi_error::FlexiLoggerError;
use flexi_logger::FlexiLogger;
use ReconfigurationHandle;
use std::collections::HashMap;

/// Function type for Format functions.
pub type FormatFunction = fn(&Record) -> String;

/// The standard entry-point for using `flexi_logger`.
///
/// Create a Logger with your desired (initial) loglevel-specification
///
/// * by specifying a String programmatically,
///   using [`Logger::with_str()`](struct.Logger.html#method.with_str),
/// * or by expecting a String in the environment,
///   using [`Logger::with_env()`](struct.Logger.html#method.with_env),
/// * or by providing an explicitly built `LogSpecification`,
///   using [`Logger::with()`](struct.Logger.html#method.with),
///
/// use its configuration methods, and finally call [start()](struct.Logger.html#method.start)
/// (or [`start_reconfigurable()`](struct.Logger.html#method.start_reconfigurable)).
///
/// ## Examples
///
/// ### Use defaults only
///
/// If you initialize `flexi_logger` like this, it behaves like `env_logger`:
///
/// ```rust
/// use flexi_logger::Logger;
///
/// Logger::with_env().start().unwrap();
/// ```
///
/// ### Write to files, use a detailed log-line format that contains the module and line number
///
/// Here we configure `flexi_logger` to write log entries with
/// time and location info into a log file in folder "`log_files`",
/// and we provide the loglevel-specification programmatically, as String, but allow it
/// to be overridden by the environment variable `RUST_LOG`:
///
/// ```
/// use flexi_logger::{Logger,opt_format};
///
/// Logger::with_env_or_str("myprog=debug, mylib=warn")
///             .log_to_file()
///             .directory("log_files")
///             .format(opt_format)
///             .start()
///             .unwrap_or_else(|e|{panic!("Logger initialization failed with {}",e)});
/// ```
///
pub struct Logger {
    spec: LogSpecification,
    config: LogConfig,
    other_writers: HashMap<String, Box<LogWriter>>,
}

/// Simple methods for influencing the behavior of the Logger.
impl Logger {
    /// Creates a Logger that you provide with an explicit LogSpecification.
    pub fn with(logspec: LogSpecification) -> Logger {
        Logger {
            spec: logspec,
            config: LogConfig::default_config_for_logger(),
            other_writers: HashMap::<String, Box<LogWriter>>::new(),
        }
    }

    /// Creates a Logger that reads the LogSpecification from a String or &str.
    /// [See LogSpecification](struct.LogSpecification.html) for the syntax.
    pub fn with_str<S: AsRef<str>>(s: S) -> Logger {
        let logspec = LogSpecification::parse(s.as_ref());
        Logger {
            spec: logspec,
            config: LogConfig::default_config_for_logger(),
            other_writers: HashMap::<String, Box<LogWriter>>::new(),
        }
    }

    /// Creates a Logger that reads the LogSpecification from the environment variable RUST_LOG.
    pub fn with_env() -> Logger {
        Logger {
            spec: LogSpecification::env(),
            config: LogConfig::default_config_for_logger(),
            other_writers: HashMap::<String, Box<LogWriter>>::new(),
        }
    }

    /// Creates a Logger that reads the LogSpecification from the environment variable RUST_LOG,
    /// or derives it from the given String, if RUST_LOG is not set.
    pub fn with_env_or_str<S: AsRef<str>>(s: S) -> Logger {
        Logger {
            spec: LogSpecification::env_or_parse(s),
            config: LogConfig::default_config_for_logger(),
            other_writers: HashMap::<String, Box<LogWriter>>::new(),
        }
    }

    /// Makes the logger write all logs to a file, rather than to stderr.
    pub fn log_to_file(mut self) -> Logger {
        self.config.log_to_file = true;
        self
    }

    /// Makes the logger print an info message to stdout when a new file is used for log-output.
    pub fn print_message(mut self) -> Logger {
        self.config.print_message = true;
        self
    }

    /// Makes the logger write all logged error messages additionally to stdout.
    pub fn duplicate_error(mut self) -> Logger {
        self.config.duplicate_error = true;
        self
    }

    /// Makes the logger write all logged error, warning, and info messages additionally to stdout.
    pub fn duplicate_info(mut self) -> Logger {
        self.config.duplicate_info = true;
        self
    }

    /// Makes the logger use the provided format function for the log entries,
    /// rather than the default ([formats::default_format](fn.default_format.html)).
    pub fn format(mut self, format: FormatFunction) -> Logger {
        self.config.format = format;
        self
    }

    /// Specifies a folder for the log files.
    ///
    /// This parameter only has an effect if `log_to_file` is set to true.
    /// If the specified folder does not exist, the initialization will fail.
    /// By default, the log files are created in the folder where the program was started.
    pub fn directory<S: Into<String>>(mut self, directory: S) -> Logger {
        self.config.directory = Some(directory.into());
        self
    }

    /// Specifies a suffix for the log files.
    ///
    /// This parameter only has an effect if `log_to_file` is set to true.
    pub fn suffix<S: Into<String>>(mut self, suffix: S) -> Logger {
        self.config.suffix = Some(suffix.into());
        self
    }

    /// Makes the logger not include a timestamp into the names of the log files
    ///
    /// This option only has an effect if `log_to_file` is used, too.
    pub fn suppress_timestamp(mut self) -> Logger {
        self.config.timestamp = false;
        self
    }

    /// By default, the log file will grow indefinitely.
    /// With this option, when the log file reaches or exceeds the specified file size,
    /// the file will be closed and a new file will be opened.
    /// Also the filename pattern changes - instead of the timestamp,
    /// a serial number is included into the filename.
    ///
    /// This option only has an effect if `log_to_file` is used, too.
    pub fn rotate_over_size(mut self, rotate_over_size: usize) -> Logger {
        self.config.rotate_over_size = Some(rotate_over_size);
        self
    }

    /// The specified String is added to the log file name.
    ///
    /// This option only has an effect if `log_to_file` is used, too.
    pub fn discriminant<S: Into<String>>(mut self, discriminant: S) -> Logger {
        self.config.discriminant = Some(discriminant.into());
        self
    }

    /// The specified String will be used on linux systems to create in the current folder
    /// a symbolic link to the current log file.
    ///
    /// This option only has an effect if `log_to_file` is used, too.
    pub fn create_symlink<S: Into<String>>(mut self, symlink: S) -> Logger {
        self.config.create_symlink = Some(symlink.into());
        self
    }

    /// Registers a LogWriter implementation under the given target name.
    ///
    /// Names should not start with an underscore.
    /// The name given here is used in calls to the log macros to identify the writer(s)
    /// to which a log message is sent.
    ///
    /// A log call with a target value that has the form `{Name1,Name2,...}`, i.e.,
    /// a comma-separated list of target names, within braces, is not sent to the default logger,
    /// but to the loggers specified explicitly in the list.
    /// In such a list you can again specify the default logger with the name `_Default`.
    ///
    /// See also [the module documentation of `writers`](writers/index.html).
    pub fn add_writer<S: Into<String>>(mut self, name: S, writer: Box<LogWriter>) -> Logger {
        self.other_writers.insert(name.into(), writer);
        self
    }

    /// Consumes the Logger object and initializes the flexi_logger.
    pub fn start(self) -> Result<(), FlexiLoggerError> {
        FlexiLogger::start_multi(self.config, self.spec, self.other_writers)
    }

    /// Consumes the Logger object and initializes the flexi_logger in a way that
    /// subsequently the log specification can be exchanged dynamically.
    ///
    /// The resulting logger is still fast, but measurable slower for those log-calls (trace!() etc)
    /// that are on a deeper level than the deepest level in the LogSpecification.
    /// This is because the Log crate has an optimization for returning very fast from deep-level
    /// log calls, but the deepest level needs be given at initialization and cannot be updated
    /// later.
    ///
    /// Here is the output from a benchmark test, runnning on a windows laptop:
    ///
    ///  ```text
    ///   1  PS C:\projects\flexi_logger> cargo bench --bench bench_standard -- --nocapture
    ///   2      Finished release [optimized] target(s) in 0.4 secs
    ///   3       Running target\release\deps\bench_standard-20539c2be6d4f2e0.exe
    ///   4
    ///   5  running 4 tests
    ///   6  test b10_no_logger_active  ... bench:         118 ns/iter (+/- 19)
    ///   7  test b20_initialize_logger ... bench:           0 ns/iter (+/- 0)
    ///   8  test b30_relevant_logs     ... bench:     291,436 ns/iter (+/- 44,658)
    ///   9  test b40_suppressed_logs   ... bench:         123 ns/iter (+/- 5)
    ///  10
    ///  11  test result: ok. 0 passed; 0 failed; 0 ignored; 4 measured; 0 filtered out
    ///  12
    ///  13  PS C:\projects\flexi_logger> cargo bench --bench bench_reconfigurable -- --nocapture
    ///  14      Finished release [optimized] target(s) in 0.4 secs
    ///  15       Running target\release\deps\bench_reconfigurable-2e292a8d5c887d0d.exe
    ///  16
    ///  17  running 4 tests
    ///  18  test b10_no_logger_active  ... bench:         130 ns/iter (+/- 37)
    ///  19  test b20_initialize_logger ... bench:           0 ns/iter (+/- 0)
    ///  20  test b30_relevant_logs     ... bench:     301,092 ns/iter (+/- 87,452)
    ///  21  test b40_suppressed_logs   ... bench:       3,482 ns/iter (+/- 339)
    ///  22
    ///  23  test result: ok. 0 passed; 0 failed; 0 ignored; 4 measured; 0 filtered out
    ///  ```
    ///
    /// It shows that logging is fastest when no logger is active (lines 6 and 18).
    /// And it is just as fast when the above-mentioned optimization kicks in (line 9).
    ///
    /// Logging has measurable costs when logs are really written (line 8 and 20), independent
    /// of the reconfigurability feature of the flexi_logger.
    ///
    /// The measurable, but still in most cases not important, price for reconfigurability
    /// can be seen by comparing lines 9 and 21.
    ///
    pub fn start_reconfigurable(self) -> Result<ReconfigurationHandle, FlexiLoggerError> {
        FlexiLogger::start_multi_reconfigurable(self.config, self.spec, self.other_writers)
    }

    // used in tests only
    #[doc(hidden)]
    #[allow(dead_code)]
    fn get_config(&self) -> &LogConfig {
        &self.config
    }
}

/// Alternative set of methods to control the behavior of the Logger.
/// Use these methods when you want to control the settings dynamically, e.g. with `doc_opts`.
impl Logger {
    /// With true, makes the logger write all logs to a file, otherwise to stderr.
    pub fn o_log_to_file(mut self, log_to_file: bool) -> Logger {
        self.config.log_to_file = log_to_file;
        self
    }

    /// With true, makes the logger print an info message to stdout, each time
    /// when a new file is used for log-output.
    pub fn o_print_message(mut self, print_message: bool) -> Logger {
        self.config.print_message = print_message;
        self
    }

    /// With true, makes the logger write all logged error messages additionally to stdout.
    pub fn o_duplicate_error(mut self, duplicate_error: bool) -> Logger {
        self.config.duplicate_error = duplicate_error;
        self
    }

    /// With true, makes the logger write all logged error, warning,
    /// and info messages additionally to stdout.
    pub fn o_duplicate_info(mut self, duplicate_info: bool) -> Logger {
        self.config.duplicate_info = duplicate_info;
        self
    }

    /// Specifies a folder for the log files.
    ///
    /// This parameter only has an effect if log_to_file is set to true.
    /// If the specified folder does not exist, the initialization will fail.
    /// With None, the log files are created in the folder where the program was started.
    pub fn o_directory<S: Into<String>>(mut self, directory: Option<S>) -> Logger {
        self.config.directory = directory.map(|d| d.into());
        self
    }

    /// Specifies a suffix for the log files.
    ///
    /// This parameter only has an effect if log_to_file is set to true.
    /// By default, the suffix 'log' is used. With None, no suffix is used.
    pub fn o_suffix<S: Into<String>>(mut self, suffix: Option<S>) -> Logger {
        self.config.suffix = suffix.map(|s| s.into());
        self
    }

    /// With true, makes the logger include a timestamp into the names of the log files.
    /// (log_to_file must be chosen, too).
    pub fn o_timestamp(mut self, timestamp: bool) -> Logger {
        self.config.timestamp = timestamp;
        self
    }

    /// This option only has an effect if log_to_file is used, too.
    ///
    /// By default, and with None, the log file will grow indefinitely.
    /// If a size is set, when the log file reaches or exceeds the specified size,
    /// the file will be closed and a new file will be opened.
    /// Also the filename pattern changes - instead of the timestamp a serial number
    /// is included into the filename.
    pub fn o_rotate_over_size(mut self, rotate_over_size: Option<usize>) -> Logger {
        self.config.rotate_over_size = rotate_over_size;
        self
    }

    /// This option only has an effect if log_to_file is used, too.
    ///
    /// The specified String is added to the log file name.
    pub fn o_discriminant<S: Into<String>>(mut self, discriminant: Option<S>) -> Logger {
        self.config.discriminant = discriminant.map(|d| d.into());
        self
    }

    /// This option only has an effect if log_to_file is used, too.
    ///
    /// If a String is specified, it will be used on linux systems to create in the current folder
    /// a symbolic link with this name to the current log file.
    pub fn o_create_symlink<S: Into<String>>(mut self, symlink: Option<S>) -> Logger {
        self.config.create_symlink = symlink.map(|s| s.into());
        self
    }
}

#[cfg(test)]
mod tests {
    extern crate log;
    use Logger;

    #[test]
    fn verify_defaults() {
        let logger = Logger::with_str("");
        let config = logger.get_config();

        assert!(config.log_to_file == false);
        assert!(config.print_message == false);
        assert!(config.duplicate_error == false);
        assert!(config.duplicate_info == false);
        assert!(config.directory == None);
        assert!(config.suffix == Some("log".to_string()));
        assert!(config.timestamp == true);
        assert!(config.rotate_over_size == None);
        assert!(config.discriminant == None);
        assert!(config.create_symlink == None);
    }

    #[test]
    fn verify_non_defaults() {
        let logger = Logger::with_str("")
            .log_to_file()
            .print_message()
            .duplicate_error()
            .duplicate_info()
            .directory("logdir")
            .suffix("trc")
            .suppress_timestamp()
            .rotate_over_size(10_000_000)
            .discriminant("TEST")
            .create_symlink("current_log_file");
        let config = logger.get_config();

        assert!(config.log_to_file == true);
        assert!(config.print_message == true);
        assert!(config.duplicate_error == true);
        assert!(config.duplicate_info == true);
        assert!(config.directory == Some("logdir".to_string()));
        assert!(config.suffix == Some("trc".to_string()));
        assert!(config.timestamp == false);
        assert!(config.rotate_over_size == Some(10_000_000));
        assert!(config.discriminant == Some("TEST".to_string()));
        assert!(config.create_symlink == Some("current_log_file".to_string()));
    }
}
