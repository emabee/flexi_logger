use crate::formats::AdaptiveFormat;
use crate::{
    filter::LogLineFilter,
    flexi_logger::FlexiLogger,
    formats::default_format,
    primary_writer::PrimaryWriter,
    threads::start_flusher_thread,
    util::{set_error_channel, set_panic_on_error_channel_error},
    writers::{FileLogWriter, FileLogWriterBuilder, LogWriter},
    Cleanup, Criterion, DeferredNow, FileSpec, FlexiLoggerError, FormatFunction, LogSpecification,
    LoggerHandle, Naming, WriteMode,
};

use log::LevelFilter;
#[cfg(feature = "specfile")]
use std::sync::Mutex;
use std::{
    collections::HashMap,
    io::IsTerminal,
    path::PathBuf,
    sync::{Arc, RwLock},
    time::Duration,
};
#[cfg(feature = "specfile_without_notification")]
use {crate::logger_handle::LogSpecSubscriber, std::io::Read, std::path::Path};
#[cfg(feature = "specfile")]
use {
    crate::util::{eprint_err, ErrorCode},
    notify_debouncer_mini::{
        new_debouncer,
        notify::{RecommendedWatcher, RecursiveMode},
        DebounceEventResult, Debouncer,
    },
};

/// The entry-point for using `flexi_logger`.
///
/// `Logger` is a builder class that allows you to
/// * specify your desired (initial) loglevel-specification
///   * either as a String ([`Logger::try_with_str`])
///   * or by providing it in the environment ([`Logger::try_with_env`]),
///   * or by combining both options ([`Logger::try_with_env_or_str`]),
///   * or by building a [`LogSpecification`] programmatically ([`Logger::with`]),
/// * use the desired configuration methods,
/// * and finally start the logger with
///
///   * [`Logger::start`], or
///   * [`Logger::start_with_specfile`].
///
/// # Usage
///
/// See [`code_examples`](code_examples/index.html) for a comprehensive list of usage possibilities.
pub struct Logger {
    spec: LogSpecification,
    log_target: LogTarget,
    duplicate_err: Duplicate,
    duplicate_out: Duplicate,
    format_for_file: FormatFunction,
    format_for_stderr: FormatFunction,
    format_for_stdout: FormatFunction,
    format_for_writer: FormatFunction,
    #[cfg(feature = "colors")]
    o_palette: Option<String>,
    flush_interval: std::time::Duration,
    flwb: FileLogWriterBuilder,
    other_writers: HashMap<String, Box<dyn LogWriter>>,
    filter: Option<Box<dyn LogLineFilter + Send + Sync>>,
    error_channel: ErrorChannel,
    use_utc: bool,
    panic_on_error_channel_error: bool,
}

enum LogTarget {
    StdErr,
    StdOut,
    Multi(bool, Option<Box<dyn LogWriter>>),
}

/// Create a Logger instance and define how to access the (initial)
/// loglevel-specification.
impl Logger {
    /// Creates a Logger that you provide with an explicit [`LogSpecification`].
    ///
    /// ## Examples
    ///
    /// ```rust
    /// use log::LevelFilter;
    /// use flexi_logger::Logger;
    /// let logger = Logger::with(LevelFilter::Info).start().unwrap();
    /// ```
    ///
    /// ```rust
    /// use flexi_logger::{Logger, LogSpecification};
    /// let logger = Logger::with(
    ///         LogSpecification::parse("info, critical_mod = trace").unwrap()
    ///     ).start().unwrap();
    /// ```
    #[must_use]
    pub fn with(logspec: impl Into<LogSpecification>) -> Self {
        Self::from_spec_and_errs(logspec.into())
    }

    /// Creates a Logger that reads the [`LogSpecification`] from a `String` or `&str`.
    /// See [`LogSpecification`] for the syntax.
    ///
    /// # Errors
    ///
    /// `FlexiLoggerError::Parse` if the String uses an erroneous syntax.
    pub fn try_with_str<S: AsRef<str>>(s: S) -> Result<Self, FlexiLoggerError> {
        Ok(Self::from_spec_and_errs(LogSpecification::parse(
            s.as_ref(),
        )?))
    }

