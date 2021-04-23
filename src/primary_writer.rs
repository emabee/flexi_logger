use crate::deferred_now::DeferredNow;
use crate::logger::Duplicate;
use crate::writers::LogWriter;
use crate::FormatFunction;
use log::Record;
use std::cell::RefCell;
use std::io::{BufWriter, Write};
use std::sync::Mutex;

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
        writers: Vec<Box<dyn LogWriter>>,
    ) -> Self {
        Self::Multi(MultiWriter {
            duplicate_stderr,
            duplicate_stdout,
            format_for_stderr,
            format_for_stdout,
            writers,
        })
    }
    pub fn stderr(format: FormatFunction, o_buffer_capacity: &Option<usize>) -> Self {
        Self::StdErr(StdErrWriter::new(format, o_buffer_capacity))
    }

    pub fn stdout(format: FormatFunction, o_buffer_capacity: &Option<usize>) -> Self {
        Self::StdOut(StdOutWriter::new(format, o_buffer_capacity))
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
        if let PrimaryWriter::Multi(writer) = self {
            writer.shutdown();
        }
    }
}

// `StdErrWriter` writes logs to stderr.
pub(crate) struct StdErrWriter {
    format: FormatFunction,
    writer: ErrWriter,
}
enum ErrWriter {
    Unbuffered(std::io::Stderr),
    Buffered(Mutex<BufWriter<std::io::Stderr>>),
}
impl StdErrWriter {
    fn new(format: FormatFunction, o_buffer_capacity: &Option<usize>) -> Self {
        match o_buffer_capacity {
            Some(capacity) => Self {
                format,
                writer: ErrWriter::Buffered(Mutex::new(BufWriter::with_capacity(
                    *capacity,
                    std::io::stderr(),
                ))),
            },
            None => Self {
                format,
                writer: ErrWriter::Unbuffered(std::io::stderr()),
            },
        }
    }
    #[inline]
    fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        match &self.writer {
            ErrWriter::Unbuffered(stderr) => {
                let mut w = stderr.lock();
                write_buffered(self.format, now, record, &mut w)
            }
            ErrWriter::Buffered(mbuf_w) => {
                let mut w = mbuf_w.lock().map_err(|e| poison_err("stderr", &e))?;
                write_buffered(self.format, now, record, &mut *w)
            }
        }
    }

    #[inline]
    fn flush(&self) -> std::io::Result<()> {
        match &self.writer {
            ErrWriter::Unbuffered(stderr) => {
                let mut w = stderr.lock();
                w.flush()
            }
            ErrWriter::Buffered(mbuf_w) => {
                let mut w = mbuf_w.lock().map_err(|e| poison_err("stderr", &e))?;
                w.flush()
            }
        }
    }
}

fn poison_err(s: &'static str, _e: &dyn std::error::Error) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, s)
}

// `StdOutWriter` writes logs to stdout.
pub(crate) struct StdOutWriter {
    format: FormatFunction,
    writer: OutWriter,
}
enum OutWriter {
    Unbuffered(std::io::Stdout),
    Buffered(Mutex<BufWriter<std::io::Stdout>>),
}
impl StdOutWriter {
    fn new(format: FormatFunction, o_buffer_capacity: &Option<usize>) -> Self {
        match o_buffer_capacity {
            Some(capacity) => Self {
                format,
                writer: OutWriter::Buffered(Mutex::new(BufWriter::with_capacity(
                    *capacity,
                    std::io::stdout(),
                ))),
            },
            None => Self {
                format,
                writer: OutWriter::Unbuffered(std::io::stdout()),
            },
        }
    }
    #[inline]
    fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        match &self.writer {
            OutWriter::Unbuffered(stdout) => {
                let mut w = stdout.lock();
                write_buffered(self.format, now, record, &mut w)
            }
            OutWriter::Buffered(mbuf_w) => {
                let mut w = mbuf_w.lock().map_err(|e| poison_err("stdout", &e))?;
                write_buffered(self.format, now, record, &mut *w)
            }
        }
    }

    #[inline]
    fn flush(&self) -> std::io::Result<()> {
        match &self.writer {
            OutWriter::Unbuffered(stdout) => {
                let mut w = stdout.lock();
                w.flush()
            }
            OutWriter::Buffered(mbuf_w) => {
                let mut w = mbuf_w.lock().map_err(|e| poison_err("stdout", &e))?;
                w.flush()
            }
        }
    }
}

// The `MultiWriter` writes logs to stderr or to a set of `Writer`s, and in the latter case
// can duplicate messages to stderr.
pub(crate) struct MultiWriter {
    duplicate_stderr: Duplicate,
    duplicate_stdout: Duplicate,
    format_for_stderr: FormatFunction,
    format_for_stdout: FormatFunction,
    writers: Vec<Box<dyn LogWriter>>,
}

impl LogWriter for MultiWriter {
    fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) {
        for writer in &self.writers {
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
            write_buffered(self.format_for_stderr, now, record, &mut std::io::stderr())?;
        }

        if match self.duplicate_stdout {
            Duplicate::Error => record.level() == log::Level::Error,
            Duplicate::Warn => record.level() <= log::Level::Warn,
            Duplicate::Info => record.level() <= log::Level::Info,
            Duplicate::Debug => record.level() <= log::Level::Debug,
            Duplicate::Trace | Duplicate::All => true,
            Duplicate::None => false,
        } {
            write_buffered(self.format_for_stdout, now, record, &mut std::io::stdout())?;
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
        if let Duplicate::None = self.duplicate_stderr {
            std::io::stderr().flush()?;
        }
        if let Duplicate::None = self.duplicate_stdout {
            std::io::stdout().flush()?;
        }
        // maybe nicer, but doesn't work with rustc 1.37:
        // if !matches!(self.duplicate_stderr, Duplicate::None) {
        //     std::io::stderr().flush()?;
        // }
        // if !matches!(self.duplicate_stdout, Duplicate::None) {
        //     std::io::stdout().flush()?;
        // }
        Ok(())
    }

    fn shutdown(&self) {
        for writer in &self.writers {
            writer.shutdown();
        }
    }
}

// Use a thread-local buffer for writing to stderr or stdout
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
    F: FnOnce(&RefCell<Vec<u8>>),
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
