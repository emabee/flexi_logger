#[cfg(feature = "specfile")]
use log::{debug, error, trace};
#[cfg(feature = "specfile")]
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::collections::HashMap;
#[cfg(feature = "specfile")]
use std::path::Path;
#[cfg(feature = "specfile")]
use std::sync::mpsc::channel;
use std::sync::{Arc, RwLock};
#[cfg(feature = "specfile")]
use std::thread;
#[cfg(feature = "specfile")]
use std::time::Duration;

use crate::flexi_logger::{FlexiLogger, LogSpec};
use crate::primary_writer::PrimaryWriter;
use crate::reconfiguration_handle::reconfiguration_handle;
use crate::writers::{FileLogWriter, FileLogWriterBuilder, LogWriter};
use crate::FormatFunction;
use crate::ReconfigurationHandle;
use crate::{formats, FlexiLoggerError, LogSpecification};

/// The entry-point for using `flexi_logger`.
///
/// A simple example with file logging might look like this:
///
/// ```rust
/// use flexi_logger::{Duplicate,Logger};
///
/// Logger::with_str("info, mycrate = debug")
///         .log_to_file()
///         .duplicate_to_stderr(Duplicate::Warn)
///         .start()
///         .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
///
/// ```
///
///
/// `Logger` is a builder class that allows you to
/// * specify your desired (initial) loglevel-specification
///   * either programmatically as a String
///    ([`Logger::with_str()`](struct.Logger.html#method.with_str))
///   * or by providing a String in the environment
///    ([`Logger::with_env()`](struct.Logger.html#method.with_env)),
///   * or by combining both options
///    ([`Logger::with_env_or_str()`](struct.Logger.html#method.with_env_or_str)),
///   * or by building a `LogSpecification` programmatically
///    ([`Logger::with()`](struct.Logger.html#method.with)),
/// * use the desired configuration methods,
/// * and finally start the logger with
///
///   * [`start()`](struct.Logger.html#method.start),
///   * or [`start_reconfigurable()`](struct.Logger.html#method.start_reconfigurable),
///   * or [`start_with_specfile()`](struct.Logger.html#method.start_with_specfile).
///
pub struct Logger {
    spec: LogSpecification,
    parse_errs: Option<Vec<String>>,
    log_to_file: bool,
    duplicate: Duplicate,
    format: FormatFunction,
    flwb: FileLogWriterBuilder,
    other_writers: HashMap<String, Box<LogWriter>>,
}

/// Choose a way to create a Logger instance and define how to access the (initial)
/// loglevel-specification.
impl Logger {
    /// Creates a Logger that you provide with an explicit LogSpecification.
    /// By default, logs are written with `default_format` to `stderr`.
    pub fn with(logspec: LogSpecification) -> Logger {
        Logger::from_spec_and_errs(logspec, None)
    }

    /// Creates a Logger that reads the LogSpecification from a String or &str.
    /// [See LogSpecification](struct.LogSpecification.html) for the syntax.
    pub fn with_str<S: AsRef<str>>(s: S) -> Logger {
        Logger::from_result(LogSpecification::parse(s.as_ref()))
    }

    /// Creates a Logger that reads the LogSpecification from the environment variable RUST_LOG.
    pub fn with_env() -> Logger {
        Logger::from_result(LogSpecification::env())
    }

    /// Creates a Logger that reads the LogSpecification from the environment variable RUST_LOG,
    /// or derives it from the given String, if RUST_LOG is not set.
    pub fn with_env_or_str<S: AsRef<str>>(s: S) -> Logger {
        Logger::from_result(LogSpecification::env_or_parse(s))
    }

    fn from_spec_and_errs(spec: LogSpecification, parse_errs: Option<Vec<String>>) -> Logger {
        Logger {
            spec,
            parse_errs,
            log_to_file: false,
            duplicate: Duplicate::None,
            format: formats::default_format,
            flwb: FileLogWriter::builder(),
            other_writers: HashMap::<String, Box<LogWriter>>::new(),
        }
    }

