use log;
use log::Record;
use std::io;
use std::io::Write;

use writers::FileLogWriter;
use writers::LogWriter;
use FormatFunction;

// Writes either to stderr or to a file.
#[allow(unknown_lints)]
#[allow(large_enum_variant)]
pub enum PrimaryWriter {
    StdErrWriter(StdErrWriter),
    ExtendedFileWriter(ExtendedFileWriter),
}
impl PrimaryWriter {
    pub fn file(duplicate_error: bool, duplicate_info: bool, w: FileLogWriter) -> PrimaryWriter {
        PrimaryWriter::ExtendedFileWriter(ExtendedFileWriter {
            duplicate_error,
            duplicate_info,
            w,
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
        }
    }

    // Flush any buffered records.
    pub fn flush(&self) -> io::Result<()> {
        match *self {
            PrimaryWriter::StdErrWriter(ref w) => w.flush(),
            PrimaryWriter::ExtendedFileWriter(ref w) => w.flush(),
        }
    }

    pub fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) -> bool {
        match *self {
            PrimaryWriter::StdErrWriter(_) => false,
            PrimaryWriter::ExtendedFileWriter(ref w) => w.validate_logs(expected),
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
/// can duplicate some messages to stdout.
pub struct ExtendedFileWriter {
    duplicate_error: bool,
    duplicate_info: bool,
    w: FileLogWriter,
}
impl ExtendedFileWriter {
    pub fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) -> bool {
        self.w.validate_logs(expected)
    }

    fn write(&self, record: &Record) -> io::Result<()> {
        if self.duplicate_error && record.level() == log::Level::Error
            || self.duplicate_info
                && (record.level() == log::Level::Error || record.level() == log::Level::Warn
                    || record.level() == log::Level::Info)
        {
            write_to_stderr(self.w.format(), record)?;
        }
        self.w.write(record)
    }

    fn flush(&self) -> io::Result<()> {
        self.w.flush()?;
        io::stderr().flush()
    }
}

#[inline]
fn write_to_stderr(f: FormatFunction, record: &Record) -> io::Result<()> {
    (f)(&mut io::stderr(), record)?;
    io::stderr().write_all(b"\n")
}
