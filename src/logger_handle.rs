use crate::{
    primary_writer::PrimaryWriter,
    util::{eprint_err, ErrorCode},
    writers::{FileLogWriterBuilder, FileLogWriterConfig, LogWriter},
    Duplicate, FlexiLoggerError, LogSpecification,
};
#[cfg(feature = "specfile")]
use notify_debouncer_mini::{notify::RecommendedWatcher, Debouncer};
#[cfg(feature = "specfile")]
use std::sync::Mutex;
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, RwLock},
};

/// Allows reconfiguring the logger while the program is running, and
/// **shuts down the logger when it is dropped**.
///
/// A `LoggerHandle` is returned from `Logger::start()` and from `Logger::start_with_specfile()`.
///
/// Keep it alive until the very end of your program, because it shuts down the logger when
/// its dropped!
/// (This is only relevant if you use one of
/// `Logger::log_to_file`, `Logger::log_to_writer`, or `Logger::log_to_file_and_writer`, or
/// a buffering or asynchronous [`WriteMode`](crate::WriteMode)).
///
/// `LoggerHandle` offers methods to modify the log specification programmatically,
/// to flush the logger explicitly, and to reconfigure the used `FileLogWriter` --
/// if one is used.
///
/// # Examples
///
/// In more trivial configurations, dropping the `LoggerHandle` has no effect and then
/// you can safely ignore the return value of `Logger::start()`:
///
/// ```rust
/// use flexi_logger::Logger;
/// use std::error::Error;
/// fn main() -> Result<(), Box<dyn Error>> {
///     Logger::try_with_str("info")?.start()?;
///     // do work
///     Ok(())
/// }
/// ```
///
/// When logging to a file or another writer,
/// and/or if you use a buffering or asynchronous [`WriteMode`](crate::WriteMode),
/// keep the `LoggerHandle` alive until the program ends:
///
/// ```rust
/// use flexi_logger::{FileSpec, Logger};
/// use std::error::Error;
/// fn main() -> Result<(), Box<dyn Error>> {
///     let _logger = Logger::try_with_str("info")?
///         .log_to_file(FileSpec::default())
///         .start()?;
///     // do work
///     Ok(())
/// }
/// ```
///
/// You can use the logger handle to permanently exchange the log specification programmatically,
/// anywhere in your code:
///
/// ```rust
/// # use flexi_logger::Logger;
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
///     let logger = Logger::try_with_str("info")?.start()?;
///     // ...
///     logger.parse_new_spec("warn");
///     // ...
///     # Ok(())
/// # }
/// ```
///
/// However, when debugging, you often want to modify the log spec only temporarily, for  
/// one or few method calls only; this is easier done with the following method, because
/// it allows switching back to the previous spec:
///
/// ```rust
/// # use flexi_logger::Logger;
/// # use std::error::Error;
/// # fn main() -> Result<(), Box<dyn Error>> {
///     let mut logger = Logger::try_with_str("info")?.start()?;
///     logger.parse_and_push_temp_spec("trace");
///     // ...
///     // critical calls
///     // ...
///     logger.pop_temp_spec();
///     // Continue with the log spec you had before.
///     // ...
/// # Ok(())
/// # }
/// ```
#[derive(Clone)]
pub struct LoggerHandle
where
    Self: Send + Sync,
    // Note: we demand Send and Sync explicitly because we want to be able to move a
    // `LoggerHandle` between threads.
    // At least with notify_debouncer_mini version 0.4.1 this would not be given if we omitted
    // the Mutex (which we don't need otherwise): we'd then get
    //     `std::sync::mpsc::Sender<notify_debouncer_mini::InnerEvent>` cannot be shared \
    //     between threads safely
{
    pub(crate) writers_handle: WritersHandle,
    #[cfg(feature = "specfile")]
    pub(crate) oam_specfile_watcher: Option<Arc<Mutex<Debouncer<RecommendedWatcher>>>>,
}
impl LoggerHandle {
    pub(crate) fn new(
        spec: Arc<RwLock<LogSpecification>>,
        primary_writer: Arc<PrimaryWriter>,
        other_writers: Arc<HashMap<String, Box<dyn LogWriter>>>,
    ) -> Self {
        Self {
            writers_handle: WritersHandle {
                spec,
                spec_stack: Vec::default(),
                primary_writer,
                other_writers,
            },
            #[cfg(feature = "specfile")]
            oam_specfile_watcher: None,
        }
    }

