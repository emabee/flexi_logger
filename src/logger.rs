#[cfg(feature = "specfile")]
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, RwLock};

use crate::flexi_logger::FlexiLogger;
use crate::primary_writer::PrimaryWriter;
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
///   * or [`start_with_specfile()`](struct.Logger.html#method.start_with_specfile).
///
pub struct Logger {
    spec: LogSpecification,
    parse_errs: Option<Vec<String>>,
    log_target: LogTarget,
    duplicate: Duplicate,
    format_for_file: FormatFunction,
    format_for_stderr: FormatFunction,
    format_for_writer: FormatFunction,
    flwb: FileLogWriterBuilder,
    other_writers: HashMap<String, Box<dyn LogWriter>>,
}

/// Describes the default log target.
///
/// All log messages, in which no target is explicitly defined, will be written to
/// the default log target.
///
/// See the [writers](writers/index.html) module for
/// how to specify non-default log targets in log macro calls,
/// and the usage of non-default log writers.
pub enum LogTarget {
    /// Log is written to stderr.
    ///
    /// This is the default behavior of `flexi_logger`.
    StdErr,
    /// Log is written to stdout.
    StdOut,
    /// Log is written to a file.
    ///
    /// The default pattern for the filename is '\<program_name\>\_\<date\>\_\<time\>.\<suffix\>',
    ///  e.g. `myprog_2015-07-08_10-44-11.log`.
    File,
    /// Log is written to an alternative `LogWriter` implementation.
    ///
    Writer(Box<dyn LogWriter>),
    /// Log is written to a file, as with `LogTarget::File`, _and_ to an alternative
    /// `LogWriter` implementation.
    FileAndWriter(Box<dyn LogWriter>),
    /// Log is processed as if it were written, but is finally not written.
    ///
    /// This can be useful for running tests with all log-levels active to ensure that the log calls
    /// which are normally not active will not cause undesired side-effects when activated
    /// (note that the log macros may prevent arguments of inactive log-calls from being evaluated).
    DevNull,
}

/// Create a Logger instance and define how to access the (initial)
/// loglevel-specification.
impl Logger {
    /// Creates a Logger that you provide with an explicit `LogSpecification`.
    /// By default, logs are written with `default_format` to `stderr`.
    #[must_use]
    pub fn with(logspec: LogSpecification) -> Self {
        Self::from_spec_and_errs(logspec, None)
    }

    /// Creates a Logger that reads the `LogSpecification` from a String or &str.
    /// [See `LogSpecification`](struct.LogSpecification.html) for the syntax.
    #[must_use]
    pub fn with_str<S: AsRef<str>>(s: S) -> Self {
        Self::from_result(LogSpecification::parse(s.as_ref()))
    }

    /// Creates a Logger that reads the `LogSpecification` from the environment variable `RUST_LOG`.
    #[must_use]
    pub fn with_env() -> Self {
        Self::from_result(LogSpecification::env())
    }

    /// Creates a Logger that reads the `LogSpecification` from the environment variable `RUST_LOG`,
    /// or derives it from the given String, if `RUST_LOG` is not set.
    #[must_use]
    pub fn with_env_or_str<S: AsRef<str>>(s: S) -> Self {
        Self::from_result(LogSpecification::env_or_parse(s))
    }

    fn from_spec_and_errs(spec: LogSpecification, parse_errs: Option<Vec<String>>) -> Self {
        #[cfg(feature = "colors")]
        let default_format = formats::colored_default_format;
        #[cfg(not(feature = "colors"))]
        let default_format = formats::default_format;

        Self {
            spec,
            parse_errs,
            log_target: LogTarget::StdErr,
            duplicate: Duplicate::None,
            format_for_file: default_format,
            format_for_stderr: default_format,
            format_for_writer: default_format,
            flwb: FileLogWriter::builder(),
            other_writers: HashMap::<String, Box<dyn LogWriter>>::new(),
        }
    }