    fn from_result(result: Result<LogSpecification, FlexiLoggerError>) -> Logger {
        match result {
            Ok(logspec) => Logger::from_spec_and_errs(logspec, None),
            Err(e) => match e {
                FlexiLoggerError::Parse(parse_errs, logspec) => {
                    Logger::from_spec_and_errs(logspec, Some(parse_errs))
                }
                _ => Logger::from_spec_and_errs(LogSpecification::off(), None),
            },
        }
    }
}

/// Choose a way how to start logging.
impl Logger {
    /// Consumes the Logger object and initializes `flexi_logger`.
    ///
    /// If started this way, the logger cannot be influenced anymore while the program is running.
    /// This is what you want in most of the cases.
    pub fn start(mut self) -> Result<(), FlexiLoggerError> {
        let max = self.spec.max_level();

        log::set_boxed_logger(Box::new(FlexiLogger::new(
            LogSpec::STATIC(self.spec),
            Arc::new(if self.log_to_file {
                self.flwb = self.flwb.format(self.format);
                PrimaryWriter::file(self.duplicate, self.flwb.instantiate()?)
            } else {
                PrimaryWriter::stderr(self.format)
            }),
            self.other_writers,
        )))?;
        log::set_max_level(max);
        Ok(())
    }

    /// Consumes the Logger object and initializes `flexi_logger` in a way that
    /// subsequently the log specification can be updated programmatically.
    ///
    /// This allows e.g. to intensify logging for (buggy) parts of a (test) program, etc.
    ///
    /// See [ReconfigurationHandle](struct.ReconfigurationHandle.html) for an example.
    pub fn start_reconfigurable(mut self) -> Result<ReconfigurationHandle, FlexiLoggerError> {
        let max = self.spec.max_level();
        let spec = Arc::new(RwLock::new(self.spec));

        let primary_writer = Arc::new(if self.log_to_file {
            self.flwb = self.flwb.format(self.format);
            PrimaryWriter::file(self.duplicate, self.flwb.instantiate()?)
        } else {
            PrimaryWriter::stderr(self.format)
        });

        let flexi_logger = FlexiLogger::new(
            LogSpec::DYNAMIC(Arc::clone(&spec)),
            Arc::clone(&primary_writer),
            self.other_writers,
        );

        log::set_boxed_logger(Box::new(flexi_logger))?;
        log::set_max_level(max);
        Ok(reconfiguration_handle(spec, primary_writer))
    }

