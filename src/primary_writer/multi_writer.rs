use crate::{
    logger::Duplicate,
    util::{eprint_err, write_buffered, ErrorCode},
    writers::{FileLogWriter, FileLogWriterBuilder, FileLogWriterConfig, LogWriter},
    LogfileSelector, {DeferredNow, FlexiLoggerError, FormatFunction},
};
use log::Record;
use std::{
    io::Write,
    path::PathBuf,
    sync::atomic::{AtomicU8, Ordering},
};

// The `MultiWriter` writes logs to a FileLogWriter and/or another Writer,
// and can duplicate messages to stderr or stdout.
pub(crate) struct MultiWriter {
    duplicate_stderr: AtomicU8,
    duplicate_stdout: AtomicU8,
    support_capture: bool,
    format_for_stderr: FormatFunction,
    format_for_stdout: FormatFunction,
    o_file_writer: Option<Box<FileLogWriter>>,
    o_other_writer: Option<Box<dyn LogWriter>>,
}

impl MultiWriter {
    pub(crate) fn new(
        duplicate_stderr: Duplicate,
        duplicate_stdout: Duplicate,
        support_capture: bool,
        format_for_stderr: FormatFunction,
        format_for_stdout: FormatFunction,
        o_file_writer: Option<Box<FileLogWriter>>,
        o_other_writer: Option<Box<dyn LogWriter>>,
    ) -> Self {
        MultiWriter {
            duplicate_stderr: AtomicU8::new(duplicate_stderr as u8),
            duplicate_stdout: AtomicU8::new(duplicate_stdout as u8),
            support_capture,
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
    pub(crate) fn reopen_output(&self) -> Result<(), FlexiLoggerError> {
        match (&self.o_file_writer, &self.o_other_writer) {
            (None, None) => Ok(()),
            (Some(ref w), None) => w.reopen_outputfile(),
            (None, Some(w)) => w.reopen_output(),
            (Some(w1), Some(w2)) => {
                let r1 = w1.reopen_outputfile();
                let r2 = w2.reopen_output();
                match (r1, r2) {
                    (Ok(()), Ok(())) => Ok(()),
                    (Err(e), _) | (Ok(()), Err(e)) => Err(e),
                }
            }
        }
    }
    pub(crate) fn trigger_rotation(&self) -> Result<(), FlexiLoggerError> {
        match (&self.o_file_writer, &self.o_other_writer) {
            (None, None) => Ok(()),
            (Some(ref w), None) => w.rotate(),
            (None, Some(w)) => w.rotate(),
            (Some(w1), Some(w2)) => {
                let r1 = w1.rotate();
                let r2 = w2.rotate();
                match (r1, r2) {
                    (Ok(()), Ok(())) => Ok(()),
                    (Err(e), _) | (Ok(()), Err(e)) => Err(e),
                }
            }
        }
    }
    pub(crate) fn existing_log_files(
        &self,
        selector: &LogfileSelector,
    ) -> Result<Vec<PathBuf>, FlexiLoggerError> {
        if let Some(fw) = self.o_file_writer.as_ref() {
            fw.existing_log_files(selector)
        } else {
            Ok(Vec::new())
        }
    }

    pub(crate) fn adapt_duplication_to_stderr(&self, dup: Duplicate) {
        self.duplicate_stderr.store(dup as u8, Ordering::Relaxed);
    }

    pub(crate) fn adapt_duplication_to_stdout(&self, dup: Duplicate) {
        self.duplicate_stdout.store(dup as u8, Ordering::Relaxed);
    }

    fn duplication_to_stderr(&self) -> Duplicate {
        Duplicate::from(self.duplicate_stderr.load(Ordering::Relaxed))
    }
    fn duplication_to_stdout(&self) -> Duplicate {
        Duplicate::from(self.duplicate_stdout.load(Ordering::Relaxed))
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
        if match self.duplication_to_stderr() {
            Duplicate::Error => record.level() == log::Level::Error,
            Duplicate::Warn => record.level() <= log::Level::Warn,
            Duplicate::Info => record.level() <= log::Level::Info,
            Duplicate::Debug => record.level() <= log::Level::Debug,
            Duplicate::Trace | Duplicate::All => true,
            Duplicate::None => false,
        } {
            if self.support_capture {
                let mut tmp_buf = Vec::<u8>::with_capacity(200);
                (self.format_for_stderr)(&mut tmp_buf, now, record)
                    .unwrap_or_else(|e| eprint_err(ErrorCode::Format, "formatting failed", &e));
                eprintln!("{}", String::from_utf8_lossy(&tmp_buf));
            } else {
                write_buffered(
                    self.format_for_stderr,
                    now,
                    record,
                    &mut std::io::stderr(),
                    #[cfg(test)]
                    None,
                )?;
            }
        }

        if match self.duplication_to_stdout() {
            Duplicate::Error => record.level() == log::Level::Error,
            Duplicate::Warn => record.level() <= log::Level::Warn,
            Duplicate::Info => record.level() <= log::Level::Info,
            Duplicate::Debug => record.level() <= log::Level::Debug,
            Duplicate::Trace | Duplicate::All => true,
            Duplicate::None => false,
        } {
            if self.support_capture {
                let mut tmp_buf = Vec::<u8>::with_capacity(200);
                (self.format_for_stdout)(&mut tmp_buf, now, record)
                    .unwrap_or_else(|e| eprint_err(ErrorCode::Format, "formatting failed", &e));
                println!("{}", String::from_utf8_lossy(&tmp_buf));
            } else {
                write_buffered(
                    self.format_for_stdout,
                    now,
                    record,
                    &mut std::io::stdout(),
                    #[cfg(test)]
                    None,
                )?;
            }
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

        if !matches!(self.duplication_to_stderr(), Duplicate::None) {
            std::io::stderr().flush()?;
        }
        if !matches!(self.duplication_to_stdout(), Duplicate::None) {
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
