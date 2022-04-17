mod multi_writer;
pub(crate) mod std_stream;
mod std_writer;

use self::{multi_writer::MultiWriter, std_stream::StdStream, std_writer::StdWriter};
use crate::{
    filter::LogLineWriter,
    logger::Duplicate,
    writers::{FileLogWriter, LogWriter},
    {DeferredNow, FormatFunction, WriteMode},
};
use log::Record;

// Writes either to stdout, or to stderr,
// or to a file (with optional duplication to stderr or stdout),
// or to nowhere (with optional "duplication" to stderr or stdout).
pub(crate) enum PrimaryWriter {
    Std(StdWriter),
    Multi(MultiWriter),
}
impl PrimaryWriter {
    pub fn multi(
        duplicate_stderr: Duplicate,
        duplicate_stdout: Duplicate,
        format_for_stderr: FormatFunction,
        format_for_stdout: FormatFunction,
        o_file_writer: Option<Box<FileLogWriter>>,
        o_other_writer: Option<Box<dyn LogWriter>>,
    ) -> Self {
        Self::Multi(MultiWriter::new(
            duplicate_stderr,
            duplicate_stdout,
            format_for_stderr,
            format_for_stdout,
            o_file_writer,
            o_other_writer,
        ))
    }
    pub fn stderr(format: FormatFunction, write_mode: &WriteMode) -> Self {
        Self::Std(StdWriter::new(
            StdStream::Err(std::io::stderr()),
            format,
            write_mode,
        ))
    }

    pub fn stdout(format: FormatFunction, write_mode: &WriteMode) -> Self {
        Self::Std(StdWriter::new(
            StdStream::Out(std::io::stdout()),
            format,
            write_mode,
        ))
    }

    // Write out a log line.
    pub fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        match *self {
            Self::Std(ref w) => w.write(now, record),
            Self::Multi(ref w) => w.write(now, record),
        }
    }

    // Flush any buffered records.
    pub fn flush(&self) -> std::io::Result<()> {
        match *self {
            Self::Std(ref w) => w.flush(),
            Self::Multi(ref w) => w.flush(),
        }
    }

    pub fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) {
        self.shutdown();
        match self {
            Self::Std(writer) => {
                writer.validate_logs(expected);
            }
            Self::Multi(writer) => {
                writer.validate_logs(expected);
            }
        }
    }

    pub fn shutdown(&self) {
        self.flush().ok();
        match self {
            Self::Std(writer) => {
                writer.shutdown();
            }
            Self::Multi(writer) => {
                writer.shutdown();
            }
        }
    }
}

impl LogLineWriter for PrimaryWriter {
    fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        self.write(now, record)
    }
}