    fn from_result(result: Result<LogSpecification, FlexiLoggerError>) -> Self {
        match result {
            Ok(logspec) => Self::from_spec_and_errs(logspec, None),
            Err(e) => match e {
                FlexiLoggerError::Parse(parse_errs, logspec) => {
                    Self::from_spec_and_errs(logspec, Some(parse_errs))
                }
                _ => Self::from_spec_and_errs(LogSpecification::off(), None),
            },
        }
    }
}

/// Simple methods for influencing the behavior of the Logger.
impl Logger {
    /// Allows verifying that no parsing errors have occured in the used factory method,
    /// and examining the parse error.
    ///
    /// Most of the factory methods for Logger (`Logger::with_...()`)
    /// parse a log specification String, and deduce from it a `LogSpecification` object.
    /// If parsing fails, errors are reported to stdout, but effectively ignored.
    /// In worst case, nothing is logged!
    ///
    /// This method gives programmatic access to parse errors, if there were any, so that errors
    /// don't happen unnoticed.
    ///
    /// In the following example we just panic if the spec was not free of errors:
    ///
    /// ```should_panic
    /// # use flexi_logger::{Logger,LogTarget};
    /// Logger::with_str("hello world")
    /// .check_parser_error()
    /// .unwrap()       // <-- here we could do better than panic
    /// .log_target(LogTarget::File)
    /// .start();
    /// ```
    ///
    /// # Errors
    ///
    /// `FlexiLoggerError::Parse` if the input for the log specification is malformed.
    pub fn check_parser_error(self) -> Result<Self, FlexiLoggerError> {
        match self.parse_errs {
            Some(parse_errs) => Err(FlexiLoggerError::Parse(parse_errs, self.spec)),
            None => Ok(self),
        }
    }

    /// Is equivalent to
    /// [`log_target`](struct.Logger.html#method.log_target)`(`[`LogTarget::File`](
    /// enum.LogTarget.html#variant.File)`)`.
    pub fn log_to_file(mut self) -> Self {
        self.log_target = LogTarget::File;
        self
    }

    /// Write the main log output to the specified target.
    ///
    /// By default, i.e. if this method is not called, the standard output goes to `stderr`.
    pub fn log_target(mut self, log_target: LogTarget) -> Self {
        self.log_target = log_target;
        self
    }

    /// Makes the logger write no logs at all.
    ///
    /// This can be useful when you want to run tests of your programs with all log-levels active.
    /// Such tests can ensure that those parts of your code, which are only executed
    /// within normally unused log calls (like `std::fmt::Display` implementations),
    /// will not cause undesired side-effects when activated (note that the log macros prevent
    /// arguments of inactive log-calls from being evaluated).
    pub fn do_not_log(mut self) -> Self {
        self.log_target = LogTarget::DevNull;
        self
    }

    /// Makes the logger print an info message to stdout with the name of the logfile
    /// when a logfile is opened for writing.
    pub fn print_message(mut self) -> Self {
        self.flwb = self.flwb.print_message();
        self
    }

    /// Makes the logger write messages with the specified minimum severity additionally to stderr.
    pub fn duplicate_to_stderr(mut self, dup: Duplicate) -> Self {
        self.duplicate = dup;
        self
    }

    /// Makes the logger use the provided format function for all messages
    /// that are written to files or to stderr or to an additional writer.
    ///
    /// You can either choose one of the provided log-line formatters,
    /// or you create and use your own format function with the signature <br>
    /// ```fn(&Record) -> String```.
    ///
    /// By default,
    /// `default_format()` is used for the output to files and
    /// `colored_default_format()` is used for the output to stderr.
    ///
    /// If the feature `colors` is switched off,
    /// `default_format()` is used for all outputs.
    pub fn format(mut self, format: FormatFunction) -> Self {
        self.format_for_file = format;
        self.format_for_stderr = format;
        self.format_for_writer = format;
        self
    }