    /// Consumes the Logger object and initializes `flexi_logger` in a way that
    /// subsequently the log specification can be updated manually.
    ///
    /// Uses the spec that was given to the factory method (`Logger::with()` etc)
    /// as initial spec and then tries to read the logspec from a file.
    ///
    /// If the file does not exist, `flexi_logger` creates the file and fills it
    /// with the initial spec (and in the respective file format, of course).
    ///
    /// ## Feature dependency
    ///
    /// The implementation of this configuration method uses some additional crates
    /// that you might not want to depend on with your program if you don't use this functionality.
    /// For that reason the method is only available if you activate the
    /// `specfile` feature. See `flexi_logger`'s [usage](index.html#usage) section for details.
    ///
    /// ## Usage
    ///
    /// A logger initialization like
    ///
    /// ```ignore
    /// use flexi_logger::Logger;
    ///     Logger::with_str("info")/*...*/.start_with_specfile("logspecification.toml");
    /// ```
    ///
    /// will create the file `logspecification.toml` (if it does not yet exist) with this content:
    ///
    /// ```toml
    /// ### Optional: Default log level
    /// global_level = 'info'
    /// ### Optional: specify a regular expression to suppress all messages that don't match
    /// #global_pattern = 'foo'
    ///
    /// ### Specific log levels per module are optionally defined in this section
    /// [modules]
    /// #'mod1' = 'warn'
    /// #'mod2' = 'debug'
    /// #'mod2::mod3' = 'trace'
    /// ```
    ///
    /// You can subsequently edit and modify the file according to your needs,
    /// while the program is running, and it will immediately take your changes into account.
    ///
    /// Currently only toml-files are supported, the file suffix thus must be `.toml`.
    ///
    /// The initial spec remains valid if the file cannot be read.
    ///
    /// If you update the specfile subsequently while the program is running, `flexi_logger`
    /// re-reads it automatically and adapts its behavior according to the new content.
    /// If the file cannot be read anymore, e.g. because the format is not correct, the
    /// previous logspec remains active.
    /// If the file is corrected subsequently, the log spec update will work again.
    #[cfg(feature = "specfile")]
    pub fn start_with_specfile<P: AsRef<Path>>(self, specfile: P) -> Result<(), FlexiLoggerError> {
        let specfile = specfile.as_ref().to_owned();
        self.spec.ensure_specfile_is_valid(&specfile)?;
        let mut handle = self.start_reconfigurable()?;

        // now setup fs notification to automatically reread the file, and initialize from the file
        thread::Builder::new().spawn(move || {
            // Create a channel to receive the events.
            let (tx, rx) = channel();
            // Create a watcher object, delivering debounced events
            let mut watcher = match watcher(tx, Duration::from_millis(800)) {
                Ok(w) => w,
                Err(e) => {
                    error!("watcher() failed with {:?}", e);
                    return;
                }
            };

            // watch the spec file
            match watcher.watch(&specfile, RecursiveMode::NonRecursive) {
                Err(e) => {
                    error!(
                        "watcher.watch() failed for the log specification file {:?}, caused by {:?}",
                        specfile, e
                    );
                    ::std::process::exit(-1);
                }
                Ok(_) => {
                    // initial read of the file: if that fails, just print an error and continue
                    match LogSpecification::file(&specfile) {
                        Ok(spec) => handle.set_new_spec(spec),
                        Err(e) => error!("Can't read the log specification file, due to {:?}", e),
                    }

                    loop {
                        match rx.recv() {
                            Ok(DebouncedEvent::Write(_)) => {
                                debug!("Got Write event");
                                match LogSpecification::file(&specfile) {
                                    Ok(spec) => handle.set_new_spec(spec),
                                    Err(e) => eprintln!(
                                        "Continuing with current log specification \
                                         because the log specification file is not readable, \
                                         due to {:?}",
                                        e
                                    ),
                                }
                            }
                            Ok(_event) => trace!("ignoring event {:?}", _event),
                            Err(e) => error!("watch error: {:?}", e),
                        }
                    }
                }
            }
        })?;

        Ok(())
    }
}

/// Simple methods for influencing the behavior of the Logger.
impl Logger {
    /// Allows verifying that no parsing errors have occured in the used factory method,
    /// and examining the parse error.
    ///
    /// The factory methods `Logger::with_str()`, `Logger::with_env()`,
    /// and `Logger::with_env_or_str()`,
    /// parse a log specification String, and deduce from it a `LogSpecification` object.
    /// Parsing errors are reported to stdout, but effectively ignored; in worst case, a
    /// LogSpecification might be used that turns off logging completely!
    ///
    /// This method gives programmatic access to parse errors, if there were any.
    ///
    /// In the following example we just panic if the spec was not free of errors:
    ///
    /// ```rust
    /// # use flexi_logger::Logger;
    /// # let some_log_spec_string = "hello";
    /// Logger::with_str(some_log_spec_string)
    /// .check_parser_error()
    /// .unwrap()
    /// .log_to_file()
    /// .start();
    /// ```
    pub fn check_parser_error(self) -> Result<Logger, FlexiLoggerError> {
        match self.parse_errs {
            Some(parse_errs) => Err(FlexiLoggerError::Parse(parse_errs, self.spec)),
            None => Ok(self),
        }
    }

    /// Makes the logger write all logs to a file, rather than to stderr.
    ///
    /// The default pattern for the filename is '\<program_name\>\_\<date\>\_\<time\>.\<suffix\>',
    ///  e.g. `myprog_2015-07-08_10-44-11.log`.
    pub fn log_to_file(mut self) -> Logger {
        self.log_to_file = true;
        self
    }

    /// Makes the logger print an info message to stdout with the name of the logfile
    /// when a logfile is opened for writing.
    pub fn print_message(mut self) -> Logger {
        self.flwb = self.flwb.print_message();
        self
    }

    /// Makes the logger write messages with the specified minimum severity additionally to stderr.
    pub fn duplicate_to_stderr(mut self, dup: Duplicate) -> Logger {
        self.duplicate = dup;
        self
    }