    /// Creates a Logger that reads the [`LogSpecification`] from the environment variable
    /// `RUST_LOG`.
    ///
    /// Note that if `RUST_LOG` is not set, nothing is logged.
    ///
    /// # Errors
    ///
    /// `FlexiLoggerError::Parse` if the value of `RUST_LOG` is malformed.
    pub fn try_with_env() -> Result<Self, FlexiLoggerError> {
        Ok(Self::from_spec_and_errs(LogSpecification::env()?))
    }

    /// Creates a Logger that reads the [`LogSpecification`] from the environment variable
    /// `RUST_LOG`, or derives it from the given `String`, if `RUST_LOG` is not set.
    ///
    /// # Errors
    ///
    /// `FlexiLoggerError::Parse` if the chosen value is malformed.
    pub fn try_with_env_or_str<S: AsRef<str>>(s: S) -> Result<Self, FlexiLoggerError> {
        Ok(Self::from_spec_and_errs(LogSpecification::env_or_parse(s)?))
    }

    fn from_spec_and_errs(spec: LogSpecification) -> Self {
        #[cfg(feature = "colors")]
        #[cfg(windows)]
        {
            nu_ansi_term::enable_ansi_support().ok();
        }

        Self {
            spec,
            log_target: LogTarget::StdErr,
            duplicate_err: Duplicate::None,
            duplicate_out: Duplicate::None,
            format_for_file: default_format,

            #[cfg(feature = "colors")]
            format_for_stdout: AdaptiveFormat::Default
                .format_function(std::io::stdout().is_terminal()),
            #[cfg(feature = "colors")]
            format_for_stderr: AdaptiveFormat::Default
                .format_function(std::io::stderr().is_terminal()),

            #[cfg(not(feature = "colors"))]
            format_for_stdout: default_format,
            #[cfg(not(feature = "colors"))]
            format_for_stderr: default_format,

            format_for_writer: default_format,
            #[cfg(feature = "colors")]
            o_palette: None,
            flush_interval: Duration::from_secs(0),
            flwb: FileLogWriter::builder(FileSpec::default()),
            other_writers: HashMap::<String, Box<dyn LogWriter>>::new(),
            filter: None,
            error_channel: ErrorChannel::default(),
            use_utc: false,
            panic_on_error_channel_error: true,
        }
    }
}

/// Simple methods for influencing the behavior of the Logger.
impl Logger {
    /// Log is written to stderr (which is the default).
    #[must_use]
    pub fn log_to_stderr(mut self) -> Self {
        self.log_target = LogTarget::StdErr;
        self
    }

    /// Log is written to stdout.
    #[must_use]
    pub fn log_to_stdout(mut self) -> Self {
        self.log_target = LogTarget::StdOut;
        self
    }

    /// Log is written to a file.
    ///
    /// See [`FileSpec`] for details about the filename pattern.
    ///
    /// You can duplicate to stdout and stderr, and you can add additional writers.
    #[must_use]
    pub fn log_to_file(mut self, file_spec: FileSpec) -> Self {
        self.log_target = LogTarget::Multi(true, None);
        self.flwb = self.flwb.file_spec(file_spec);
        self
    }

    /// Log is written to the provided writer.
    ///
    /// You can duplicate to stdout and stderr, and you can add additional writers.
    #[must_use]
    pub fn log_to_writer(mut self, w: Box<dyn LogWriter>) -> Self {
        self.log_target = LogTarget::Multi(false, Some(w));
        self
    }

    /// Log is written to a file, as with [`Logger::log_to_file`], _and_ to an alternative
    /// [`LogWriter`] implementation.
    ///
    /// And you can duplicate to stdout and stderr, and you can add additional writers.
    #[must_use]
    pub fn log_to_file_and_writer(mut self, file_spec: FileSpec, w: Box<dyn LogWriter>) -> Self {
        self.log_target = LogTarget::Multi(true, Some(w));
        self.flwb = self.flwb.file_spec(file_spec);
        self
    }

    /// Log is processed, including duplication, but not written to any destination.
    ///
    /// This can be useful e.g. for running application tests with all log-levels active and still
    /// avoiding tons of log files etc.
    /// Such tests ensure that the log calls which are normally not active
    /// will not cause undesired side-effects when activated
    /// (note that the log macros may prevent arguments of inactive log-calls from being evaluated).
    ///
    /// Or, if you want to get logs both to stdout and stderr, but nowhere else,
    /// then use this option and combine it with
    /// [`Logger::duplicate_to_stdout`] and [`Logger::duplicate_to_stderr`].
    #[must_use]
    pub fn do_not_log(mut self) -> Self {
        self.log_target = LogTarget::Multi(false, None);
        self
    }

