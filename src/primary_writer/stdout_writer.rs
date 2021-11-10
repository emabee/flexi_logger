#[cfg(feature = "async")]
use crate::util::{eprint_err, ERRCODE};
use crate::util::{io_err, write_buffered};
#[cfg(feature = "async")]
use crate::util::{ASYNC_FLUSH, ASYNC_SHUTDOWN};
use crate::{writers::LogWriter, DeferredNow, EffectiveWriteMode, FormatFunction, WriteMode};
#[cfg(test)]
use std::io::Cursor;

#[cfg(feature = "async")]
use crossbeam::{
    channel::{self, SendError, Sender},
    queue::ArrayQueue,
};
use log::Record;
use std::io::{BufWriter, Write};
#[cfg(any(feature = "async", test))]
use std::sync::Arc;
use std::sync::Mutex;
#[cfg(feature = "async")]
use std::thread::JoinHandle;

// `StdOutWriter` writes logs to stdout.
pub(crate) struct StdOutWriter {
    format: FormatFunction,
    writer: OutWriter,
    #[cfg(test)]
    validation_buffer: Arc<Mutex<Cursor<Vec<u8>>>>,
}
enum OutWriter {
    Unbuffered(std::io::Stdout),
    Buffered(Mutex<BufWriter<std::io::Stdout>>),
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
        _bufsize: usize,
        pool_capa: usize,
        msg_capa: usize,
        #[cfg(test)] validation_buffer: &Arc<Mutex<Cursor<Vec<u8>>>>,
    ) -> Self {
        let (sender, receiver) = channel::unbounded::<Vec<u8>>();
        let a_pool = Arc::new(ArrayQueue::new(pool_capa));
        let t_pool = Arc::clone(&a_pool);
        #[cfg(test)]
        let t_validation_buffer = Arc::clone(validation_buffer);

        let mo_thread_handle = Mutex::new(Some(
            std::thread::Builder::new()
                .name("flexi_logger-async_stdout".to_string())
                .spawn(move || {
                    let mut stdout = std::io::stdout();
                    loop {
                        match receiver.recv() {
                            Err(_) => break,
                            Ok(mut message) => {
                                match message.as_ref() {
                                    ASYNC_FLUSH => {
                                        stdout
                                            .flush()
                                            .unwrap_or_else(
                                                |e| eprint_err(ERRCODE::Flush, "flushing failed", &e)
                                            );
                                    }
                                    ASYNC_SHUTDOWN => {
                                        break;
                                    }
                                    _ => {
                                        stdout
                                            .write_all(&message)
                                            .unwrap_or_else(
                                                |e| eprint_err(ERRCODE::Write,"writing failed", &e)
                                            );
                                        #[cfg(test)]
                                        if let Ok(mut guard) = t_validation_buffer.lock() {
                                            (*guard).write_all(&message).ok();
                                        }
                                    }
                                }
                                if message.capacity() <= msg_capa {
                                    message.clear();
                                    t_pool.push(message).ok();
                                }
                            }
                        }
                    }
                })
                .unwrap(/* yes, let's panic if the thread can't be spawned */),
        ));
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

impl StdOutWriter {
    pub(crate) fn new(format: FormatFunction, write_mode: &WriteMode) -> Self {
        #[cfg(test)]
        let validation_buffer = Arc::new(Mutex::new(Cursor::new(Vec::<u8>::new())));

        let writer = match write_mode.inner() {
            EffectiveWriteMode::Direct => OutWriter::Unbuffered(std::io::stdout()),
            EffectiveWriteMode::BufferDontFlushWith(capacity) => OutWriter::Buffered(Mutex::new(
                BufWriter::with_capacity(capacity, std::io::stdout()),
            )),
            EffectiveWriteMode::BufferAndFlushWith(_, _) => {
                unreachable!("Sync OutWriter with own flushing is not implemented")
            }
            #[cfg(feature = "async")]
            EffectiveWriteMode::AsyncWith {
                bufsize,
                pool_capa,
                message_capa,
                flush_interval,
            } => {
                assert_eq!(
                    flush_interval,
                    std::time::Duration::from_secs(0),
                    "Async OutWriter with own flushing is not implemented"
                );
                OutWriter::Async(AsyncHandle::new(
                    bufsize,
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
impl LogWriter for StdOutWriter {
    #[inline]
    fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        match &self.writer {
            OutWriter::Unbuffered(stdout) => {
                let mut w = stdout.lock();

                write_buffered(
                    self.format,
                    now,
                    record,
                    &mut w,
                    #[cfg(test)]
                    Some(&self.validation_buffer),
                )
            }
            OutWriter::Buffered(mbuf_w) => {
                let mut w = mbuf_w.lock().map_err(|_e| io_err("Poison"))?;
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
            OutWriter::Async(handle) => {
                let mut buffer = handle.pop_buffer();
                (self.format)(&mut buffer, now, record)
                    .unwrap_or_else(|e| eprint_err(ERRCODE::Format, "formatting failed", &e));
                buffer
                    .write_all(b"\n")
                    .unwrap_or_else(|e| eprint_err(ERRCODE::Write, "writing failed", &e));
                handle.send(buffer).map_err(|_e| io_err("Send"))?;
                Ok(())
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
                let mut w = mbuf_w.lock().map_err(|_e| io_err("Poison"))?;
                w.flush()
            }
            #[cfg(feature = "async")]
            OutWriter::Async(handle) => {
                let mut buffer = handle.pop_buffer();
                buffer.extend(ASYNC_FLUSH);
                handle.send(buffer).ok();
                Ok(())
            }
        }
    }

    fn shutdown(&self) {
        #[cfg(feature = "async")]
        if let OutWriter::Async(handle) = &self.writer {
            let mut buffer = handle.pop_buffer();
            buffer.extend(ASYNC_SHUTDOWN);
            handle.send(buffer).ok();
            if let Ok(ref mut o_th) = handle.mo_thread_handle.lock() {
                o_th.take().and_then(|th| th.join().ok());
            }
        }
    }

    #[allow(unused_variables)]
    fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) {
        #[cfg(test)]
        {
            use std::io::BufRead;
            let write_cursor = self.validation_buffer.lock().unwrap();
            let mut reader = std::io::BufReader::new(Cursor::new(write_cursor.get_ref()));
            let mut buf = String::new();
            for tuple in expected {
                buf.clear();
                reader.read_line(&mut buf).unwrap();
                assert!(buf.contains(&tuple.0), "Did not find tuple.0 = {}", tuple.0);
                assert!(buf.contains(&tuple.1), "Did not find tuple.1 = {}", tuple.1);
                assert!(buf.contains(&tuple.2), "Did not find tuple.2 = {}", tuple.2);
            }
            buf.clear();
            reader.read_line(&mut buf).unwrap();
            assert!(
                buf.is_empty(),
                "Found more log lines than expected: {} ",
                buf
            );
        }
    }
}

#[cfg(test)]
mod test {
    use super::StdOutWriter;
    use crate::{opt_format, writers::LogWriter, DeferredNow, WriteMode};
    use log::Level::{Error, Info, Warn};

    #[test]
    fn test_with_validation() {
        let writer = StdOutWriter::new(opt_format, &WriteMode::Direct);
        let mut rb = log::Record::builder();
        rb.target("myApp")
            .file(Some("stdout_writer.rs"))
            .line(Some(222))
            .module_path(Some("stdout_writer::test::test_with_validation"));

        rb.level(Error)
            .args(format_args!("This is an error message"));
        writer.write(&mut DeferredNow::new(), &rb.build()).unwrap();

        rb.level(Warn).args(format_args!("This is a warning"));
        writer.write(&mut DeferredNow::new(), &rb.build()).unwrap();

        rb.level(Info).args(format_args!("This is an info message"));
        writer.write(&mut DeferredNow::new(), &rb.build()).unwrap();

        writer.validate_logs(&[
            ("ERROR", "stdout_writer.rs:222", "error"),
            ("WARN", "stdout_writer.rs:222", "warning"),
            ("INFO", "stdout_writer.rs:222", "info"),
        ]);
    }
}