    /// Makes the logger write all logged error messages additionally to stderr.
    #[deprecated(note = "use duplicate_to_stderr(dup: Duplicate)")]
    pub fn duplicate_error(mut self) -> Logger {
        self.duplicate = Duplicate::Error;
        self
    }

    /// Makes the logger write all logged error, warning, and info messages additionally to stderr.
    #[deprecated(note = "use duplicate_to_stderr(dup: Duplicate)")]
    pub fn duplicate_info(mut self) -> Logger {
        self.duplicate = Duplicate::Info;
        self
    }

    /// Makes the logger use the provided format function for the log entries,
    /// rather than [formats::default_format](fn.default_format.html).
    ///
    /// You can either choose between some predefined variants,
    /// ```default_format```, ```opt_format```, ```detailed_format```, ```with_thread```,
    /// or you create and use your own format function
    /// with the signature ```fn(&Record) -> String```.
    pub fn format(mut self, format: FormatFunction) -> Logger {
        self.format = format;
        self
    }

    /// Specifies a folder for the log files.
    ///
    /// This parameter only has an effect if `log_to_file()` is used, too.
    /// If the specified folder does not exist, the initialization will fail.
    /// By default, the log files are created in the folder where the program was started.
    pub fn directory<S: Into<String>>(mut self, directory: S) -> Logger {
        self.flwb = self.flwb.directory(directory);
        self
    }

    /// Specifies a suffix for the log files.
    ///
    /// This parameter only has an effect if `log_to_file()` is used, too.
    pub fn suffix<S: Into<String>>(mut self, suffix: S) -> Logger {
        self.flwb = self.flwb.suffix(suffix);
        self
    }

    /// Makes the logger not include a timestamp into the names of the log files.
    ///
    /// This option only has an effect if `log_to_file()` is used, too.
    pub fn suppress_timestamp(mut self) -> Logger {
        self.flwb = self.flwb.suppress_timestamp();
        self
    }

    /// Prevents indefinite growth of log files.
    ///
    /// For details, see
    /// [FileLogWriterBuilder::rotate_over_size](./writers/struct.FileLogWriterBuilder.html#method.rotate_over_size).
    pub fn rotate_over_size(mut self, rotate_over_size: usize) -> Logger {
        self.flwb = self
            .flwb
            .rotate_over_size(rotate_over_size)
            .o_timestamp(false);
        self
    }

    /// Makes the logger append to the specified output file, if it exists already;
    /// by default, the file would be truncated.
    ///
    /// This option only has an effect if `log_to_file()` is used, too.
    /// This option will hardly make an effect if `suppress_timestamp()` is not used.
    pub fn append(mut self) -> Logger {
        self.flwb = self.flwb.append();
        self
    }

    /// The specified String is added to the log file name after the program name.
    ///
    /// This option only has an effect if `log_to_file()` is used, too.
    pub fn discriminant<S: Into<String>>(mut self, discriminant: S) -> Logger {
        self.flwb = self.flwb.discriminant(discriminant);
        self
    }

    /// The specified String will be used on linux systems to create in the current folder
    /// a symbolic link to the current log file.
    ///
    /// This option only has an effect if `log_to_file()` is used, too.
    pub fn create_symlink<S: Into<String>>(mut self, symlink: S) -> Logger {
        self.flwb = self.flwb.create_symlink(symlink);
        self
    }

    /// Registers a LogWriter implementation under the given target name.
    ///
    /// The target name should not start with an underscore.
    ///
    /// See [the module documentation of `writers`](writers/index.html).
    pub fn add_writer<S: Into<String>>(mut self, name: S, writer: Box<LogWriter>) -> Logger {
        self.other_writers.insert(name.into(), writer);
        self
    }
}

/// Alternative set of methods to control the behavior of the Logger.
/// Use these methods when you want to control the settings flexibly,
/// e.g. with commandline arguments via `docopts` or `clap`.
impl Logger {
    /// With true, makes the logger write all logs to a file, otherwise to stderr.
    pub fn o_log_to_file(mut self, log_to_file: bool) -> Logger {
        self.log_to_file = log_to_file;
        self
    }

