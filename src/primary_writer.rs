use log;
use log::Record;
use std::io::Write;

use crate::logger::Duplicate;
use crate::writers::{FileLogWriter, LogWriter};
use crate::FormatFunction;

// Writes either to stderr,
// or to a file (with optional duplication to stderr),
// or to nowhere (with optional "duplication" to stderr).
#[allow(clippy::large_enum_variant)]
pub(crate) enum PrimaryWriter {
    StdErrWriter(StdErrWriter),
    ExtendedFileWriter(ExtendedFileWriter),
    BlackHole(BlackHoleWriter),
}
impl PrimaryWriter {
    pub fn file(
        duplicate: Duplicate,
        format_for_stderr: FormatFunction,
        file_log_writer: FileLogWriter,
    ) -> PrimaryWriter {
        PrimaryWriter::ExtendedFileWriter(ExtendedFileWriter {
            duplicate,
            format_for_stderr,
            file_log_writer,
        })
    }
    pub fn stderr(format: FormatFunction) -> PrimaryWriter {
        PrimaryWriter::StdErrWriter(StdErrWriter::new(format))
    }

    pub fn black_hole(duplicate: Duplicate, format: FormatFunction) -> PrimaryWriter {
        PrimaryWriter::BlackHole(BlackHoleWriter { duplicate, format })
    }

    // Write out a log line.
    pub fn write(&self, record: &Record) -> std::io::Result<()> {
        match *self {
            PrimaryWriter::StdErrWriter(ref w) => w.write(record),
            PrimaryWriter::ExtendedFileWriter(ref w) => w.write(record),
            PrimaryWriter::BlackHole(ref w) => w.write(record),
        }
    }

    // Flush any buffered records.
    pub fn flush(&self) -> std::io::Result<()> {
        match *self {
            PrimaryWriter::StdErrWriter(ref w) => w.flush(),
            PrimaryWriter::ExtendedFileWriter(ref w) => w.flush(),
            PrimaryWriter::BlackHole(ref w) => w.flush(),
        }
    }

    pub fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) -> bool {
        match *self {
            PrimaryWriter::StdErrWriter(_) => false,
            PrimaryWriter::ExtendedFileWriter(ref w) => w.validate_logs(expected),
            PrimaryWriter::BlackHole(_) => false,
        }
    }
}

// `StdErrWriter` writes logs to stderr.
pub(crate) struct StdErrWriter {
    format: FormatFunction,
}

impl StdErrWriter {
    fn new(format: FormatFunction) -> StdErrWriter {
        StdErrWriter { format }
    }
    #[inline]
    fn write(&self, record: &Record) -> std::io::Result<()> {
        let mut out = std::io::stderr();
        (self.format)(&mut out, record)?;
        out.write_all(b"\n")
    }

    #[inline]
    fn flush(&self) -> std::io::Result<()> {
        std::io::stderr().flush()
    }
}

// The `BlackHoleWriter` does not write any log, but can 'duplicate' messages to stderr.
pub(crate) struct BlackHoleWriter {
    duplicate: Duplicate,
    format: FormatFunction,
}
impl BlackHoleWriter {
    fn write(&self, record: &Record) -> std::io::Result<()> {
        let dupl = match self.duplicate {
            Duplicate::Error => record.level() == log::Level::Error,
            Duplicate::Warn => record.level() <= log::Level::Warn,
            Duplicate::Info => record.level() <= log::Level::Info,
            Duplicate::Debug => record.level() <= log::Level::Debug,
            Duplicate::Trace | Duplicate::All => true,
            Duplicate::None => false,
        };
        if dupl {
            (self.format)(&mut std::io::stderr(), record)?;
            std::io::stderr().write_all(b"\n")?;
        }
        Ok(())
    }

    fn flush(&self) -> std::io::Result<()> {
        std::io::stderr().flush()
    }
}

// The `ExtendedFileWriter` writes logs to stderr or to a `FileLogWriter`, and in the latter case
// can duplicate messages to stderr.
pub(crate) struct ExtendedFileWriter {
    duplicate: Duplicate,
    format_for_stderr: FormatFunction,
    file_log_writer: FileLogWriter,
}
impl ExtendedFileWriter {
    fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) -> bool {
        self.file_log_writer.validate_logs(expected)
    }

    fn write(&self, record: &Record) -> std::io::Result<()> {
        let dupl = match self.duplicate {
            Duplicate::Error => record.level() == log::Level::Error,
            Duplicate::Warn => record.level() <= log::Level::Warn,
            Duplicate::Info => record.level() <= log::Level::Info,
            Duplicate::Debug => record.level() <= log::Level::Debug,
            Duplicate::Trace | Duplicate::All => true,
            Duplicate::None => false,
        };
        if dupl {
            (self.format_for_stderr)(&mut std::io::stderr(), record)?;
            std::io::stderr().write_all(b"\n")?;
        }
        self.file_log_writer.write(record)
    }

    fn flush(&self) -> std::io::Result<()> {
        self.file_log_writer.flush()?;
        std::io::stderr().flush()
    }
}
