use crate::logger::Duplicate;
use crate::util::write_buffered;
use crate::writers::{FileLogWriter, FileLogWriterBuilder, FileLogWriterConfig, LogWriter};
use crate::{DeferredNow, FlexiLoggerError, FormatFunction};
use log::Record;
use std::io::Write;

// The `MultiWriter` writes logs to stderr or to a set of `Writer`s, and in the latter case
// can duplicate messages to stderr.
pub(crate) struct MultiWriter {
    duplicate_stderr: Duplicate,
    duplicate_stdout: Duplicate,
    format_for_stderr: FormatFunction,
    format_for_stdout: FormatFunction,
    o_file_writer: Option<Box<FileLogWriter>>,
    o_other_writer: Option<Box<dyn LogWriter>>,
}

impl MultiWriter {
    pub(crate) fn new(
        duplicate_stderr: Duplicate,
        duplicate_stdout: Duplicate,
        format_for_stderr: FormatFunction,
        format_for_stdout: FormatFunction,
        o_file_writer: Option<Box<FileLogWriter>>,
        o_other_writer: Option<Box<dyn LogWriter>>,
    ) -> Self {
        MultiWriter {
            duplicate_stderr,
            duplicate_stdout,
            format_for_stderr,
            format_for_stdout,
            o_file_writer,
            o_other_writer,
        }
    }
    pub(crate) fn reset_file_log_writer(
        &self,
        flwb: &FileLogWriterBuilder,
    ) -> Result<(), FlexiLoggerError> {
        self.o_file_writer
            .as_ref()
            .map_or(Err(FlexiLoggerError::NoFileLogger), |flw| flw.reset(flwb))
    }
    pub(crate) fn flw_config(&self) -> Result<FileLogWriterConfig, FlexiLoggerError> {
        self.o_file_writer
            .as_ref()
            .map_or(Err(FlexiLoggerError::NoFileLogger), |flw| flw.config())
    }
    pub(crate) fn reopen_outputfile(&self) -> Result<(), FlexiLoggerError> {
        self.o_file_writer
            .as_ref()
            .map_or(Err(FlexiLoggerError::NoFileLogger), |flw| {
                flw.reopen_outputfile()
            })
    }
}

impl LogWriter for MultiWriter {
    fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) {
        if let Some(ref writer) = self.o_file_writer {
            (*writer).validate_logs(expected);
        }
        if let Some(ref writer) = self.o_other_writer {
            (*writer).validate_logs(expected);
        }
    }

    fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        if match self.duplicate_stderr {
            Duplicate::Error => record.level() == log::Level::Error,
            Duplicate::Warn => record.level() <= log::Level::Warn,
            Duplicate::Info => record.level() <= log::Level::Info,
            Duplicate::Debug => record.level() <= log::Level::Debug,
            Duplicate::Trace | Duplicate::All => true,
            Duplicate::None => false,
        } {
            write_buffered(
                self.format_for_stderr,
                now,
                record,
                &mut std::io::stderr(),
                #[cfg(test)]
                None,
            )?;
        }

        if match self.duplicate_stdout {
            Duplicate::Error => record.level() == log::Level::Error,
            Duplicate::Warn => record.level() <= log::Level::Warn,
            Duplicate::Info => record.level() <= log::Level::Info,
            Duplicate::Debug => record.level() <= log::Level::Debug,
            Duplicate::Trace | Duplicate::All => true,
            Duplicate::None => false,
        } {
            write_buffered(
                self.format_for_stdout,
                now,
                record,
                &mut std::io::stdout(),
                #[cfg(test)]
                None,
            )?;
        }

        if let Some(ref writer) = self.o_file_writer {
            writer.write(now, record)?;
        }
        if let Some(ref writer) = self.o_other_writer {
            writer.write(now, record)?;
        }
        Ok(())
    }

    /// Provides the maximum log level that is to be written.
    fn max_log_level(&self) -> log::LevelFilter {
        *self
            .o_file_writer
            .as_ref()
            .map(|w| w.max_log_level())
            .iter()
            .chain(
                self.o_other_writer
                    .as_ref()
                    .map(|w| w.max_log_level())
                    .iter(),
            )
            .max()
            .unwrap(/*ok*/)
    }

    fn flush(&self) -> std::io::Result<()> {
        if let Some(ref writer) = self.o_file_writer {
            writer.flush()?;
        }
        if let Some(ref writer) = self.o_other_writer {
            writer.flush()?;
        }

        if !matches!(self.duplicate_stderr, Duplicate::None) {
            std::io::stderr().flush()?;
        }
        if !matches!(self.duplicate_stdout, Duplicate::None) {
            std::io::stdout().flush()?;
        }
        Ok(())
    }

    fn shutdown(&self) {
        if let Some(ref writer) = self.o_file_writer {
            writer.shutdown();
        }
        if let Some(ref writer) = self.o_other_writer {
            writer.shutdown();
        }
    }
}