    /// With true, makes the logger print an info message to stdout, each time
    /// when a new file is used for log-output.
    pub fn o_print_message(mut self, print_message: bool) -> Logger {
        self.flwb = self.flwb.o_print_message(print_message);
        self
    }

    /// With true, makes the logger write all logged error messages additionally to stderr;
    /// with false, no messages are duplicated.
    #[deprecated(note = "use duplicate_to_stderr(dup: Duplicate)")]
    pub fn o_duplicate_error(mut self, duplicate_error: bool) -> Logger {
        if duplicate_error {
            self.duplicate = Duplicate::Error;
        } else {
            self.duplicate = Duplicate::None;
        }
        self
    }

    /// With true, makes the logger write all logged error, warning,
    /// and info messages additionally to stderr;
    /// with false, no messages are duplicated.
    #[deprecated(note = "use duplicate_to_stderr(dup: Duplicate)")]
    pub fn o_duplicate_info(mut self, duplicate_info: bool) -> Logger {
        if duplicate_info {
            self.duplicate = Duplicate::Info;
        } else {
            self.duplicate = Duplicate::None;
        }
        self
    }

    /// Specifies a folder for the log files.
    ///
    /// This parameter only has an effect if `log_to_file` is set to true.
    /// If the specified folder does not exist, the initialization will fail.
    /// With None, the log files are created in the folder where the program was started.
    pub fn o_directory<S: Into<String>>(mut self, directory: Option<S>) -> Logger {
        self.flwb = self.flwb.o_directory(directory);
        self
    }

    /// This option only has an effect if `log_to_file` is set to true.
    ///
    /// By default, and with None, the log file will grow indefinitely.
    /// If a size is set, when the log file reaches or exceeds the specified size,
    /// the file will be closed and a new file will be opened.
    /// Also the filename pattern changes: instead of the timestamp, a serial number
    /// is included into the filename.
    ///
    /// The size is given in bytes, e.g. `o_rotate_over_size(Some(1_000))` will rotate
    /// files once they reach a size of 1 kB.
    pub fn o_rotate_over_size(mut self, rotate_over_size: Option<usize>) -> Logger {
        self.flwb = self
            .flwb
            .o_rotate_over_size(rotate_over_size)
            .o_timestamp(rotate_over_size.is_none());
        self
    }

    /// With true, makes the logger include a timestamp into the names of the log files.
    /// `true` is the default, but `rotate_over_size` sets it to `false`.
    /// With this method you can set it to `true` again.
    ///
    /// This parameter only has an effect if `log_to_file` is set to true.
    pub fn o_timestamp(mut self, timestamp: bool) -> Logger {
        self.flwb = self.flwb.o_timestamp(timestamp);
        self
    }

    /// This option only has an effect if `log_to_file` is set to true.
    ///
    /// If append is set to true, makes the logger append to the specified output file, if it exists.
    /// By default, or with false, the file would be truncated.
    ///
    /// This option will hardly make an effect if `suppress_timestamp()` is not used.

    pub fn o_append(mut self, append: bool) -> Logger {
        self.flwb = self.flwb.o_append(append);
        self
    }

    /// This option only has an effect if `log_to_file` is set to true.
    ///
    /// The specified String is added to the log file name.
    pub fn o_discriminant<S: Into<String>>(mut self, discriminant: Option<S>) -> Logger {
        self.flwb = self.flwb.o_discriminant(discriminant);
        self
    }

    /// This option only has an effect if `log_to_file` is set to true.
    ///
    /// If a String is specified, it will be used on linux systems to create in the current folder
    /// a symbolic link with this name to the current log file.
    pub fn o_create_symlink<S: Into<String>>(mut self, symlink: Option<S>) -> Logger {
        self.flwb = self.flwb.o_create_symlink(symlink);
        self
    }
}

/// Used to control which messages are to be duplicated to stderr, when log_to_file() is used.
pub enum Duplicate {
    /// No messages are duplicated.
    None,
    /// Only error messages are duplicated.
    Error,
    /// Error and warn messages are duplicated.
    Warn,
    /// Error, warn, and info messages are duplicated.
    Info,
    /// Error, warn, info, and debug messages are duplicated.
    Debug,
    /// All messages are duplicated.
    Trace,
    /// All messages are duplicated.
    All,
}
