use log::LogRecord;
use LogConfig;
use LogSpecification;
use flexi_error::FlexiLoggerError;
use FlexiLogger;

/// Function type for Format functions.
pub type FormatFunction = fn(&LogRecord) -> String;


/// The standard entry-point for using flexi_logger.
///
/// Create a Logger with your desired loglevel-specification
///
/// * by specifying a String programmatically,
///   using [Logger::with_str()](struct.Logger.html#method.with_str),
/// * or by expecting a String in the environment,
///   using [Logger::with_env()](struct.Logger.html#method.with_env),
/// * or by providing an explicitly built LogSpecification,
///   using [Logger::with()](struct.Logger.html#method.with),
///
/// use its configuration methods, and finally call start().
///
/// ## Examples
///
/// ### Use defaults only
///
/// If you initialize flexi_logger like this, it behaves like env_logger:
///
/// ```
/// use flexi_logger::Logger;
///
/// Logger::with_env().start().unwrap();
/// ```
///
/// ### Write to files, use a detailed log-line format that contains the module and line number
///
/// Here we configure flexi_logger to write log entries with
/// time and location info into a log file in folder "log_files",
/// and we provide the loglevel-specification programmatically, as String:
///
/// ```
/// use flexi_logger::{Logger,opt_format};
///
/// Logger::with_str("myprog=debug, mylib=warn")
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
}

/// Simple methods for influencing the behavior of the Logger.
impl Logger {
    /// Create a Logger that you provide with an explicit LogSpecification.
    pub fn with(logspec: LogSpecification) -> Logger {
        Logger {
            spec: logspec,
            config: LogConfig::default_config_for_logger(),
        }
    }

    /// Create a Logger that reads the LogSpecification from a String or &str.
    pub fn with_str<S: AsRef<str>>(s: S) -> Logger {
        let logspec = LogSpecification::parse(s.as_ref());
        Logger {
            spec: logspec,
            config: LogConfig::default_config_for_logger(),
        }
    }

    /// Create a Logger that reads the LogSpecification from the environment variable RUST_LOG.
    pub fn with_env() -> Logger {
        Logger {
            spec: LogSpecification::env(),
            config: LogConfig::default_config_for_logger(),
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
    /// This parameter only has an effect if log_to_file is set to true.
    /// If the specified folder does not exist, the initialization will fail.
    /// By default, the log files are created in the folder where the program was started.
    pub fn directory<S: Into<String>>(mut self, directory: S) -> Logger {
        self.config.directory = Some(directory.into());
        self
    }

    /// Specifies a suffix for the log files.
    ///
    /// This parameter only has an effect if log_to_file is set to true.
    pub fn suffix<S: Into<String>>(mut self, suffix: S) -> Logger {
        self.config.suffix = Some(suffix.into());
        self
    }

    /// Makes the logger not include a timestamp into the names of the log files
    /// (log_to_file must be chosen, too).
    pub fn suppress_timestamp(mut self) -> Logger {
        self.config.timestamp = false;
        self
    }

    /// This option only has an effect if log_to_file is used, too.
    ///
    /// By default, the log file will grow indefinitely.
    /// With this option, when the log file reaches or exceeds the specified file size,
    /// the file will be closed and a new file will be opened.
    /// Also he filename pattern changes - instead of the timestamp a serial number is included into the filename.
    pub fn rotate_over_size(mut self, rotate_over_size: usize) -> Logger {
        self.config.rotate_over_size = Some(rotate_over_size);
        self
    }

    /// This option only has an effect if log_to_file is used, too.
    ///
    /// The specified String is added to the log file name.
    pub fn discriminant<S: Into<String>>(mut self, discriminant: S) -> Logger {
        self.config.discriminant = Some(discriminant.into());
        self
    }

    /// This option only has an effect if log_to_file is used, too.
    ///
    /// The specified String will be used on linux systems to create in the current folder
    /// a symbolic link to the current log file.
    pub fn create_symlink<S: Into<String>>(mut self, symlink: S) -> Logger {
        self.config.create_symlink = Some(symlink.into());
        self
    }

    /// Consumes the Logger object and initializes the flexi_logger.
    pub fn start(self) -> Result<(), FlexiLoggerError> {
        FlexiLogger::start(self.config, self.spec)
    }

    // used in tests only
    #[doc(hidden)]
    #[allow(dead_code)]
    fn get_config(&self) -> &LogConfig {
        &self.config
    }
}

/// Alternative set of methods to control the behavior of the Logger.
/// use these methods when you want to control the settings dynamically, e.g. with doc_opts.
impl Logger {
    /// With true, makes the logger write all logs to a file, otherwise to stderr.
    pub fn o_log_to_file(mut self, log_to_file: bool) -> Logger {
        self.config.log_to_file = log_to_file;
        self
    }

    /// With true, makes the logger print an info message to stdout when a new file is used for log-output.
    pub fn o_print_message(mut self, print_message: bool) -> Logger {
        self.config.print_message = print_message;
        self
    }

    /// With true, makes the logger write all logged error messages additionally to stdout.
    pub fn o_duplicate_error(mut self, duplicate_error: bool) -> Logger {
        self.config.duplicate_error = duplicate_error;
        self
    }

    /// With true, makes the logger write all logged error, warning, and info messages additionally to stdout.
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