    /// Makes the logger use the provided format function for messages
    /// that are written to files.
    ///
    /// Regarding the default, see [`Logger::format`](struct.Logger.html#method.format).
    pub fn format_for_files(mut self, format: FormatFunction) -> Self {
        self.format_for_file = format;
        self
    }

    /// Makes the logger use the provided format function for messages
    /// that are written to stderr or to stdout.
    ///
    /// Regarding the default, see [`Logger::format`](struct.Logger.html#method.format).
    pub fn format_for_stderr(mut self, format: FormatFunction) -> Self {
        self.format_for_stderr = format;
        self
    }

    /// Allows specifying a format function for an additional writer.
    /// Note that it is up to the implementation of the additional writer
    /// whether it evaluates this setting or not.
    ///
    /// Regarding the default, see [`Logger::format`](struct.Logger.html#method.format).
    pub fn format_for_writer(mut self, format: FormatFunction) -> Self {
        self.format_for_writer = format;
        self
    }

    /// Specifies a folder for the log files.
    ///
    /// This parameter only has an effect if `log_to_file()` is used, too.
    /// If the specified folder does not exist, the initialization will fail.
    /// By default, the log files are created in the folder where the program was started.
    pub fn directory<S: Into<PathBuf>>(mut self, directory: S) -> Self {
        self.flwb = self.flwb.directory(directory);
        self
    }

    /// Specifies a suffix for the log files.
    ///
    /// This parameter only has an effect if `log_to_file()` is used, too.
    pub fn suffix<S: Into<String>>(mut self, suffix: S) -> Self {
        self.flwb = self.flwb.suffix(suffix);
        self
    }

    /// Makes the logger not include a timestamp into the names of the log files.
    ///
    /// This option only has an effect if `log_to_file()` is used, too.
    pub fn suppress_timestamp(mut self) -> Self {
        self.flwb = self.flwb.suppress_timestamp();
        self
    }

    /// When rotation is used with some `Cleanup` variant, then this option defines
    /// if the cleanup activities (finding files, deleting files, evtl zipping files) is done in
    /// the current thread (in the current log-call), or whether cleanup is delegated to a
    /// background thread.
    ///
    /// As of `flexi_logger` version `0.14.7`,
    /// the cleanup activities are done by default in a background thread.
    /// This minimizes the blocking impact to your application caused by IO operations.
    ///
    /// In earlier versions of `flexi_logger`, or if you call this method with
    /// `use_background_thread = false`,
    /// the cleanup is done in the thread that is currently causing a file rotation.
    #[must_use]
    pub fn cleanup_in_background_thread(mut self, use_background_thread: bool) -> Self {
        self.flwb = self
            .flwb
            .cleanup_in_background_thread(use_background_thread);
        self
    }

    /// Prevent indefinite growth of the log file by applying file rotation
    /// and a clean-up strategy for older log files.
    ///
    /// By default, the log file is fixed while your program is running and will grow indefinitely.
    /// With this option being used, when the log file reaches the specified criterion,
    /// the file will be closed and a new file will be opened.
    ///
    /// Note that also the filename pattern changes:
    ///
    /// - by default, no timestamp is added to the filename
    /// - the logs are always written to a file with infix `_rCURRENT`
    /// - when the rotation criterion is fulfilled, it is closed and renamed to a file
    ///   with another infix (see `Naming`),
    ///   and then the logging continues again to the (fresh) file with infix `_rCURRENT`.
    ///
    /// Example:
    ///
    /// After some logging with your program `my_prog` and rotation with `Naming::Numbers`,
    /// you will find files like
    ///
    /// ```text
    /// my_prog_r00000.log
    /// my_prog_r00001.log
    /// my_prog_r00002.log
    /// my_prog_rCURRENT.log
    /// ```
    ///
    /// ## Parameters
    ///
    /// `rotate_over_size` is given in bytes, e.g. `10_000_000` will rotate
    /// files once they reach a size of 10 MiB.
    ///     
    /// `cleanup` defines the strategy for dealing with older files.
    /// See [Cleanup](enum.Cleanup.html) for details.
    pub fn rotate(mut self, criterion: Criterion, naming: Naming, cleanup: Cleanup) -> Self {
        self.flwb = self.flwb.rotate(criterion, naming, cleanup);
        self
    }