    /// Makes the logger print an info message to stdout with the name of the logfile
    /// when a logfile is opened for writing.
    #[must_use]
    pub fn print_message(mut self) -> Self {
        self.flwb = self.flwb.print_message();
        self
    }

    /// Makes the logger write messages with the specified minimum severity additionally to stderr.
    ///
    /// Does not work with [`Logger::log_to_stdout`] or [`Logger::log_to_stderr`].
    #[must_use]
    pub fn duplicate_to_stderr(mut self, dup: Duplicate) -> Self {
        self.duplicate_err = dup;
        self
    }

    /// Makes the logger write messages with the specified minimum severity additionally to stdout.
    ///
    /// Does not work with [`Logger::log_to_stdout`] or [`Logger::log_to_stderr`].
    #[must_use]
    pub fn duplicate_to_stdout(mut self, dup: Duplicate) -> Self {
        self.duplicate_out = dup;
        self
    }

    /// Makes the logger use the provided format function for all messages
    /// that are written to files, stderr, stdout, or to an additional writer.
    ///
    /// You can either choose one of the provided log-line formatters,
    /// or you create and use your own format function with the signature <br>
    /// ```rust
    /// fn my_format(
    ///    write: &mut dyn std::io::Write,
    ///    now: &mut flexi_logger::DeferredNow,
    ///    record: &log::Record,
    /// ) -> std::io::Result<()>
    /// # {unimplemented!("")}
    /// ```
    ///
    /// By default, [`default_format`] is used for output to files and to custom writers,
    /// and [`AdaptiveFormat::Default`] is used for output to `stderr` and `stdout`.
    /// If the feature `colors` is switched off, [`default_format`] is used for all outputs.
    #[must_use]
    pub fn format(mut self, format: FormatFunction) -> Self {
        self.format_for_file = format;
        self.format_for_stderr = format;
        self.format_for_stdout = format;
        self.format_for_writer = format;
        self
    }

    /// Makes the logger use the provided format function for messages
    /// that are written to files.
    ///
    /// Regarding the default, see [`Logger::format`].
    #[must_use]
    pub fn format_for_files(mut self, format: FormatFunction) -> Self {
        self.format_for_file = format;
        self
    }

    /// Makes the logger use the provided format function for messages
    /// that are written to stderr.
    ///
    /// Regarding the default, see [`Logger::format`].
    #[must_use]
    pub fn format_for_stderr(mut self, format_function: FormatFunction) -> Self {
        self.format_for_stderr = format_function;
        self
    }

    /// Makes the logger use the specified format for messages that are written to `stderr`.
    /// Coloring is used if `stderr` is a tty.
    ///
    /// Regarding the default, see [`Logger::format`].
    #[must_use]
    pub fn adaptive_format_for_stderr(mut self, adaptive_format: AdaptiveFormat) -> Self {
        self.format_for_stderr = adaptive_format.format_function(std::io::stderr().is_terminal());
        self
    }

    /// Makes the logger use the provided format function to format messages
    /// that are written to stdout.
    ///
    /// Regarding the default, see [`Logger::format`].
    #[must_use]
    pub fn format_for_stdout(mut self, format_function: FormatFunction) -> Self {
        self.format_for_stdout = format_function;
        self
    }

    /// Makes the logger use the specified format for messages that are written to `stdout`.
    /// Coloring is used if `stdout` is a tty.
    ///
    /// Regarding the default, see [`Logger::format`].
    #[must_use]
    pub fn adaptive_format_for_stdout(mut self, adaptive_format: AdaptiveFormat) -> Self {
        self.format_for_stdout = adaptive_format.format_function(std::io::stdout().is_terminal());
        self
    }

    /// Allows specifying a format function for an additional writer.
    /// Note that it is up to the implementation of the additional writer
    /// whether it evaluates this setting or not.
    ///
    /// Regarding the default, see [`Logger::format`].
    #[must_use]
    pub fn format_for_writer(mut self, format: FormatFunction) -> Self {
        self.format_for_writer = format;
        self
    }