    //
    pub(crate) fn reconfigure(&self, max_level: log::LevelFilter) {
        self.writers_handle.reconfigure(max_level);
    }

    /// Replaces the active `LogSpecification`.
    #[allow(clippy::missing_panics_doc)]
    pub fn set_new_spec(&self, new_spec: LogSpecification) {
        self.writers_handle
            .set_new_spec(new_spec)
            .map_err(|e| eprint_err(ErrorCode::Poison, "rwlock on log spec is poisoned", &e))
            .ok();
    }

    /// Tries to replace the active `LogSpecification` with the result from parsing the given String.
    ///
    /// # Errors
    ///
    /// [`FlexiLoggerError::Parse`] if the input is malformed.
    pub fn parse_new_spec(&self, spec: &str) -> Result<(), FlexiLoggerError> {
        self.set_new_spec(LogSpecification::parse(spec)?);
        Ok(())
    }

    /// Replaces the active `LogSpecification` and pushes the previous one to a Stack.
    #[allow(clippy::missing_panics_doc)]
    pub fn push_temp_spec(&mut self, new_spec: LogSpecification) {
        self.writers_handle
            .spec_stack
            .push(self.writers_handle.spec.read().unwrap(/* catch and expose error? */).clone());
        self.set_new_spec(new_spec);
    }

    /// Tries to replace the active `LogSpecification` with the result from parsing the given String
    ///  and pushes the previous one to a Stack.
    ///
    /// # Errors
    ///
    /// [`FlexiLoggerError::Parse`] if the input is malformed.
    pub fn parse_and_push_temp_spec<S: AsRef<str>>(
        &mut self,
        new_spec: S,
    ) -> Result<(), FlexiLoggerError> {
        self.writers_handle.spec_stack.push(
            self.writers_handle
                .spec
                .read()
                .map_err(|_| FlexiLoggerError::Poison)?
                .clone(),
        );
        self.set_new_spec(LogSpecification::parse(new_spec)?);
        Ok(())
    }

    /// Reverts to the previous `LogSpecification`, if any.
    pub fn pop_temp_spec(&mut self) {
        if let Some(previous_spec) = self.writers_handle.spec_stack.pop() {
            self.set_new_spec(previous_spec);
        }
    }

    /// Flush all writers.
    pub fn flush(&self) {
        self.writers_handle.primary_writer.flush().ok();
        for writer in self.writers_handle.other_writers.values() {
            writer.flush().ok();
        }
    }

    /// Replaces parts of the configuration of the file log writer.
    ///
    /// Note that neither the write mode nor the format function can be reset and
    /// that the provided `FileLogWriterBuilder` must have the same values for these as the
    /// currently used `FileLogWriter`.
    ///
    /// # Example
    ///
    /// See [`code_examples`](code_examples/index.html#reconfigure-the-file-log-writer).
    ///
    /// # Errors
    ///
    /// `FlexiLoggerError::NoFileLogger` if no file log writer is configured.
    ///
    /// `FlexiLoggerError::Reset` if a reset was tried with a different write mode.
    ///
    /// `FlexiLoggerError::Io` if the specified path doesn't work.
    ///
    /// `FlexiLoggerError::Poison` if some mutex is poisoned.
    pub fn reset_flw(&self, flwb: &FileLogWriterBuilder) -> Result<(), FlexiLoggerError> {
        if let PrimaryWriter::Multi(ref mw) = &*self.writers_handle.primary_writer {
            mw.reset_file_log_writer(flwb)
        } else {
            Err(FlexiLoggerError::NoFileLogger)
        }
    }

    /// Returns the current configuration of the file log writer.
    ///
    /// # Errors
    ///
    /// `FlexiLoggerError::NoFileLogger` if no file log writer is configured.
    ///
    /// `FlexiLoggerError::Poison` if some mutex is poisoned.
    pub fn flw_config(&self) -> Result<FileLogWriterConfig, FlexiLoggerError> {
        if let PrimaryWriter::Multi(ref mw) = &*self.writers_handle.primary_writer {
            mw.flw_config()
        } else {
            Err(FlexiLoggerError::NoFileLogger)
        }
    }

