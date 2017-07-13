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

impl Logger {
    /// Create a Logger that you provide with an explicit LogSpecification.
    pub fn with(logspec: LogSpecification) -> Logger {
        Logger {
            spec: logspec,
            config: LogConfig::new(),
        }
    }

    /// Create a Logger that reads the LogSpecification from a String or &str.
    pub fn with_str<S: AsRef<str>>(s: S) -> Logger {
        let logspec = LogSpecification::parse(s.as_ref());
        Logger {
            spec: logspec,
            config: LogConfig::new(),
        }
    }

    /// Create a Logger that reads the LogSpecification from the environment variable RUST_LOG.
    pub fn with_env() -> Logger {
        Logger {
            spec: LogSpecification::env(),
            config: LogConfig::new(),
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

    /// Makes the logger include a timestamp into the names of the log files
    /// (log_to_file must be chosen, too).
    /// Deprecated because this is the default anyway.
    #[deprecated]
    pub fn timestamp(mut self) -> Logger {
        self.config.timestamp = true;
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
}
