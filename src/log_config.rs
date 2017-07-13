use formats;
use LogRecord;

/// Internal struct for influencing the behavior of flexi_logger.
///
/// You only use this structure explicitly if you want to instantiate multiple loggers in a process.
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
