use log::Record;
use FlexiLoggerError;
use Logger;
use log_config::LogConfig;

pub type FormatFunction = fn(&Record) -> String;

/// Deprecated. Use Logger instead.
#[allow(unknown_lints)]
#[allow(new_without_default)]
#[deprecated]
pub struct LogOptions(LogConfig);

#[allow(deprecated)]
impl Default for LogOptions {
    fn default() -> Self {
        LogOptions(LogConfig::new())
    }
}

#[allow(deprecated)]
impl LogOptions {
    /// The defaults for the logger initialization are
    ///
    /// *  log_to_file(false)
    /// *  print_message(true)
    /// *  duplicate_error(true)
    /// *  duplicate_info(false)
    /// *  format(flexi_logger::default_format)
    /// *  directory(None)
    /// *  suffix(Some("log".to_string()))
    /// *  timestamp(false)
    /// *  rotate_over_size(None)
    /// *  discriminant(None)
    /// *  symlink(None)
    pub fn new() -> LogOptions {
        LogOptions(LogConfig::new())
    }

    /// Sets the option for logging to a file.
    ///
    /// If this option is set to true,  all logs will be written to a file, otherwise to stderr.
    pub fn log_to_file(mut self, log_to_file: bool) -> LogOptions {
        self.0.log_to_file = log_to_file;
        self
    }

    /// Sets the option for printing out an info message when a new output file is used.
    ///
    /// If this option is set to true, an info message is printed to stdout when a new file is used for log-output.
    pub fn print_message(mut self, print_message: bool) -> LogOptions {
        self.0.print_message = print_message;
        self
    }

    /// Sets the option for duplicating logged error messages to stdout.
    ///
    /// If this option is true and duplicate_error is set to true,
    /// then all logged error messages are additionally written to stdout.
    pub fn duplicate_error(mut self, duplicate_error: bool) -> LogOptions {
        self.0.duplicate_error = duplicate_error;
        self
    }

    /// Sets the option for duplicating logged error, warning and info messages to stdout.
    ///
    /// If log_to_file is true and duplicate_info is set to true,
    /// then all logged error, warning, and info messages are additionally written to stdout.
    pub fn duplicate_info(mut self, duplicate_info: bool) -> LogOptions {
        self.0.duplicate_info = duplicate_info;
        self
    }

    /// Specifies a formatting function for the log entries.
    ///
    /// The function being used by default is [formats::default_format](fn.default_format.html).
    pub fn format(mut self, format: FormatFunction) -> LogOptions {
        self.0.format = format;
        self
    }

    /// Specifies a folder for the log files.
    ///
    /// This parameter only has an effect if log_to_file is set to true.
    /// If set to None, the log files are created in the folder where the program was started,
    /// otherwise in the specified folder.
    /// If the folder does not exist, the initialization will fail.
    pub fn directory(mut self, directory: Option<String>) -> LogOptions {
        self.0.directory = directory;
        self
    }

    /// Specifies a suffix for the log files.
    ///
    /// This parameter only has an effect if log_to_file is set to true.
    ///
    /// If not set to None, then the log files are created with the specified suffix.
    pub fn suffix(mut self, suffix: Option<String>) -> LogOptions {
        self.0.suffix = suffix;
        self
    }

    /// Sets the option for including a timestamp into the name of the log files.
    ///
    /// This parameter only has an effect if log_to_file is set to true.
    ///
    /// If set to true, then the names of the log files will include a timestamp.
    pub fn timestamp(mut self, timestamp: bool) -> LogOptions {
        self.0.timestamp = timestamp;
        self
    }

    /// Sets the option for specifying a maximum size for log files in bytes.
    ///
    /// This parameter only has an effect if log_to_file is set to true.
    ///
    /// If set to None, the log file will grow indefinitely.
    /// If a value is set, then when the log file reaches or exceeds the specified file size, the file will be closed
    /// and a new file will be opened. The filename pattern changes - instead of the
    /// timestamp a serial number is included into the filename.
    pub fn rotate_over_size(mut self, rotate_over_size: Option<usize>) -> LogOptions {
        self.0.rotate_over_size = rotate_over_size.map(|value| value);
        self
    }

    /// Sets the option for specifying an additional part of the log file name.
    ///
    /// This parameter only has an effect if log_to_file is set to true.
    ///
    /// If specified, the additional part of the log file name is inserted after the program name.
    pub fn discriminant(mut self, discriminant: Option<String>) -> LogOptions {
        self.0.discriminant = discriminant;
        self
    }

    /// Sets the option for including a timestamp into the name of the log files.
    ///
    /// This parameter only has an effect if log_to_file is set to true.
    ///
    /// If a String is specified, it will be used on linux systems to create in the current folder
    /// a symbolic link to the current log file.
    pub fn symlink(mut self, create_symlink: Option<String>) -> LogOptions {
        self.0.create_symlink = create_symlink;
        self
    }

    /// Consumes the LogOptions object and initializes the flexi_logger.
    pub fn init(self, loglevelspec: Option<String>) -> Result<(), FlexiLoggerError> {
        //::flexi_logger::initialize(self.0, loglevelspec)
        match loglevelspec {
            Some(loglevelspec) => Logger::with_str(loglevelspec).start()?,
            None => Logger::with_env().start()?,
        };
        Ok(())
    }
}
