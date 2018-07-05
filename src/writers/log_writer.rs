use log::Record;
use std::io;

/// Writes to a single log output stream.
///
/// Boxed instances of `LogWriter` can be used as additional log targets.
pub trait LogWriter: Sync + Send {
    /// write out a log line
    fn write(&self, record: &Record) -> io::Result<()>;

    /// Flushes any buffered records.
    fn flush(&self) -> io::Result<()>;
}
