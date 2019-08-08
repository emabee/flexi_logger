use crate::deferred_now::DeferredNow;
use log::Record;
use std::io;

/// Writes to a single log output stream.
///
/// Boxed instances of `LogWriter` can be used as additional log targets.
pub trait LogWriter: Sync + Send {
    /// Writes out a log line.
    fn write(&self, now: &mut DeferredNow, record: &Record) -> io::Result<()>;

    /// Flushes any buffered records.
    fn flush(&self) -> io::Result<()>;

    /// Provides the maximum log level that is to be written.
    fn max_log_level(&self) -> log::LevelFilter;

    /// Takes a vec with three patterns per line that represent the log out,
    /// compares the written log with the expected lines,
    /// and asserts that both are in sync.
    ///
    /// This function is not meant for productive code, only for tests.
    #[doc(hidden)]
    fn validate_logs(&self, _expected: &[(&'static str, &'static str, &'static str)]) {
        unimplemented!("only useful for tests");
    }
}
