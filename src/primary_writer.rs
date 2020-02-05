use log::Record;
use std::cell::RefCell;
use std::io::Write;

use crate::deferred_now::DeferredNow;
use crate::logger::Duplicate;
use crate::writers::LogWriter;
use crate::FormatFunction;

// Writes either to stderr,
// or to a file (with optional duplication to stderr),
// or to nowhere (with optional "duplication" to stderr).
#[allow(clippy::large_enum_variant)]
pub(crate) enum PrimaryWriter {
    StdOutWriter(StdOutWriter),
    StdErrWriter(StdErrWriter),
    MultiWriter(MultiWriter),
    BlackHole(BlackHoleWriter),
}
impl PrimaryWriter {
    pub fn multi(
        duplicate: Duplicate,
        format_for_stderr: FormatFunction,
        writers: Vec<Box<dyn LogWriter>>,
    ) -> Self {
        Self::MultiWriter(MultiWriter {
            duplicate,
            format_for_stderr,
            writers,
        })
    }
    pub fn stderr(format: FormatFunction) -> Self {
        Self::StdErrWriter(StdErrWriter::new(format))
    }

    pub fn stdout(format: FormatFunction) -> Self {
        Self::StdOutWriter(StdOutWriter::new(format))
    }

    pub fn black_hole(duplicate: Duplicate, format: FormatFunction) -> Self {
        Self::BlackHole(BlackHoleWriter { duplicate, format })
    }

    // Write out a log line.
    pub fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        match *self {
            Self::StdErrWriter(ref w) => w.write(now, record),
            Self::StdOutWriter(ref w) => w.write(now, record),
            Self::MultiWriter(ref w) => w.write(now, record),
            Self::BlackHole(ref w) => w.write(now, record),
        }
    }

    // Flush any buffered records.
    pub fn flush(&self) -> std::io::Result<()> {
        match *self {
            Self::StdErrWriter(ref w) => w.flush(),
            Self::StdOutWriter(ref w) => w.flush(),
            Self::MultiWriter(ref w) => w.flush(),
            Self::BlackHole(ref w) => w.flush(),
        }
    }

    pub fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) {
        if let Self::MultiWriter(ref w) = *self {
            w.validate_logs(expected);
        }
    }
}

// `StdErrWriter` writes logs to stderr.
pub(crate) struct StdErrWriter {
    format: FormatFunction,
}

impl StdErrWriter {
    fn new(format: FormatFunction) -> Self {
        Self { format }
    }
    #[inline]
    fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        write_buffered(self.format, now, record, &mut std::io::stderr())
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
    fn new(format: FormatFunction) -> Self {
        Self { format }
    }
    #[inline]
    fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        write_buffered(self.format, now, record, &mut std::io::stdout())
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

// The `MultiWriter` writes logs to stderr or to a set of `Writer`s, and in the latter case
// can duplicate messages to stderr.
pub(crate) struct MultiWriter {
    duplicate: Duplicate,
    format_for_stderr: FormatFunction,
    writers: Vec<Box<dyn LogWriter>>,
}
impl LogWriter for MultiWriter {
    fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) {
        for writer in &self.writers {
            (*writer).validate_logs(expected);
        }
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
            write_buffered(self.format_for_stderr, now, record, &mut std::io::stderr())?;
        }
        for writer in &self.writers {
            writer.write(now, record)?;
        }
        Ok(())
    }

    /// Provides the maximum log level that is to be written.
    fn max_log_level(&self) -> log::LevelFilter {
        self.writers
            .iter()
            .map(|w| w.max_log_level())
            .max()
            .unwrap()
    }

    fn flush(&self) -> std::io::Result<()> {
        for writer in &self.writers {
            writer.flush()?;
        }
        std::io::stderr().flush()
    }
    fn shutdown(&self) {
        for writer in &self.writers {
            writer.shutdown();
        }
    }
}

// Use a thread-local buffer for writing to stderr
fn write_buffered(
    format_function: FormatFunction,
    now: &mut DeferredNow,
    record: &Record,
    w: &mut dyn Write,
) -> Result<(), std::io::Error> {
    let mut result: Result<(), std::io::Error> = Ok(());

    buffer_with(|tl_buf| match tl_buf.try_borrow_mut() {
        Ok(mut buffer) => {
            (format_function)(&mut *buffer, now, record)
                .unwrap_or_else(|e| write_err(ERR_FORMATTING, &e));
            buffer
                .write_all(b"\n")
                .unwrap_or_else(|e| write_err(ERR_FORMATTING, &e));

            result = w.write_all(&*buffer).map_err(|e| {
                write_err(ERR_WRITING, &e);
                e
            });

            buffer.clear();
        }
        Err(_e) => {
            // We arrive here in the rare cases of recursive logging
            // (e.g. log calls in Debug or Display implementations)
            // we print the inner calls, in chronological order, before finally the
            // outer most message is printed
            let mut tmp_buf = Vec::<u8>::with_capacity(200);
            (format_function)(&mut tmp_buf, now, record)
                .unwrap_or_else(|e| write_err(ERR_FORMATTING, &e));
            tmp_buf
                .write_all(b"\n")
                .unwrap_or_else(|e| write_err(ERR_FORMATTING, &e));

            result = w.write_all(&tmp_buf).map_err(|e| {
                write_err(ERR_WRITING, &e);
                e
            });
        }
    });
    result
}

pub(crate) fn buffer_with<F>(f: F)
where
    F: FnOnce(&RefCell<Vec<u8>>) -> (),
{
    thread_local! {
        static BUFFER: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(200));
    }
    BUFFER.with(f);
}

const ERR_FORMATTING: &str = "formatting failed with ";
const ERR_WRITING: &str = "writing failed with ";

fn write_err(msg: &str, err: &std::io::Error) {
    eprintln!("[flexi_logger] {} with {}", msg, err);
}