    /// Makes the logger re-open the current log file.
    ///
    /// If the log is written to a file, `flexi_logger` expects that nobody else modifies the file,
    /// and offers capabilities to rotate, compress, and clean up log files.
    ///
    /// However, if you use tools like linux' `logrotate`
    /// to rename or delete the current output file, you need to inform `flexi_logger` about
    /// such actions by calling this method. Otherwise `flexi_logger` will not stop
    /// writing to the renamed or even deleted file!
    ///
    /// In more complex configurations, i.e. when more than one output stream is written to,
    /// all of them will be attempted to be re-opened; only the first error will be reported.
    ///
    /// # Example
    ///
    /// `logrotate` e.g. can be configured to send a `SIGHUP` signal to your program. You need to
    /// handle `SIGHUP` in your program explicitly,
    /// e.g. using a crate like [`ctrlc`](https://docs.rs/ctrlc/latest/ctrlc/),
    /// and call this function from the registered signal handler.
    ///
    /// # Errors
    ///
    /// `FlexiLoggerError::Poison` if some mutex is poisoned.
    ///
    /// Other variants of `FlexiLoggerError`, depending on the used writers.
    pub fn reopen_output(&self) -> Result<(), FlexiLoggerError> {
        let mut result = if let PrimaryWriter::Multi(ref mw) = &*self.writers_handle.primary_writer
        {
            mw.reopen_output()
        } else {
            Ok(())
        };

        for blw in self.writers_handle.other_writers.values() {
            let result2 = blw.reopen_output();
            if result.is_ok() && result2.is_err() {
                result = result2;
            }
        }

        result
    }

    /// Trigger an extra log file rotation.
    ///
    /// Does nothing if rotation is not configured.
    ///
    /// # Errors
    ///
    /// `FlexiLoggerError::Poison` if some mutex is poisoned.
    ///
    /// IO errors.
    pub fn trigger_rotation(&self) -> Result<(), FlexiLoggerError> {
        let mut result = if let PrimaryWriter::Multi(ref mw) = &*self.writers_handle.primary_writer
        {
            mw.trigger_rotation()
        } else {
            Ok(())
        };

        for blw in self.writers_handle.other_writers.values() {
            let result2 = blw.rotate();
            if result.is_ok() && result2.is_err() {
                result = result2;
            }
        }
        result
    }

    /// Shutdown all participating writers.
    ///
    /// This method is supposed to be called at the very end of your program, if
    ///
    /// - you use some [`Cleanup`](crate::Cleanup) strategy with compression:
    ///   then you want to ensure that a termination of your program
    ///   does not interrput the cleanup-thread when it is compressing a log file,
    ///   which could leave unexpected files in the filesystem
    /// - you use your own writer(s), and they need to clean up resources
    ///
    /// See also [`writers::LogWriter::shutdown`](crate::writers::LogWriter::shutdown).
    pub fn shutdown(&self) {
        self.writers_handle.primary_writer.shutdown();
        for writer in self.writers_handle.other_writers.values() {
            writer.shutdown();
        }
    }

    /// Returns the list of existing log files according to the current `FileSpec`.
    ///
    /// Depending on the given selector, the list may include the CURRENT log file
    /// and the compressed files, if they exist.
    /// The list is empty if the logger is not configured for writing to files.
    ///
    /// # Errors
    ///
    /// `FlexiLoggerError::Poison` if some mutex is poisoned.
    pub fn existing_log_files(
        &self,
        selector: &LogfileSelector,
    ) -> Result<Vec<PathBuf>, FlexiLoggerError> {
        let mut log_files = self
            .writers_handle
            .primary_writer
            .existing_log_files(selector)?;
        log_files.sort();
        Ok(log_files)
    }

    /// Allows re-configuring duplication to stderr.
    ///
    ///  # Errors
    ///  
    ///  `FlexiLoggerError::NoDuplication`
    ///   if `FlexiLogger` was initialized without duplication support
    pub fn adapt_duplication_to_stderr(&mut self, dup: Duplicate) -> Result<(), FlexiLoggerError> {
        if let PrimaryWriter::Multi(ref mw) = &*self.writers_handle.primary_writer {
            mw.adapt_duplication_to_stderr(dup);
            Ok(())
        } else {
            Err(FlexiLoggerError::NoFileLogger)
        }
    }