    /// Sets the color palette for function [`style`](crate::style), which is used in the
    /// provided coloring format functions.
    ///
    /// The palette given here overrides the default palette.
    ///
    /// The palette is specified in form of a String that contains a semicolon-separated list
    /// of numbers (0..=255) and/or dashes (´-´).
    /// The first five values denote the fixed color that is
    /// used for coloring `error`, `warn`, `info`, `debug`, and `trace` messages.
    ///
    /// The String `"196;208;-;7;8"` describes the default palette, where color 196 is
    /// used for error messages, and so on. The `-` means that no coloring is done,
    /// i.e., with `"-;-;-;-;-"` all coloring is switched off.
    ///
    /// Prefixing a number with 'b' makes the output being written in bold.
    /// The String `"b1;3;2;4;6"` e.g. describes the palette used by `env_logger`.
    ///
    /// The palette can further be overridden at runtime by setting the environment variable
    /// `FLEXI_LOGGER_PALETTE` to a palette String. This allows adapting the used text colors to
    /// differently colored terminal backgrounds.
    ///
    /// For your convenience, if you want to specify your own palette,
    /// you can produce a colored list with all 255 colors with `cargo run --example colors`.
    #[cfg_attr(docsrs, doc(cfg(feature = "colors")))]
    #[cfg(feature = "colors")]
    #[must_use]
    pub fn set_palette(mut self, palette: String) -> Self {
        self.o_palette = Some(palette);
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
    /// - by default, no timestamp is added to the filename if rotation is used
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
    /// `criterion` defines *when* the log file should be rotated, based on its size or age.
    /// See [`Criterion`] for details.
    ///
    /// `naming` defines the naming convention for the rotated log files.
    /// See [`Naming`] for details.
    ///
    /// `cleanup` defines the strategy for dealing with older files.
    /// See [`Cleanup`] for details.
    #[must_use]
    pub fn rotate(mut self, criterion: Criterion, naming: Naming, cleanup: Cleanup) -> Self {
        self.flwb = self.flwb.rotate(criterion, naming, cleanup);
        self
    }

    /// When [`Logger::rotate`] is used with some [`Cleanup`] variant other than [`Cleanup::Never`],
    /// then this method can be used to define
    /// if the cleanup activities (finding files, deleting files, evtl compressing files) are
    /// delegated to a background thread (which is the default,
    /// to minimize the blocking impact to your application caused by IO operations),
    /// or whether they are done synchronously in the current log-call.
    ///
    /// If you call this method with `use_background_thread = false`,
    /// the cleanup is done synchronously.
    #[must_use]
    pub fn cleanup_in_background_thread(mut self, use_background_thread: bool) -> Self {
        self.flwb = self
            .flwb
            .cleanup_in_background_thread(use_background_thread);
        self
    }

    /// Apply the provided filter before really writing log lines.
    ///
    /// See the documentation of module [`filter`](crate::filter) for a usage example.
    #[must_use]
    pub fn filter(mut self, filter: Box<dyn LogLineFilter + Send + Sync>) -> Self {
        self.filter = Some(filter);
        self
    }

    /// Makes the logger append to the specified output file, if it exists already;
    /// by default, the file would be truncated.
    ///
    /// This option only has an effect if logs are written to files, but
    /// it will hardly make an effect if [`FileSpec::suppress_timestamp`] is not used.
    #[must_use]
    pub fn append(mut self) -> Self {
        self.flwb = self.flwb.append();
        self
    }

    /// Makes the logger use UTC timestamps rather than local timestamps.
    #[must_use]
    pub fn use_utc(mut self) -> Self {
        self.use_utc = true;
        self
    }

    /// The specified path will be used on unix systems to create a symbolic link
    /// to the current log file.
    ///
    /// This option has no effect on filesystems where symlinks are not supported,
    /// and it only has an effect if logs are written to files.
    ///
    /// ### Example
    ///
    /// You can use the symbolic link to follow the log output with `tail`,
    /// even if the log files are rotated.
    ///
    /// Assuming you use `create_symlink("link_to_log_file")`, then use:
    ///
    /// ```text
    /// tail --follow=name --max-unchanged-stats=1 --retry link_to_log_file
    /// ```
    ///
    #[must_use]
    pub fn create_symlink<P: Into<PathBuf>>(mut self, symlink: P) -> Self {
        self.flwb = self.flwb.create_symlink(symlink);
        self
    }

    /// Registers a [`LogWriter`] implementation under the given target name.
    ///
    /// The target name must not start with an underscore.
    /// See module [`writers`](crate::writers) for more details.
    #[must_use]
    pub fn add_writer<S: Into<String>>(
        mut self,
        target_name: S,
        writer: Box<dyn LogWriter>,
    ) -> Self {
        self.other_writers.insert(target_name.into(), writer);
        self
    }

    /// Sets the write mode for the logger.
    ///
    /// See [`WriteMode`] for more (important!) details.
    #[must_use]
    pub fn write_mode(mut self, write_mode: WriteMode) -> Self {
        self.flwb = self.flwb.write_mode(write_mode.without_flushing());
        self.flush_interval = write_mode.get_flush_interval();
        self
    }

    /// Use Windows line endings, rather than just `\n`.
    #[must_use]
    pub fn use_windows_line_ending(mut self) -> Self {
        self.flwb = self.flwb.use_windows_line_ending();
        self
    }

    /// Define the output channel for `flexi_logger`'s own error messages.
    ///
    /// These are only written if `flexi_logger` cannot do what it is supposed to do.
    /// Under normal circumstances no single message should appear.
    ///
    /// By default these error messages are printed to `stderr`.
    #[must_use]
    pub fn error_channel(mut self, error_channel: ErrorChannel) -> Self {
        self.error_channel = error_channel;
        self
    }

    /// Decides what `flexi_logger` should do if the error output channel cannot be written to.
    ///
    /// By default, it will panic if error messages cannot be written to the chosen
    /// error output channel.
    /// Calling this method with `false` will let `flexi_logger` ignore the issue and suppress
    /// the error messages.
    #[must_use]
    pub fn panic_if_error_channel_is_broken(mut self, panic: bool) -> Self {
        self.panic_on_error_channel_error = panic;
        self
    }
}

/// Enum for defining the output channel for `flexi_logger`'s own error messages.
///
/// These are only written if `flexi_logger` cannot do what it is supposed to do,
/// so under normal circumstances no single message shuld appear.
///
/// By default these error messages are printed to `stderr`.
#[derive(Debug, Default)]
pub enum ErrorChannel {
    /// Write `flexi_logger`'s own error messages to `stderr`.
    #[default]
    StdErr,
    /// Write `flexi_logger`'s own error messages to `stdout`.
    StdOut,
    /// Write `flexi_logger`'s own error messages to the specified file.
    File(PathBuf),
    /// Don't write `flexi_logger`'s own error messages.
    DevNull,
}

/// Alternative set of methods to control the behavior of the Logger.
/// Use these methods when you want to control the settings flexibly,
/// e.g. with commandline arguments via `docopts` or `clap`.
impl Logger {
    /// With true, makes the logger print an info message to stdout, each time
    /// when a new file is used for log-output.
    #[must_use]
    pub fn o_print_message(mut self, print_message: bool) -> Self {
        self.flwb = self.flwb.o_print_message(print_message);
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
    #[must_use]
    pub fn o_rotate(mut self, rotate_config: Option<(Criterion, Naming, Cleanup)>) -> Self {
        self.flwb = self.flwb.o_rotate(rotate_config);
        self
    }

    /// If append is set to true, makes the logger append to the specified output file, if it exists.
    /// By default, or with false, the file would be truncated.
    ///
    /// This option only has an effect if logs are written to files,
    /// and it will hardly make an effect if `suppress_timestamp()` is not used.
    #[must_use]
    pub fn o_append(mut self, append: bool) -> Self {
        self.flwb = self.flwb.o_append(append);
        self
    }

    /// If a String is specified, it will be used on unix systems to create in the current folder
    /// a symbolic link with this name to the current log file.
    ///
    /// This option only has an effect on unix systems and if logs are written to files.
    #[must_use]
    pub fn o_create_symlink<P: Into<PathBuf>>(mut self, symlink: Option<P>) -> Self {
        self.flwb = self.flwb.o_create_symlink(symlink);
        self
    }
}

/// Finally, start logging, optionally with a spec-file.
impl Logger {
    /// Consumes the Logger object and initializes `flexi_logger`.
    ///
    /// **Keep the [`LoggerHandle`] alive up to the very end of your program!**
    /// Dropping the [`LoggerHandle`] flushes and shuts down [`FileLogWriter`]s
    /// and other [`LogWriter`]s, and then may prevent further logging!
    /// This should happen immediately before the program terminates, but not earlier.
    ///
    /// Dropping the [`LoggerHandle`] is uncritical
    /// only with [`Logger::log_to_stdout`] or [`Logger::log_to_stderr`].
    ///
    /// The [`LoggerHandle`] also allows updating the log specification programmatically,
    /// e.g. to intensify logging for (buggy) parts of a (test) program, etc.
    ///
    /// # Example
    ///
    /// ```rust
    /// use flexi_logger::{Logger,WriteMode, FileSpec};
    /// fn main() -> Result<(), Box<dyn std::error::Error>> {
    ///     let _logger = Logger::try_with_str("info")?
    ///         .log_to_file(FileSpec::default())
    ///         .write_mode(WriteMode::BufferAndFlush)
    ///         .start()?;
    ///
    ///     // ... do all your work and join back all threads whose logs you want to see ...
    ///
    ///     Ok(())
    /// }
    /// ```
    ///
    /// # Errors
    ///
    /// Several variants of [`FlexiLoggerError`] can occur.
    pub fn start(self) -> Result<LoggerHandle, FlexiLoggerError> {
        let (boxed_logger, handle) = self.build()?;
        log::set_boxed_logger(boxed_logger)?;
        Ok(handle)
    }

