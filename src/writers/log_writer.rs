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
}
