use formats;
use Record;

/// Internal struct for influencing the behavior of `flexi_logger`.
///
/// You only need to use this structure explicitly if you want to instantiate multiple loggers
/// in a process.
pub struct LogConfig {
    /// Defines the logline format.
    /// You can either choose between some predefined variants,
    /// ```default_format```, ```opt_format```, ```detailed_format```, ```with_thread```,
    /// or you create and use your own format function with the signature ```fn(&Record) -> String```.
    pub format: fn(&Record) -> String,

    /// * If `false`, the log is written to stderr.
    /// * If `true`, a new file is created and the log is written to it.
    /// 
    /// The default pattern for the filename is '\<program_name\>\_\<date\>\_\<time\>.\<suffix\>',
    ///  e.g. `myprog_2015-07-08_10-44-11.log`.
    ///
    /// Note that all following members are only relevant if this one is set to `true`.
    pub log_to_file: bool,

    /// If `true`, the name of the logfile is documented in a message to stdout.
    pub print_message: bool,

    /// If `true`, all logged error messages are duplicated to stdout.
    pub duplicate_error: bool,

    /// If `true`, all logged warning and info messages are also duplicated to stdout.
    pub duplicate_info: bool,

    /// Allows specifying a directory in which the log files are created.
    pub directory: Option<String>,

    /// Allows specifying the filesystem suffix of the log files (without the dot).
    pub suffix: Option<String>,

    /// Allows specifying whether or not the filename should include a timestamp.
    pub timestamp: bool,

    /// Allows specifying a maximum size for log files in bytes;
    /// when the specified file size is reached or exceeded,
    /// the file will be closed and a new one will be opened.
    /// The filename pattern changes - instead of the timestamp,
    /// the serial number is included into the filename.
    pub rotate_over_size: Option<usize>,

    /// Allows specifying an additional part of the log file name that is inserted after
    /// the program name.
    pub discriminant: Option<String>,

    /// Allows specifying an option to create a symlink to the most recent log file created
    /// using the given name.
    ///
    /// Note that this option is only effective on linux systems.
    pub create_symlink: Option<String>,
}

impl Default for LogConfig {
    fn default() -> Self {
        LogConfig::default_config_for_logger()
    }
}

impl LogConfig {
    /// Default configuration used by Logger.
    ///
    /// *  log_to_file: false
    /// *  print_message: false
    /// *  duplicate_error: false
    /// *  duplicate_info: false
    /// *  format: flexi_logger::default_format
    /// *  no directory: log files (if used) are created where the program was started
    /// *  no rotate_over: log file (if used) grows indefinitely
    /// *  the name of the log file (if used) consists of progname, timestamp, and suffix ```log```
    /// *  no symlink is created
    ///
    pub fn default_config_for_logger() -> LogConfig {
        LogConfig {
            print_message: false,
            duplicate_error: false,
            ..LogConfig::new()
        }
    }

    /// Default configuration used by LogOptions.
    ///
    /// *  log_to_file: false
    /// *  print_message: true
    /// *  duplicate_error: true
    /// *  duplicate_info: false
    /// *  format: flexi_logger::default_format
    /// *  no directory: log files (if used) are created where the program was started
    /// *  no rotate_over: log file (if used) grows indefinitely
    /// *  the name of the log file (if used) consists of progname, timestamp, and suffix ```log```
    /// *  no symlink is created
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
