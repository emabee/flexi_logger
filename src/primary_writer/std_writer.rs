#[cfg(feature = "async")]
use {
    crate::util::{eprint_err, ErrorCode, ASYNC_FLUSH, ASYNC_SHUTDOWN},
    crossbeam_channel::{SendError, Sender},
    crossbeam_queue::ArrayQueue,
};

use {
    super::std_stream::StdStream,
    crate::{
        util::{io_err, write_buffered},
        writers::LogWriter,
        DeferredNow, EffectiveWriteMode, FormatFunction, WriteMode,
    },
    log::Record,
    std::io::{BufWriter, Write},
};

#[cfg(test)]
use std::io::Cursor;

#[cfg(any(feature = "async", test))]
use std::sync::Arc;
use std::sync::Mutex;
#[cfg(feature = "async")]
use std::thread::JoinHandle;

// `StdWriter` writes logs to stdout or stderr.
pub(crate) struct StdWriter {
    format: FormatFunction,
    writer: InnerStdWriter,
    #[cfg(test)]
    validation_buffer: Arc<Mutex<Cursor<Vec<u8>>>>,
}
enum InnerStdWriter {
    Unbuffered(StdStream),
    Buffered(Mutex<BufWriter<StdStream>>),
    #[cfg(feature = "async")]
    Async(AsyncHandle),
}
#[cfg(feature = "async")]
#[derive(Debug)]
struct AsyncHandle {
    sender: Sender<Vec<u8>>,
    mo_thread_handle: Mutex<Option<JoinHandle<()>>>,
    a_pool: Arc<ArrayQueue<Vec<u8>>>,
    msg_capa: usize,
}
#[cfg(feature = "async")]
impl AsyncHandle {
    fn new(
        stdstream: StdStream,
        pool_capa: usize,
        msg_capa: usize,
        #[cfg(test)] validation_buffer: &Arc<Mutex<Cursor<Vec<u8>>>>,
    ) -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded::<Vec<u8>>();
        let a_pool = Arc::new(ArrayQueue::new(pool_capa));

        let mo_thread_handle = crate::threads::start_async_stdwriter(
            stdstream,
            receiver,
            Arc::clone(&a_pool),
            msg_capa,
            #[cfg(test)]
            Arc::clone(validation_buffer),
        );

        AsyncHandle {
            sender,
            mo_thread_handle,
            a_pool,
            msg_capa,
        }
    }

    fn pop_buffer(&self) -> Vec<u8> {
        self.a_pool
            .pop()
            .unwrap_or_else(|| Vec::with_capacity(self.msg_capa))
    }

    fn send(&self, buffer: Vec<u8>) -> Result<(), SendError<Vec<u8>>> {
        self.sender.send(buffer)
    }
}

impl StdWriter {
    pub(crate) fn new(
        stdstream: StdStream,
        format: FormatFunction,
        write_mode: &WriteMode,
    ) -> Self {
        #[cfg(test)]
        let validation_buffer = Arc::new(Mutex::new(Cursor::new(Vec::<u8>::new())));

        let writer = match write_mode.inner() {
            EffectiveWriteMode::Direct => InnerStdWriter::Unbuffered(stdstream),
            EffectiveWriteMode::BufferDontFlushWith(capacity) => {
                InnerStdWriter::Buffered(Mutex::new(BufWriter::with_capacity(capacity, stdstream)))
            }
            EffectiveWriteMode::BufferAndFlushWith(_, _) => {
                unreachable!("Sync InnerStdWriter with own flushing is not implemented")
            }
            #[cfg(feature = "async")]
            EffectiveWriteMode::AsyncWith {
                pool_capa,
                message_capa,
                flush_interval,
            } => {
                assert_eq!(
                    flush_interval,
                    std::time::Duration::from_secs(0),
                    "Async InnerStdWriter with own flushing is not implemented"
                );
                InnerStdWriter::Async(AsyncHandle::new(
                    stdstream,
                    pool_capa,
                    message_capa,
                    #[cfg(test)]
                    &validation_buffer,
                ))
            }
        };
        Self {
            format,
            writer,
            #[cfg(test)]
            validation_buffer,
        }
    }
}
impl LogWriter for StdWriter {
    #[inline]
    fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        match &self.writer {
            InnerStdWriter::Unbuffered(stdstream) => {
                let mut w = stdstream.lock();
                write_buffered(
                    self.format,
                    now,
                    record,
                    &mut w,
                    #[cfg(test)]
                    Some(&self.validation_buffer),
                )
            }
            InnerStdWriter::Buffered(m_w) => {
                let mut w = m_w.lock().map_err(|_e| io_err("Poison"))?;
                write_buffered(
                    self.format,
                    now,
                    record,
                    &mut *w,
                    #[cfg(test)]
                    Some(&self.validation_buffer),
                )
            }
            #[cfg(feature = "async")]
            InnerStdWriter::Async(handle) => {
                let mut buffer = handle.pop_buffer();
                (self.format)(&mut buffer, now, record)
                    .unwrap_or_else(|e| eprint_err(ErrorCode::Format, "formatting failed", &e));
                buffer
                    .write_all(b"\n")
                    .unwrap_or_else(|e| eprint_err(ErrorCode::Write, "writing failed", &e));
                handle.send(buffer).map_err(|_e| io_err("Send"))?;
                Ok(())
            }
        }
    }

    #[inline]
    fn flush(&self) -> std::io::Result<()> {
        match &self.writer {
            InnerStdWriter::Unbuffered(stdstream) => {
                let mut w = stdstream.lock();
                w.flush()
            }
            InnerStdWriter::Buffered(m_w) => {
                let mut w = m_w.lock().map_err(|_e| io_err("Poison"))?;
                w.flush()
            }
            #[cfg(feature = "async")]
            InnerStdWriter::Async(handle) => {
                let mut buffer = handle.pop_buffer();
                buffer.extend(ASYNC_FLUSH);
                handle.send(buffer).ok();
                Ok(())
            }
        }
    }

    fn shutdown(&self) {
        #[cfg(feature = "async")]
        if let InnerStdWriter::Async(handle) = &self.writer {
            let mut buffer = handle.pop_buffer();
            buffer.extend(ASYNC_SHUTDOWN);
            handle.send(buffer).ok();
            if let Ok(ref mut o_th) = handle.mo_thread_handle.lock() {
                o_th.take().and_then(|th| th.join().ok());
            }
        }
    }

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

#[cfg(test)]
mod test {
    use super::{StdStream, StdWriter};
    use crate::{opt_format, writers::LogWriter, DeferredNow, WriteMode};
    use log::Level::{Error, Info, Warn};

    #[test]
    fn test_with_validation() {
        let writer = StdWriter::new(
            StdStream::Err(std::io::stderr()),
            opt_format,
            &WriteMode::Direct,
        );
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