    /// Builds a boxed logger and a `LoggerHandle` for it,
    /// but does not initialize the global logger.
    ///
    /// The returned boxed logger implements the [`Log`](log::Log) trait
    /// and can be installed manually or nested within another logger.
    ///
    /// **Keep the [`LoggerHandle`] alive up to the very end of your program!**
    /// See [`Logger::start`] for more details.
    ///
    /// # Errors
    ///
    /// Several variants of [`FlexiLoggerError`] can occur.
    pub fn build(mut self) -> Result<(Box<dyn log::Log>, LoggerHandle), FlexiLoggerError> {
        #[cfg(feature = "colors")]
        crate::formats::set_palette(&self.o_palette)?;

        if self.use_utc {
            self.flwb = self.flwb.use_utc();
        }
        set_panic_on_error_channel_error(self.panic_on_error_channel_error);

        let a_primary_writer = Arc::new(match self.log_target {
            LogTarget::StdOut => {
                if let WriteMode::SupportCapture = self.flwb.get_write_mode() {
                    PrimaryWriter::test(true, self.format_for_stdout)
                } else {
                    PrimaryWriter::stdout(self.format_for_stdout, self.flwb.get_write_mode())
                }
            }
            LogTarget::StdErr => {
                if let WriteMode::SupportCapture = self.flwb.get_write_mode() {
                    PrimaryWriter::test(false, self.format_for_stderr)
                } else {
                    PrimaryWriter::stderr(self.format_for_stderr, self.flwb.get_write_mode())
                }
            }
            LogTarget::Multi(use_file, mut o_writer) => PrimaryWriter::multi(
                self.duplicate_err,
                self.duplicate_out,
                WriteMode::SupportCapture == *self.flwb.get_write_mode(),
                self.format_for_stderr,
                self.format_for_stdout,
                if use_file {
                    Some(Box::new(
                        self.flwb.format(self.format_for_file).try_build()?,
                    ))
                } else {
                    None
                },
                {
                    if let Some(ref mut writer) = o_writer {
                        writer.format(self.format_for_writer);
                    }
                    o_writer
                },
            ),
        });

        let a_other_writers = Arc::new(self.other_writers);

        if self.flush_interval.as_secs() != 0 || self.flush_interval.subsec_nanos() != 0 {
            start_flusher_thread(
                Arc::clone(&a_primary_writer),
                Arc::clone(&a_other_writers),
                self.flush_interval,
            )?;
        }

        let max_level = self.spec.max_level();
        let a_l_spec = Arc::new(RwLock::new(self.spec));
        set_error_channel(self.error_channel);

        // initialize the lazy_statics in DeferredNow before threads are spawned
        if self.use_utc {
            DeferredNow::force_utc();
        }
        let mut now = DeferredNow::new();
        now.now();

        let flexi_logger = FlexiLogger::new(
            Arc::clone(&a_l_spec),
            Arc::clone(&a_primary_writer),
            Arc::clone(&a_other_writers),
            self.filter,
        );

        let handle = LoggerHandle::new(a_l_spec, a_primary_writer, a_other_writers);
        handle.reconfigure(max_level);
        Ok((Box::new(flexi_logger), handle))
    }