    /// Allows re-configuring duplication to stdout.
    ///
    ///  # Errors
    ///  
    ///  `FlexiLoggerError::NoDuplication`
    ///   if `FlexiLogger` was initialized without duplication support
    pub fn adapt_duplication_to_stdout(&mut self, dup: Duplicate) -> Result<(), FlexiLoggerError> {
        if let PrimaryWriter::Multi(ref mw) = &*self.writers_handle.primary_writer {
            mw.adapt_duplication_to_stdout(dup);
            Ok(())
        } else {
            Err(FlexiLoggerError::NoFileLogger)
        }
    }

    // Allows checking the logs written so far to the writer
    #[doc(hidden)]
    pub fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) {
        self.writers_handle.primary_writer.validate_logs(expected);
    }
}

/// Used in [`LoggerHandle::existing_log_files`].
///
/// Example:
///
/// ```rust
/// # use flexi_logger::{LogfileSelector,Logger};
/// # let logger_handle = Logger::try_with_env().unwrap().start().unwrap();
/// let all_log_files = logger_handle.existing_log_files(
///     &LogfileSelector::default()
///         .with_r_current()
///         .with_compressed_files()
/// );
/// ```
pub struct LogfileSelector {
    pub(crate) with_plain_files: bool,
    pub(crate) with_r_current: bool,
    pub(crate) with_compressed_files: bool,
}
impl Default for LogfileSelector {
    /// Selects plain log files without the `rCURRENT` file.
    fn default() -> Self {
        Self {
            with_plain_files: true,
            with_r_current: false,
            with_compressed_files: false,
        }
    }
}
impl LogfileSelector {
    /// Selects no file at all.
    #[must_use]
    pub fn none() -> Self {
        Self {
            with_plain_files: false,
            with_r_current: false,
            with_compressed_files: false,
        }
    }
    /// Selects additionally the `rCURRENT` file.
    #[must_use]
    pub fn with_r_current(mut self) -> Self {
        self.with_r_current = true;
        self
    }

    /// Selects additionally the compressed log files.
    #[must_use]
    pub fn with_compressed_files(mut self) -> Self {
        self.with_compressed_files = true;
        self
    }
}

#[derive(Clone)]
pub(crate) struct WritersHandle {
    spec: Arc<RwLock<LogSpecification>>,
    spec_stack: Vec<LogSpecification>,
    primary_writer: Arc<PrimaryWriter>,
    other_writers: Arc<HashMap<String, Box<dyn LogWriter>>>,
}
impl WritersHandle {
    fn set_new_spec(&self, new_spec: LogSpecification) -> Result<(), FlexiLoggerError> {
        let max_level = new_spec.max_level();
        self.spec
            .write()
            .map_err(|_| FlexiLoggerError::Poison)?
            .update_from(new_spec);
        self.reconfigure(max_level);
        Ok(())
    }

    pub(crate) fn reconfigure(&self, mut max_level: log::LevelFilter) {
        for w in self.other_writers.as_ref().values() {
            max_level = std::cmp::max(max_level, w.max_log_level());
        }
        log::set_max_level(max_level);
    }
}
impl Drop for WritersHandle {
    fn drop(&mut self) {
        self.primary_writer.shutdown();
        for writer in self.other_writers.values() {
            writer.shutdown();
        }
    }
}

/// Trait that allows to register for changes to the log specification.
#[cfg(feature = "specfile_without_notification")]
#[cfg_attr(docsrs, doc(cfg(feature = "specfile")))]
pub trait LogSpecSubscriber: 'static + Send {
    /// Apply a new `LogSpecification`.
    ///
    /// # Errors
    fn set_new_spec(&mut self, new_spec: LogSpecification) -> Result<(), FlexiLoggerError>;

    /// Provide the current log spec.
    ///
    /// # Errors
    fn initial_spec(&self) -> Result<LogSpecification, FlexiLoggerError>;
}
#[cfg(feature = "specfile_without_notification")]
impl LogSpecSubscriber for WritersHandle {
    fn set_new_spec(&mut self, new_spec: LogSpecification) -> Result<(), FlexiLoggerError> {
        WritersHandle::set_new_spec(self, new_spec)
    }

    fn initial_spec(&self) -> Result<LogSpecification, FlexiLoggerError> {
        Ok((*self.spec.read().map_err(|_e| FlexiLoggerError::Poison)?).clone())
    }
}
