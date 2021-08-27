use crate::{deferred_now::DeferredNow, FormatFunction};
use log::Record;
use std::cell::RefCell;
use std::io::Write;

#[cfg(feature = "async")]
pub(crate) const ASYNC_FLUSH: &[u8] = b"F";
#[cfg(feature = "async")]
pub(crate) const ASYNC_SHUTDOWN: &[u8] = b"S";

#[cfg(feature = "async")]
pub(crate) const ERR_FLUSHING: &str = "[flexi_logger] flushing failed ";
pub(crate) const ERR_WRITING: &str = "[flexi_logger] writing failed ";
pub(crate) const ERR_FORMATTING: &str = "[flexi_logger] formatting failed ";

pub(crate) fn write_err(msg: &str, err: &std::io::Error) {
    eprintln!("[flexi_logger] {} with {}", msg, err);
}

pub(crate) fn poison_err(s: &'static str, _e: &dyn std::error::Error) -> std::io::Error {
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