    /// Makes the logger append to the specified output file, if it exists already;
    /// by default, the file would be truncated.
    ///
    /// This option only has an effect if `log_to_file()` is used, too.
    /// This option will hardly make an effect if `suppress_timestamp()` is not used.
    pub fn append(mut self) -> Self {
        self.flwb = self.flwb.append();
        self
    }

    /// The specified String is added to the log file name after the program name.
    ///
    /// This option only has an effect if `log_to_file()` is used, too.
    pub fn discriminant<S: Into<String>>(mut self, discriminant: S) -> Self {
        self.flwb = self.flwb.discriminant(discriminant);
        self
    }

    /// The specified path will be used on linux systems to create a symbolic link
    /// to the current log file.
    ///
    /// This option has no effect on filesystems where symlinks are not supported,
    /// and it only has an effect if `log_to_file()` is used, too.
    ///
    /// ### Example
    ///
    /// You can use the symbolic link to follow the log output with `tail`,
    /// even if the log files are rotated.
    ///
    /// Assuming the link has the name `link_to_log_file`, then use:
    ///
    /// ```text
    /// tail --follow=name --max-unchanged-stats=1 --retry link_to_log_file
    /// ```
    ///
    pub fn create_symlink<P: Into<PathBuf>>(mut self, symlink: P) -> Self {
        self.flwb = self.flwb.create_symlink(symlink);
        self
    }

    /// Registers a `LogWriter` implementation under the given target name.
    ///
    /// The target name must not start with an underscore.
    ///
    /// See [the module documentation of `writers`](writers/index.html).
    pub fn add_writer<S: Into<String>>(
        mut self,
        target_name: S,
        writer: Box<dyn LogWriter>,
    ) -> Self {
        self.other_writers.insert(target_name.into(), writer);
        self
    }

    /// Use Windows line endings, rather than just `\n`.
    pub fn use_windows_line_ending(mut self) -> Self {
        self.flwb = self.flwb.use_windows_line_ending();
        self
    }
}

/// Alternative set of methods to control the behavior of the Logger.
/// Use these methods when you want to control the settings flexibly,
/// e.g. with commandline arguments via `docopts` or `clap`.
impl Logger {
    /// With true, makes the logger write all logs to a file, otherwise to stderr.
    #[deprecated(since = "0.13.3", note = "please use `log_target` instead")]
    pub fn o_log_to_file(mut self, log_to_file: bool) -> Self {
        if log_to_file {
            self.log_target = LogTarget::File;
        } else {
            self.log_target = LogTarget::StdErr;
        }
        self
    }

    /// With true, makes the logger print an info message to stdout, each time
    /// when a new file is used for log-output.
    pub fn o_print_message(mut self, print_message: bool) -> Self {
        self.flwb = self.flwb.o_print_message(print_message);
        self
    }

    /// Specifies a folder for the log files.
    ///
    /// This parameter only has an effect if `log_to_file` is set to true.
    /// If the specified folder does not exist, the initialization will fail.
    /// With None, the log files are created in the folder where the program was started.
    pub fn o_directory<P: Into<PathBuf>>(mut self, directory: Option<P>) -> Self {
        self.flwb = self.flwb.o_directory(directory);
        self
    }

