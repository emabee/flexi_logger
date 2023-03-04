use {
    crate::{writers::LogWriter, DeferredNow, FormatFunction},
    log::Record,
};

#[cfg(test)]
use std::io::Cursor;

use std::cell::RefCell;
#[cfg(test)]
use std::{
    io::Write,
    sync::{Arc, Mutex},
};

use crate::util::{eprint_err, ErrorCode};

// `TestWriter` writes logs using println!
pub(crate) struct TestWriter {
    format: FormatFunction,
    stdout: bool,
    #[cfg(test)]
    validation_buffer: Arc<Mutex<Cursor<Vec<u8>>>>,
}

impl TestWriter {
    pub(crate) fn new(stdout: bool, format: FormatFunction) -> Self {
        #[cfg(test)]
        let validation_buffer = Arc::new(Mutex::new(Cursor::new(Vec::<u8>::new())));

        Self {
            format,
            stdout,
            #[cfg(test)]
            validation_buffer,
        }
    }
}
impl LogWriter for TestWriter {
    #[inline]
    fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        buffer_with(|tl_buf| match tl_buf.try_borrow_mut() {
            Ok(mut buffer) => {
                (self.format)(&mut *buffer, now, record)
                    .unwrap_or_else(|e| eprint_err(ErrorCode::Format, "formatting failed", &e));
                if self.stdout {
                    println!("{}", String::from_utf8_lossy(&buffer));
                } else {
                    eprintln!("{}", String::from_utf8_lossy(&buffer));
                }

                #[cfg(test)]
                {
                    let mut cursor = self.validation_buffer.lock().unwrap();
                    cursor.write_all(&buffer).ok();
                    cursor.write_all(&[b'\n']).ok();
                }

                buffer.clear();
            }
            Err(_e) => {
                // We arrive here in the rare cases of recursive logging
                // (e.g. log calls in Debug or Display implementations)
                // we print the inner calls, in chronological order, before finally the
                // outer most message is printed
                let mut tmp_buf = Vec::<u8>::with_capacity(200);
                (self.format)(&mut tmp_buf, now, record)
                    .unwrap_or_else(|e| eprint_err(ErrorCode::Format, "formatting failed", &e));
                if self.stdout {
                    println!("{}", String::from_utf8_lossy(&tmp_buf));
                } else {
                    eprintln!("{}", String::from_utf8_lossy(&tmp_buf));
                }

                #[cfg(test)]
                {
                    let mut cursor = self.validation_buffer.lock().unwrap();
                    cursor.write_all(&tmp_buf).ok();
                    cursor.write_all(&[b'\n']).ok();
                }
            }
        });
        Ok(())
    }

    #[inline]
    fn flush(&self) -> std::io::Result<()> {
        Ok(())
    }

    fn shutdown(&self) {}

    #[cfg(not(test))]
    fn validate_logs(&self, _expected: &[(&'static str, &'static str, &'static str)]) {}
    #[cfg(test)]
    fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) {
        {
            use std::io::BufRead;
            let write_cursor = self.validation_buffer.lock().unwrap();
            let mut reader = std::io::BufReader::new(Cursor::new(write_cursor.get_ref()));
            let mut buf = String::new();
            for tuple in expected {
                buf.clear();
                reader.read_line(&mut buf).unwrap();
                assert!(buf.contains(tuple.0), "Did not find tuple.0 = {}", tuple.0);
                assert!(buf.contains(tuple.1), "Did not find tuple.1 = {}", tuple.1);
                assert!(buf.contains(tuple.2), "Did not find tuple.2 = {}", tuple.2);
            }
            buf.clear();
            reader.read_line(&mut buf).unwrap();
            assert!(buf.is_empty(), "Found more log lines than expected: {buf} ",);
        }
    }
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

#[cfg(test)]
mod test {
    use super::TestWriter;
    use crate::{opt_format, writers::LogWriter, DeferredNow};
    use log::Level::{Error, Info, Warn};

    #[test]
    fn test_with_validation() {
        let writer = TestWriter::new(true, opt_format);
        let mut rb = log::Record::builder();
        rb.target("myApp")
            .file(Some("std_writer.rs"))
            .line(Some(222))
            .module_path(Some("std_writer::test::test_with_validation"));

        rb.level(Error)
            .args(format_args!("This is an error message"));
        writer.write(&mut DeferredNow::new(), &rb.build()).unwrap();

        rb.level(Warn).args(format_args!("This is a warning"));
        writer.write(&mut DeferredNow::new(), &rb.build()).unwrap();

        rb.level(Info).args(format_args!("This is an info message"));
        writer.write(&mut DeferredNow::new(), &rb.build()).unwrap();

        writer.validate_logs(&[
            ("ERROR", "std_writer.rs:222", "error"),
            ("WARN", "std_writer.rs:222", "warning"),
            ("INFO", "std_writer.rs:222", "info"),
        ]);
    }
}
