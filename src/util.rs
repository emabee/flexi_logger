use crate::logger::ErrorChannel;
use crate::{DeferredNow, FormatFunction};
use log::Record;
use std::cell::RefCell;
use std::io::Write;
use std::path::Path;
use std::sync::RwLock;

#[cfg(test)]
use std::io::Cursor;
#[cfg(test)]
use std::sync::{Arc, Mutex};

#[cfg(feature = "async")]
pub(crate) const ASYNC_FLUSH: &[u8] = b"F";
#[cfg(feature = "async")]
pub(crate) const ASYNC_SHUTDOWN: &[u8] = b"S";

#[derive(Copy, Clone, Debug)]
pub(crate) enum ErrorCode {
    Write,
    Flush,
    Format,
    LogFile,
    #[cfg(feature = "external_rotation")]
    LogFileWatcher,
    #[cfg(feature = "specfile")]
    LogSpecFile,
    Poison,
    #[cfg(target_family = "unix")]
    Symlink,
    WriterSpec,
}
impl ErrorCode {
    fn as_index(self) -> &'static str {
        match self {
            Self::Write => "write",
            Self::Flush => "flush",
            Self::Format => "format",
            Self::LogFile => "logfile",
            #[cfg(feature = "external_rotation")]
            Self::LogFileWatcher => "logfilewatcher",
            #[cfg(feature = "specfile")]
            Self::LogSpecFile => "logspecfile",
            Self::Poison => "poison",
            #[cfg(target_family = "unix")]
            Self::Symlink => "symlink",
            Self::WriterSpec => "writerspec",
        }
    }
}

pub(crate) fn eprint_err(error_code: ErrorCode, msg: &str, err: &dyn std::error::Error) {
    let s = format!(
        "[flexi_logger][ERRCODE::{code:?}] {msg}, caused by {err:?}\n    \
         See https://docs.rs/flexi_logger/latest/flexi_logger/error_info/index.html#{code_lc}",
        msg = msg,
        err = err,
        code = error_code,
        code_lc = error_code.as_index(),
    );
    try_to_write(&s);
}

pub(crate) fn eprint_msg(error_code: ErrorCode, msg: &str) {
    let s = format!(
        "[flexi_logger][ERRCODE::{code:?}] {msg}\n    \
         See https://docs.rs/flexi_logger/latest/flexi_logger/error_info/index.html#{code_lc}",
        msg = msg,
        code = error_code,
        code_lc = error_code.as_index(),
    );
    try_to_write(&s);
}

lazy_static::lazy_static! {
    pub(crate) static ref ERROR_CHANNEL: RwLock<ErrorChannel> = RwLock::new(ErrorChannel::default());
}

pub(crate) fn set_error_channel(channel: ErrorChannel) {
    match ERROR_CHANNEL.write() {
        Ok(mut guard) => {
            *guard = channel;
        }
        Err(e) => {
            eprint_err(ErrorCode::Poison, "Error channel cannot be set", &e);
        }
    }
}

fn try_to_write(s: &str) {
    match &*(ERROR_CHANNEL.read().unwrap()) {
        ErrorChannel::StdErr => {
            eprintln!("{s}");
        }
        ErrorChannel::StdOut => {
            println!("{s}");
        }
        ErrorChannel::File(path) => try_to_write_to_file(s, path).unwrap_or_else(|e| {
            eprintln!("{s}");
            eprintln!("Can't open error output file, caused by: {e}");
        }),
        ErrorChannel::DevNull => {}
    }
}

fn try_to_write_to_file(s: &str, path: &Path) -> Result<(), std::io::Error> {
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{s}")?;
    file.flush()
}

pub(crate) fn io_err(s: &'static str) -> std::io::Error {
    std::io::Error::new(std::io::ErrorKind::Other, s)
}

// Thread-local buffer
pub(crate) fn buffer_with<F>(f: F)
where
    F: FnOnce(&RefCell<Vec<u8>>),
{
    thread_local! {
        static BUFFER: RefCell<Vec<u8>> = RefCell::new(Vec::with_capacity(200));
    }
    BUFFER.with(f);
}

// Use the thread-local buffer for formatting before writing into the given writer
pub(crate) fn write_buffered(
    format_function: FormatFunction,
    now: &mut DeferredNow,
    record: &Record,
    w: &mut dyn Write,
    #[cfg(test)] o_validation_buffer: Option<&Arc<Mutex<Cursor<Vec<u8>>>>>,
) -> Result<(), std::io::Error> {
    let mut result: Result<(), std::io::Error> = Ok(());

    buffer_with(|tl_buf| match tl_buf.try_borrow_mut() {
        Ok(mut buffer) => {
            (format_function)(&mut *buffer, now, record)
                .unwrap_or_else(|e| eprint_err(ErrorCode::Format, "formatting failed", &e));
            buffer
                .write_all(b"\n")
                .unwrap_or_else(|e| eprint_err(ErrorCode::Write, "writing failed", &e));

            result = w.write_all(&buffer).map_err(|e| {
                eprint_err(ErrorCode::Write, "writing failed", &e);
                e
            });

            #[cfg(test)]
            if let Some(valbuf) = o_validation_buffer {
                valbuf.lock().unwrap().write_all(&buffer).ok();
            }
            buffer.clear();
        }
        Err(_e) => {
            // We arrive here in the rare cases of recursive logging
            // (e.g. log calls in Debug or Display implementations)
            // we print the inner calls, in chronological order, before finally the
            // outer most message is printed
            let mut tmp_buf = Vec::<u8>::with_capacity(200);
            (format_function)(&mut tmp_buf, now, record)
                .unwrap_or_else(|e| eprint_err(ErrorCode::Format, "formatting failed", &e));
            tmp_buf
                .write_all(b"\n")
                .unwrap_or_else(|e| eprint_err(ErrorCode::Write, "writing failed", &e));

            result = w.write_all(&tmp_buf).map_err(|e| {
                eprint_err(ErrorCode::Write, "writing failed", &e);
                e
            });

            #[cfg(test)]
            if let Some(valbuf) = o_validation_buffer {
                valbuf.lock().unwrap().write_all(&tmp_buf).ok();
            }
        }
    });
    result
}
