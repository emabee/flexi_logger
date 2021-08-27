mod multi_writer;
mod stderr_writer;
mod stdout_writer;

use self::multi_writer::MultiWriter;
use self::stderr_writer::StdErrWriter;
use self::stdout_writer::StdOutWriter;
use crate::deferred_now::DeferredNow;
use crate::filter::LogLineWriter;
use crate::logger::Duplicate;
use crate::writers::{FileLogWriter, FlWriteMode, LogWriter};
use crate::FormatFunction;
use log::Record;

// Writes either to stdout, or to stderr,
// or to a file (with optional duplication to stderr),
// or to nowhere (with optional "duplication" to stderr).
#[allow(clippy::large_enum_variant)]
pub(crate) enum PrimaryWriter {
    StdOut(StdOutWriter),
    StdErr(StdErrWriter),
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
    pub fn stderr(format: FormatFunction, fl_write_mode: &FlWriteMode) -> Self {
        Self::StdErr(StdErrWriter::new(format, fl_write_mode))
    }

    pub fn stdout(format: FormatFunction, fl_write_mode: &FlWriteMode) -> Self {
        Self::StdOut(StdOutWriter::new(format, fl_write_mode))
    }

    // Write out a log line.
    pub fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        match *self {
            Self::StdErr(ref w) => w.write(now, record),
            Self::StdOut(ref w) => w.write(now, record),
            Self::Multi(ref w) => w.write(now, record),
        }
    }

    // Flush any buffered records.
    pub fn flush(&self) -> std::io::Result<()> {
        match *self {
            Self::StdErr(ref w) => w.flush(),
            Self::StdOut(ref w) => w.flush(),
            Self::Multi(ref w) => w.flush(),
        }
    }

    pub fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) {
        if let Self::Multi(ref w) = *self {
            w.validate_logs(expected);
        }
    }

    pub fn shutdown(&self) {
        self.flush().ok();
        match self {
            PrimaryWriter::StdOut(writer) => {
                writer.shutdown();
            }
            PrimaryWriter::StdErr(writer) => {
                writer.shutdown();
            }
            PrimaryWriter::Multi(writer) => {
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