    /// By default, and with None, the log file will grow indefinitely.
    /// If a `rotate_config` is set, when the log file reaches or exceeds the specified size,
    /// the file will be closed and a new file will be opened.
    /// Also the filename pattern changes: instead of the timestamp, a serial number
    /// is included into the filename.
    ///
    /// The size is given in bytes, e.g. `o_rotate_over_size(Some(1_000))` will rotate
    /// files once they reach a size of 1 kB.
    ///
    /// The cleanup strategy allows delimiting the used space on disk.
    pub fn o_rotate(mut self, rotate_config: Option<(Criterion, Naming, Cleanup)>) -> Self {
        self.flwb = self.flwb.o_rotate(rotate_config);
        self
    }

    /// With true, makes the logger include a timestamp into the names of the log files.
    /// `true` is the default, but `rotate_over_size` sets it to `false`.
    /// With this method you can set it to `true` again.
    ///
    /// This parameter only has an effect if `log_to_file` is set to true.
    pub fn o_timestamp(mut self, timestamp: bool) -> Self {
        self.flwb = self.flwb.o_timestamp(timestamp);
        self
    }

    /// This option only has an effect if `log_to_file` is set to true.
    ///
    /// If append is set to true, makes the logger append to the specified output file, if it exists.
    /// By default, or with false, the file would be truncated.
    ///
    /// This option will hardly make an effect if `suppress_timestamp()` is not used.

    pub fn o_append(mut self, append: bool) -> Self {
        self.flwb = self.flwb.o_append(append);
        self
    }

    /// This option only has an effect if `log_to_file` is set to true.
    ///
    /// The specified String is added to the log file name.
    pub fn o_discriminant<S: Into<String>>(mut self, discriminant: Option<S>) -> Self {
        self.flwb = self.flwb.o_discriminant(discriminant);
        self
    }

    /// This option only has an effect if `log_to_file` is set to true.
    ///
    /// If a String is specified, it will be used on linux systems to create in the current folder
    /// a symbolic link with this name to the current log file.
    pub fn o_create_symlink<P: Into<PathBuf>>(mut self, symlink: Option<P>) -> Self {
        self.flwb = self.flwb.o_create_symlink(symlink);
        self
    }
}