    /// Consumes the Logger object and initializes `flexi_logger` in a way that
    /// subsequently the log specification can be updated,
    /// while the program is running, by editing a file.
    ///
    /// Uses the spec that was given to the factory method ([`Logger::with`] etc)
    /// as initial spec and then tries to read the logspec from a file.
    ///
    /// If the file does not exist, `flexi_logger` creates the file and fills it
    /// with the initial spec (and in the respective file format, of course).
    ///
    /// **Keep the returned [`LoggerHandle`] alive up to the very end of your program!**
    /// See [`Logger::start`] for more details.
    ///
    /// # Feature dependency
    ///
    /// The implementation of this configuration method uses some additional crates
    /// that you might not want to depend on with your program if you don't use this functionality.
    /// For that reason the method is only available if you activate the
    /// `specfile` feature. See the usage section on
    /// [crates.io](https://crates.io/crates/flexi_logger) for details.
    ///
    /// # Usage
    ///
    /// A logger initialization like
    ///
    /// ```rust,no_run
    /// use flexi_logger::Logger;
    /// Logger::try_with_str("info")
    ///     .unwrap()
    ///     // more logger configuration
    ///     .start_with_specfile("logspecification.toml");
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
    /// Several variants of [`FlexiLoggerError`] can occur.
    #[cfg_attr(docsrs, doc(cfg(feature = "specfile")))]
    #[cfg(feature = "specfile_without_notification")]
    pub fn start_with_specfile<P: AsRef<Path>>(
        self,
        specfile: P,
    ) -> Result<LoggerHandle, FlexiLoggerError> {
        let (boxed_logger, handle) = self.build_with_specfile(specfile)?;
        log::set_boxed_logger(boxed_logger)?;
        Ok(handle)
    }

