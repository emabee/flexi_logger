use log::Record;
use std::cell::RefCell;
use std::io::Write;

use crate::deferred_now::DeferredNow;
use crate::logger::Duplicate;
use crate::writers::{FileLogWriter, LogWriter};
use crate::FormatFunction;

// Writes either to stderr,
// or to a file (with optional duplication to stderr),
// or to nowhere (with optional "duplication" to stderr).
#[allow(clippy::large_enum_variant)]
pub(crate) enum PrimaryWriter {
    StdOutWriter(StdOutWriter),
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

    pub fn stdout(format: FormatFunction) -> PrimaryWriter {
        PrimaryWriter::StdOutWriter(StdOutWriter::new(format))
    }

    pub fn black_hole(duplicate: Duplicate, format: FormatFunction) -> PrimaryWriter {
        PrimaryWriter::BlackHole(BlackHoleWriter { duplicate, format })
    }

    // Write out a log line.
    pub fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        match *self {
            PrimaryWriter::StdErrWriter(ref w) => w.write(now, record),
            PrimaryWriter::StdOutWriter(ref w) => w.write(now, record),
            PrimaryWriter::ExtendedFileWriter(ref w) => w.write(now, record),
            PrimaryWriter::BlackHole(ref w) => w.write(now, record),
        }
    }

    // Flush any buffered records.
    pub fn flush(&self) -> std::io::Result<()> {
        match *self {
            PrimaryWriter::StdErrWriter(ref w) => w.flush(),
            PrimaryWriter::StdOutWriter(ref w) => w.flush(),
            PrimaryWriter::ExtendedFileWriter(ref w) => w.flush(),
            PrimaryWriter::BlackHole(ref w) => w.flush(),
        }
    }

    pub fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) -> bool {
        match *self {
            PrimaryWriter::StdErrWriter(_) => false,
            PrimaryWriter::StdOutWriter(_) => false,
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
    fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        write_buffered(self.format, now, record, &mut std::io::stderr().lock());
        Ok(())
    }

    #[inline]
    fn flush(&self) -> std::io::Result<()> {
        std::io::stderr().flush()
    }
}

// `StdOutWriter` writes logs to stderr.
pub(crate) struct StdOutWriter {
    format: FormatFunction,
}

impl StdOutWriter {
    fn new(format: FormatFunction) -> StdOutWriter {
        StdOutWriter { format }
    }
    #[inline]
    fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        write_buffered(self.format, now, record, &mut std::io::stdout().lock());
        Ok(())
    }

    #[inline]
    fn flush(&self) -> std::io::Result<()> {
        std::io::stdout().flush()
    }
}

// The `BlackHoleWriter` does not write any log, but can 'duplicate' messages to stderr.
pub(crate) struct BlackHoleWriter {
    duplicate: Duplicate,
    format: FormatFunction,
}
impl BlackHoleWriter {
    fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        let dupl = match self.duplicate {
            Duplicate::Error => record.level() == log::Level::Error,
            Duplicate::Warn => record.level() <= log::Level::Warn,
            Duplicate::Info => record.level() <= log::Level::Info,
            Duplicate::Debug => record.level() <= log::Level::Debug,
            Duplicate::Trace | Duplicate::All => true,
            Duplicate::None => false,
        };
        if dupl {
            (self.format)(&mut std::io::stderr(), now, record)?;
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

    fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        let dupl = match self.duplicate {
            Duplicate::Error => record.level() == log::Level::Error,
            Duplicate::Warn => record.level() <= log::Level::Warn,
            Duplicate::Info => record.level() <= log::Level::Info,
            Duplicate::Debug => record.level() <= log::Level::Debug,
            Duplicate::Trace | Duplicate::All => true,
            Duplicate::None => false,
        };
        if dupl {
            write_buffered(self.format_for_stderr, now, record, &mut std::io::stderr());
        }
        self.file_log_writer.write(now, record)
    }

    fn flush(&self) -> std::io::Result<()> {
        self.file_log_writer.flush()?;
        std::io::stderr().flush()
    }
}

// Use a thread-local buffer for writing to stderr
fn write_buffered(
    format_function: FormatFunction,
    now: &mut DeferredNow,
    record: &Record,
    w: &mut dyn Write,
) {
    thread_local! {
        static BUFFER: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(200));
    }
    BUFFER.with(|tl_buf| match tl_buf.try_borrow_mut() {
        Ok(mut buffer) => {
            (format_function)(&mut *buffer, now, record).unwrap_or_else(|e| write_err(ERR_1, e));
            w.write_all(&*buffer)
                .unwrap_or_else(|e| write_err(ERR_2, e));
            w.write_all(b"\n").unwrap_or_else(|e| write_err(ERR_2, e));
            buffer.clear();
        }
        Err(_e) => {
            // We arrive here in the rare cases of recursive logging
            // (e.g. log calls in Debug or Display implementations)
            let mut tmp_buf = Vec::<u8>::with_capacity(200);
            (format_function)(&mut tmp_buf, now, record).unwrap_or_else(|e| write_err(ERR_1, e));
            w.write_all(&tmp_buf)
                .unwrap_or_else(|e| write_err(ERR_2, e));
            w.write_all(b"\n").unwrap_or_else(|e| write_err(ERR_2, e));
        }
    });
}

const ERR_1: &str = "FlexiLogger: formatting failed with ";
const ERR_2: &str = "FlexiLogger: writing failed with ";

fn write_err(msg: &str, err: std::io::Error) {
    eprintln!("{} with {}", msg, err);
}