/// Finally, start logging, optionally with a spec-file.
impl Logger {
    /// Consumes the Logger object and initializes `flexi_logger`.
    ///
    /// The returned reconfiguration handle allows updating the log specification programmatically
    /// later on, e.g. to intensify logging for (buggy) parts of a (test) program, etc.
    /// See [`ReconfigurationHandle`](struct.ReconfigurationHandle.html) for an example.
    ///
    /// # Errors
    ///
    /// Several variants of `FlexiLoggerError` can occur.
    pub fn start(mut self) -> Result<ReconfigurationHandle, FlexiLoggerError> {
        let max_level = self.spec.max_level();
        let spec = Arc::new(RwLock::new(self.spec));
        let other_writers = Arc::new(self.other_writers);

        let primary_writer = Arc::new(match self.log_target {
            LogTarget::File => {
                self.flwb = self.flwb.format(self.format_for_file);
                PrimaryWriter::multi(
                    self.duplicate,
                    self.format_for_stderr,
                    vec![Box::new(self.flwb.try_build()?)],
                )
            }
            LogTarget::Writer(mut w) => {
                w.format(self.format_for_writer);
                PrimaryWriter::multi(self.duplicate, self.format_for_stderr, vec![w])
            }
            LogTarget::FileAndWriter(mut w) => {
                self.flwb = self.flwb.format(self.format_for_file);
                w.format(self.format_for_writer);
                PrimaryWriter::multi(
                    self.duplicate,
                    self.format_for_stderr,
                    vec![Box::new(self.flwb.try_build()?), w],
                )
            }
            LogTarget::StdOut => PrimaryWriter::stdout(self.format_for_stderr),
            LogTarget::StdErr => PrimaryWriter::stderr(self.format_for_stderr),
            LogTarget::DevNull => PrimaryWriter::black_hole(self.duplicate, self.format_for_stderr),
        });

        let flexi_logger = FlexiLogger::new(
            Arc::clone(&spec),
            Arc::clone(&primary_writer),
            Arc::clone(&other_writers),
        );

        log::set_boxed_logger(Box::new(flexi_logger))?;
        let handle = ReconfigurationHandle::new(spec, primary_writer, other_writers);
        handle.reconfigure(max_level);
        Ok(handle)
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
    ///
    /// # Errors
    ///
    /// Several variants of `FlexiLoggerError` can occur.
    ///
    /// # Returns
    ///
    /// A `ReconfigurationHandle` is returned, predominantly to allow using its
    /// [`shutdown`](struct.ReconfigurationHandle.html#method.shutdown) method.
    #[cfg(feature = "specfile")]
    pub fn start_with_specfile<P: AsRef<std::path::Path>>(
        self,
        specfile: P,
    ) -> Result<ReconfigurationHandle, FlexiLoggerError> {
        // Make logging work, before caring for the specfile
        let mut handle = self.start()?;
        let handle2 = handle.clone();
        let specfile = specfile.as_ref().to_owned();

        handle.synchronize_with_specfile(&specfile)?;

        // Now that the file exists, we can canonicalize the path
        let specfile = specfile.canonicalize().map_err(FlexiLoggerError::Io)?;

        // Watch the parent folder of the specfile, using debounced events
        let (tx, rx) = std::sync::mpsc::channel();
        let debouncing_delay = std::time::Duration::from_millis(1000);
        let mut watcher = watcher(tx, debouncing_delay)?;
        watcher.watch(&specfile.parent().unwrap(), RecursiveMode::NonRecursive)?;

        // in a separate thread, reread the specfile when it was updated
        std::thread::Builder::new()
            .name("flexi_logger-specfile-watcher".to_string())
            .stack_size(128 * 1024)
            .spawn(move || {
                let _anchor_for_watcher = watcher; // keep it alive!
                loop {
                    match rx.recv() {
                        Ok(debounced_event) => {
                            // println!("got debounced event {:?}", debounced_event);
                            match debounced_event {
                                DebouncedEvent::Create(ref path)
                                | DebouncedEvent::Write(ref path) => {
                                    if path.canonicalize().unwrap() == specfile {
                                        match LogSpecification::try_from_file(&specfile) {
                                            Ok(spec) => handle.set_new_spec(spec),
                                            Err(e) => eprintln!(
                                            "[flexi_logger] rereading the log specification file \
                                         failed with {:?}, \
                                         continuing with previous log specification",
                                            e
                                        ),
                                        }
                                    }
                                }
                                _event => {}
                            }
                        }
                        Err(e) => {
                            eprintln!("[flexi_logger] error while watching the specfile: {:?}", e)
                        }
                    }
                }
            })?;

        Ok(handle2)
    }
}

/// Criterion when to rotate the log file.
///
/// Used in [`Logger::rotate`](struct.Logger.html#method.rotate).
pub enum Criterion {
    /// Rotate the log file when it exceeds the specified size in bytes.
    Size(u64),
    /// Rotate the log file when it has become older than the specified age.
    ///
    /// ## Minor limitation
    ///
    /// ### TL,DR
    /// the combination of `Logger::append()`
    /// with `Criterion::Age` works OK, but not perfectly correct on Windows or Linux
    /// when the program is restarted.
    ///
    /// ### Details
    /// Applying the age criterion works fine while your program is running.
    /// Ideally, we should also apply it to the rCURRENT file when the program is restarted
    /// and you chose the `Logger::append()` option.
    ///
    /// Unfortunately, this does not work on Windows, and it does not work on linux,
    /// for different reasons.
    ///
    /// To minimize the impact on age-based file-rotation,
    /// `flexi_logger` uses on Windows and linux its initialization time
    /// rather than the real file property
    /// as the created_at-info of an rCURRENT file that already exists, and the
    /// current timestamp when file rotation happens during further execution.
    /// Consequently, a left-over rCURRENT file from a previous program run will look newer
    /// than it is, and will be used longer than it should be.
    ///
    /// #### Issue on Windows
    ///
    /// For compatibility with DOS (sic!), Windows magically transfers the created_at-info
    /// of a file that is deleted (or renamed) to its successor,
    /// when the recreation happens within some seconds [[1]](#ref-1).
    ///
    /// <a name="ref-1">[1]</a> [https://superuser.com/questions/966490/windows-7-what-is-date-created-file-property-referring-to](https://superuser.com/questions/966490/windows-7-what-is-date-created-file-property-referring-to).
    ///
    /// If the file property were used by `flexi_logger`,
    /// the rCURRENT file would always appear to be as old as the
    /// first one that ever was created - rotation by time would completely fail.
    ///
    /// #### Issue on Linux
    ///
    /// `std::fs::metadata.created()` returns `Err`, because linux does not maintain a
    /// created-at-timestamp.
    ///
    Age(Age),
}

/// The age after which a log file rotation will be triggered,
/// when [`Criterion::Age`](enum.Criterion.html#variant.Age) is chosen.
#[derive(Copy, Clone)]
pub enum Age {
    /// Rotate the log file when the local clock has started a new day since the
    /// current file had been created.
    Day,
    /// Rotate the log file when the local clock has started a new hour since the
    /// current file had been created.
    Hour,
    /// Rotate the log file when the local clock has started a new minute since the
    /// current file had been created.
    Minute,
    /// Rotate the log file when the local clock has started a new second since the
    /// current file had been created.
    Second,
}

/// The naming convention for rotated log files.
///
/// With file rotation, the logs are written to a file with infix `_rCURRENT`.
/// When rotation happens, the CURRENT log file will be renamed to a file with
/// another infix of the form `"_r..."`. `Naming` defines which other infix will be used.
///
/// Used in [`Logger::rotate`](struct.Logger.html#method.rotate).
#[derive(Copy, Clone)]
pub enum Naming {
    /// File rotation rotates to files with a timestamp-infix, like `"r2020-01-27_14-41-08"`.
    Timestamps,
    /// File rotation rotates to files with a number-infix.
    Numbers,
}

/// Defines the strategy for handling older log files.
///
/// Is used in [`Logger::rotate`](struct.Logger.html#method.rotate).
///
/// Note that if you use a strategy other than `Cleanup::Never`, then the cleanup work is
/// by default done in an extra thread, to minimize the impact on the program.
/// See
/// [`Logger::cleanup_in_background_thread`](struct.Logger.html#method.cleanup_in_background_thread)
/// if you want to control whether this extra thread is created and used.
#[derive(Copy, Clone)]
pub enum Cleanup {
    /// Older log files are not touched - they remain for ever.
    Never,
    /// The specified number of rotated log files are kept.
    /// Older files are deleted, if necessary.
    KeepLogFiles(usize),
    /// The specified number of rotated log files are zipped and kept.
    /// Older files are deleted, if necessary.
    ///
    /// This option is only available with feature `ziplogs`.
    #[cfg(feature = "ziplogs")]
    KeepZipFiles(usize),
    /// Allows keeping some files as text files and some as zip files.
    ///
    /// Is used in
    ///
    /// ## Example
    ///
    /// `KeepLogAndZipFiles(5,30)` ensures that the youngest five log files are kept as text files,
    /// the next 30 are kept as zip files, and older files are removed.
    ///
    /// This option is only available with feature `ziplogs`.
    #[cfg(feature = "ziplogs")]
    KeepLogAndZipFiles(usize, usize),
}
impl Cleanup {
    // Returns true if some cleanup is to be done.
    #[must_use]
    pub(crate) fn do_cleanup(&self) -> bool {
        match self {
            Self::Never => false,
            _ => true,
        }
    }
}

/// Used to control which messages are to be duplicated to stderr, when `log_to_file()` is used.
#[derive(Debug)]
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