    /// Builds a boxed logger and a `LoggerHandle` for it,
    /// but does not initialize the global logger.
    ///
    /// See also [`Logger::start`] and [`Logger::start_with_specfile`].
    /// for the properties of the returned logger.
    ///
    /// # Errors
    ///
    /// Several variants of [`FlexiLoggerError`] can occur.
    #[cfg_attr(docsrs, doc(cfg(feature = "specfile")))]
    #[cfg(feature = "specfile_without_notification")]
    pub fn build_with_specfile<P: AsRef<Path>>(
        self,
        specfile: P,
    ) -> Result<(Box<dyn log::Log>, LoggerHandle), FlexiLoggerError> {
        let (boxed_log, mut handle) = self.build()?;

        let specfile = specfile.as_ref();
        synchronize_subscriber_with_specfile(&mut handle.writers_handle, specfile)?;

        #[cfg(feature = "specfile")]
        {
            handle.oam_specfile_watcher = Some(Arc::new(Mutex::new(create_specfile_watcher(
                specfile,
                handle.writers_handle.clone(),
            )?)));
        }

        Ok((boxed_log, handle))
    }
}

// Reread the specfile when it was updated
#[cfg(feature = "specfile")]
pub(crate) fn create_specfile_watcher<S: LogSpecSubscriber>(
    specfile: &Path,
    mut subscriber: S,
) -> Result<Debouncer<RecommendedWatcher>, FlexiLoggerError> {
    let specfile = specfile
        .canonicalize()
        .map_err(FlexiLoggerError::SpecfileIo)?;
    let clone = specfile.clone();
    let parent = clone.parent().unwrap(/*cannot fail*/);

    let mut debouncer = new_debouncer(
        std::time::Duration::from_millis(1000),
        move |res: DebounceEventResult| match res {
            Ok(events) => events.iter().for_each(|e| {
                if e.path
                    .canonicalize()
                    .map(|x| x == specfile)
                    .unwrap_or(false)
                {
                    log_spec_string_from_file(&specfile)
                        .map_err(FlexiLoggerError::SpecfileIo)
                        .and_then(LogSpecification::from_toml)
                        .and_then(|spec| subscriber.set_new_spec(spec))
                        .map_err(|e| {
                            eprint_err(
                                ErrorCode::LogSpecFile,
                                "continuing with previous log specification, because \
                                            rereading the log specification file failed",
                                &e,
                            );
                        })
                        .ok();
                }
            }),
            Err(e) => eprint_err(
                ErrorCode::LogSpecFile,
                "error while watching the specfile",
                &e,
            ),
        },
    )
    .unwrap();

    debouncer
        .watcher()
        .watch(parent, RecursiveMode::NonRecursive)
        .unwrap();

    Ok(debouncer)
}

// If the specfile exists, read the file and update the subscriber's logspec from it;
// otherwise try to create the file, with the current spec as content, under the specified name.
#[cfg(feature = "specfile_without_notification")]
pub(crate) fn synchronize_subscriber_with_specfile<S: LogSpecSubscriber>(
    subscriber: &mut S,
    specfile: &Path,
) -> Result<(), FlexiLoggerError> {
    if specfile
        .extension()
        .unwrap_or_else(|| std::ffi::OsStr::new(""))
        .to_str()
        .unwrap_or("")
        != "toml"
    {
        return Err(FlexiLoggerError::SpecfileExtension(
            "only spec files with extension toml are supported",
        ));
    }

    if Path::is_file(specfile) {
        let s = log_spec_string_from_file(specfile).map_err(FlexiLoggerError::SpecfileIo)?;
        subscriber.set_new_spec(LogSpecification::from_toml(s)?)?;
    } else {
        if let Some(specfolder) = specfile.parent() {
            std::fs::DirBuilder::new()
                .recursive(true)
                .create(specfolder)
                .map_err(FlexiLoggerError::SpecfileIo)?;
        }
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(specfile)
            .map_err(FlexiLoggerError::SpecfileIo)?;

        subscriber.initial_spec()?.to_toml(&mut file)?;
    }
    Ok(())
}

#[cfg(feature = "specfile_without_notification")]
pub(crate) fn log_spec_string_from_file<P: AsRef<Path>>(
    specfile: P,
) -> Result<String, std::io::Error> {
    let mut buf = String::new();
    let mut file = std::fs::File::open(specfile)?;
    file.read_to_string(&mut buf)?;
    Ok(buf)
}

/// Used to control which messages are to be duplicated to stderr, when `log_to_file()` is used.
#[derive(Debug, Clone, Copy)]
pub enum Duplicate {
    /// No messages are duplicated.
    None = 0,
    /// Only error messages are duplicated.
    Error = 1,
    /// Error and warn messages are duplicated.
    Warn = 2,
    /// Error, warn, and info messages are duplicated.
    Info = 3,
    /// Error, warn, info, and debug messages are duplicated.
    Debug = 4,
    /// All messages are duplicated.
    Trace = 5,
    /// All messages are duplicated.
    All = 6,
}
impl From<u8> for Duplicate {
    fn from(val: u8) -> Self {
        match val {
            0 => Duplicate::None,
            1 => Duplicate::Error,
            2 => Duplicate::Warn,
            3 => Duplicate::Info,
            4 => Duplicate::Debug,
            5 => Duplicate::Trace,
            6 => Duplicate::All,
            _ => unreachable!(),
        }
    }
}

impl From<LevelFilter> for Duplicate {
    fn from(level: LevelFilter) -> Self {
        match level {
            LevelFilter::Off => Duplicate::None,
            LevelFilter::Error => Duplicate::Error,
            LevelFilter::Warn => Duplicate::Warn,
            LevelFilter::Info => Duplicate::Info,
            LevelFilter::Debug => Duplicate::Debug,
            LevelFilter::Trace => Duplicate::Trace,
        }
    }
}
impl From<Duplicate> for LevelFilter {
    fn from(level: Duplicate) -> Self {
        match level {
            Duplicate::None => LevelFilter::Off,
            Duplicate::Error => LevelFilter::Error,
            Duplicate::Warn => LevelFilter::Warn,
            Duplicate::Info => LevelFilter::Info,
            Duplicate::Debug => LevelFilter::Debug,
            Duplicate::Trace | Duplicate::All => LevelFilter::Trace,
        }
    }
}
