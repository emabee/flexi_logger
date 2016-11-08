use log::LogRecord;
use formats;
use flexi_error::FlexiLoggerError;

pub type FormatFunction = fn(&LogRecord) -> String;


/// Internal struct for influencing the behavior of flexi_logger.
///
/// This structure is only needed if you want to instantiate multiple loggers in a your process.
pub struct LogConfig {
    /// Allows providing a custom logline format; by default ```flexi_logger::default_format``` is used.
    /// You can either choose between three predefined variants,
    /// ```default_format```, ```opt_format``` and ```detailed_format```,
    /// or you create and use your own format function with the signature ```fn(&LogRecord) -> String```.
    pub format: fn(&LogRecord) -> String,

    /// * If `false` (default), the log is written to stderr.
    /// * If `true`, a new file is created and the log is written to it.
    /// The default pattern for the filename is '\<program_name\>\_\<date\>\_\<time\>.\<suffix\>',
    ///  e.g. `myprog_2015-07-08_10-44-11.log`.
    ///
    /// <p>Note that all following members are only relevant if this one is set to `true`.
    pub log_to_file: bool,

    /// If `true` (default), the name of the logfile is documented in a message to stdout.
    pub print_message: bool,

    /// If `true` (default), all logged error messages are duplicated to stdout.
    pub duplicate_error: bool,

    /// If `true` (default), all logged warning and info messages are also duplicated to stdout.
    pub duplicate_info: bool,

    /// Allows specifying a directory in which the log files are created. Default is ```None```.
    pub directory: Option<String>,

    /// Allows specifying the filesystem suffix of the log files (without the dot).  Default is ```log```.
    pub suffix: Option<String>,

    /// Allows specifying whether or not the filename should include a timestamp. Default is ```true```.
    pub timestamp: bool,

    /// Allows specifying a maximum size for log files in bytes; when
    /// the specified file size is reached or exceeded, the file will be closed and a new one will be opened.
    /// The filename pattern changes - instead of the timestamp the serial number is included into the filename.
    pub rotate_over_size: Option<usize>,

    /// Allows specifying an additional part of the log file name that is inserted after the program name.
    /// Default is ```None```.
    pub discriminant: Option<String>,

    /// Allows specifying an option to create a symlink to the most recent log file created
    /// using the given name. Default is ```None```.
    ///
    /// Note that this option is only effective on linux systems.
    pub create_symlink: Option<String>,
}
impl LogConfig {
    /// The defaults for the logger initialization are
    ///
    /// *  log_to_file = false
    /// *  print_message = true
    /// *  duplicate_error = true
    /// *  duplicate_info = false
    /// *  format = flexi_logger::default_format
    /// *  no directory: log files (if they were used) are created where the program was started
    /// *  no rotate_over: log file (if it were used) grows indefinitely
    /// *  the name of the log file (if it were used) consists of progname, timestamp, and suffix ```log```
    /// *  no symlink being created.
    ///
    /// We recommend using this constructor as described in the examples of function [init](fn.init.html)
    /// to avoid compilation issues with your code, if future versions of flexi_logger
    /// come with extensions to LogConfig.
    pub fn new() -> LogConfig {
        LogConfig {
            log_to_file: false,
            print_message: true,
            duplicate_error: true,
            duplicate_info: false,
            format: formats::default_format,
            directory: None,
            suffix: Some("log".to_string()),
            timestamp: true,
            rotate_over_size: None,
            discriminant: None,
            create_symlink: None,
        }
    }
}


/// Allows initializing flexi_logger to your needs.
///
/// In order to initialize flexi_logger, create an instance of LogOptions,
/// use one or several of its methods to adapt the configuration to your needs, and finally
/// call init() with your desired loglevel-specification.
///
/// ## Examples
///
/// ### Use defaults only
///
/// If you initialize flexi_logger with default settings, then it behaves like env_logger:
///
/// ```
/// use flexi_logger::LogOptions;
///
/// LogOptions::new().init(None).unwrap();
/// ```
///
/// ### Write to files, use a detailed log-line format that contains the module and line number
///
/// Here we configure flexi_logger to write log entries with
/// time and location info into a log file in folder "log_files",
/// and we provide the loglevel-specification programmatically:
///
/// ```
/// use flexi_logger::{LogOptions,opt_format};
///
/// LogOptions::new()
///             .log_to_file(true)
///             .directory(Some("log_files".to_string()))
///             .format(opt_format)
///             .init(Some("myprog=debug,mylib=warn".to_string()) )
///             .unwrap_or_else(|e|{panic!("Logger initialization failed with {}",e)});
/// ```
///
pub struct LogOptions(LogConfig);


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
        self.0.rotate_over_size = rotate_over_size;
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
        ::flexi_logger::initialize(self.0, loglevelspec)
    }
}
