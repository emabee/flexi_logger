use crate::{DeferredNow, FlexiLoggerError, FormatFunction};
use log::Record;

/// Writes to a single log output stream.
///
/// Boxed instances of `LogWriter` can be used as additional log targets
/// (see [writers](crate::writers) for more details).
pub trait LogWriter: Sync + Send {
    /// Writes out a log line.
    ///
    /// # Errors
    ///
    /// [`std::io::Error`]
    fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()>;

    /// Flushes any buffered records.
    ///
    /// # Errors
    ///
    /// [`std::io::Error`]
    fn flush(&self) -> std::io::Result<()>;

    /// Provides the maximum log level that is to be written.
    fn max_log_level(&self) -> log::LevelFilter {
        log::LevelFilter::Trace
    }

    /// Sets the format function.
    ///
    /// Defaults to [`default_format`](crate::default_format),
    /// but can be changed with a call to
    /// [`Logger::format_for_writer`](crate::Logger::format_for_writer).
    ///
    /// The default implementation is a no-op.
    fn format(&mut self, format: FormatFunction) {
        _ = format;
    }

    /// Cleanup open resources, if necessary.
    fn shutdown(&self) {}

    /// Re-open the current output, if meaningful.
    ///
    /// This method is called from
    /// [`LoggerHandle::reopen_output`](crate::LoggerHandle::reopen_output)
    /// for all registered additional writers.
    ///
    /// # Errors
    ///
    /// Depend on registered writers.
    fn reopen_output(&self) -> Result<(), FlexiLoggerError> {
        Ok(())
    }

    /// Rotate the current output, if meaningful.
    ///
    /// This method is called from
    /// [`LoggerHandle::trigger_rotation`](crate::LoggerHandle::trigger_rotation)
    /// for all registered additional writers.
    ///
    /// # Errors
    ///
    /// Depend on registered writers.
    fn rotate(&self) -> Result<(), FlexiLoggerError> {
        Ok(())
    }

    // Takes a vec with three patterns per line that represent the log line,
    // compares the written log with the expected lines,
    // and asserts that both are in sync.
    //
    // This function is not meant for productive code, only for tests.
    #[doc(hidden)]
    fn validate_logs(&self, _expected: &[(&'static str, &'static str, &'static str)]) {
        unimplemented!("only useful for tests");
    }
}
