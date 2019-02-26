use log;
use log::Record;
use std::io::{self, Write};

use crate::logger::Duplicate;
use crate::writers::FileLogWriter;
use crate::writers::LogWriter;
use crate::FormatFunction;

// Writes either to stderr or to a file.
#[allow(clippy::large_enum_variant)]
pub(crate) enum PrimaryWriter {
    StdErrWriter(StdErrWriter),
    ExtendedFileWriter(ExtendedFileWriter),
    BlackHole,
}
impl PrimaryWriter {
    pub fn file(duplicate: Duplicate, file_log_writer: FileLogWriter) -> PrimaryWriter {
        PrimaryWriter::ExtendedFileWriter(ExtendedFileWriter {
            duplicate,
            file_log_writer,
        })
    }
    pub fn stderr(format: FormatFunction) -> PrimaryWriter {
        PrimaryWriter::StdErrWriter(StdErrWriter { format })
    }

    // Write out a log line.
    pub fn write(&self, record: &Record) -> io::Result<()> {
        match *self {
            PrimaryWriter::StdErrWriter(ref w) => w.write(record),
            PrimaryWriter::ExtendedFileWriter(ref w) => w.write(record),
            PrimaryWriter::BlackHole => Ok(()),
        }
    }

    // Flush any buffered records.
    pub fn flush(&self) -> io::Result<()> {
        match *self {
            PrimaryWriter::StdErrWriter(ref w) => w.flush(),
            PrimaryWriter::ExtendedFileWriter(ref w) => w.flush(),
            PrimaryWriter::BlackHole => Ok(()),
        }
    }

    pub(crate) fn validate_logs(
        &self,
        expected: &[(&'static str, &'static str, &'static str)],
    ) -> bool {
        match *self {
            PrimaryWriter::StdErrWriter(_) => false,
            PrimaryWriter::ExtendedFileWriter(ref w) => w.validate_logs(expected),
            PrimaryWriter::BlackHole => false,
        }
    }
}

/// `StdErrWriter` writes logs to stderr.
pub struct StdErrWriter {
    format: FormatFunction,
}

impl StdErrWriter {
    #[inline]
    fn write(&self, record: &Record) -> io::Result<()> {
        write_to_stderr(self.format, record)
    }

    #[inline]
    fn flush(&self) -> io::Result<()> {
        io::stderr().flush()
    }
}

/// `ExtendedFileWriter` writes logs to stderr or to a `FileLogWriter`, and in the latter case
/// can duplicate messages to stderr.
pub struct ExtendedFileWriter {
    duplicate: Duplicate,
    file_log_writer: FileLogWriter,
}
impl ExtendedFileWriter {
    #[doc(hidden)]
    pub fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) -> bool {
        self.file_log_writer.validate_logs(expected)
    }

    fn write(&self, record: &Record) -> io::Result<()> {
        if match self.duplicate {
            Duplicate::Error => record.level() == log::Level::Error,
            Duplicate::Warn => record.level() <= log::Level::Warn,
            Duplicate::Info => record.level() <= log::Level::Info,
            Duplicate::Debug => record.level() <= log::Level::Debug,
            Duplicate::Trace | Duplicate::All => true,
            Duplicate::None => false,
        } {
            write_to_stderr(self.file_log_writer.format(), record)?;
        }
        self.file_log_writer.write(record)
    }

    fn flush(&self) -> io::Result<()> {
        self.file_log_writer.flush()?;
        io::stderr().flush()
    }
}

#[inline]
fn write_to_stderr(f: FormatFunction, record: &Record) -> io::Result<()> {
    (f)(&mut io::stderr(), record)?;
    io::stderr().write_all(b"\n")
}
