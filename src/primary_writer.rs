mod multi_writer;
pub(crate) mod std_stream;
mod std_writer;
mod test_writer;

use self::{
    multi_writer::MultiWriter, std_stream::StdStream, std_writer::StdWriter,
    test_writer::TestWriter,
};
use crate::{
    filter::LogLineWriter,
    logger::Duplicate,
    writers::{FileLogWriter, LogWriter},
    DeferredNow, FlexiLoggerError, FormatFunction, LogfileSelector, WriteMode,
};
use log::Record;
use std::path::PathBuf;

// Primary writer
//
// all normal logging goes here
pub(crate) enum PrimaryWriter {
    // Writes to stdout or to stderr
    Std(StdWriter),
    // Writes to a file or to nowhere, with optional "duplication" to stderr or stdout
    Multi(MultiWriter),
    // Writes using println! to stdout, to enable capturing in tests
    Test(TestWriter),
}
impl PrimaryWriter {
    pub fn multi(
        duplicate_stderr: Duplicate,
        duplicate_stdout: Duplicate,
        support_capture: bool,
        format_for_stderr: FormatFunction,
        format_for_stdout: FormatFunction,
        o_file_writer: Option<Box<FileLogWriter>>,
        o_other_writer: Option<Box<dyn LogWriter>>,
    ) -> Self {
        Self::Multi(MultiWriter::new(
            duplicate_stderr,
            duplicate_stdout,
            support_capture,
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

    pub fn test(stdout: bool, format: FormatFunction) -> Self {
        Self::Test(TestWriter::new(stdout, format))
    }

    // Write out a log line.
    pub fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        match *self {
            Self::Std(ref w) => w.write(now, record),
            Self::Multi(ref w) => w.write(now, record),
            Self::Test(ref w) => w.write(now, record),
        }
    }

    // Flush any buffered records.
    pub fn flush(&self) -> std::io::Result<()> {
        match *self {
            Self::Std(ref w) => w.flush(),
            Self::Multi(ref w) => w.flush(),
            Self::Test(ref w) => w.flush(),
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
            Self::Test(writer) => {
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
            Self::Test(writer) => {
                writer.shutdown();
            }
        }
    }

    pub fn existing_log_files(
        &self,
        selector: &LogfileSelector,
    ) -> Result<Vec<PathBuf>, FlexiLoggerError> {
        match self {
            Self::Multi(multi_writer) => multi_writer.existing_log_files(selector),
            _ => Ok(Vec::new()),
        }
    }
}

impl LogLineWriter for PrimaryWriter {
    fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        self.write(now, record)
    }
}
