use crate::deferred_now::DeferredNow;
#[cfg(feature = "async")]
use crate::util::write_err;
use crate::util::{poison_err, write_buffered};
#[cfg(feature = "async")]
use crate::util::{ASYNC_FLUSH, ASYNC_SHUTDOWN, ERR_FLUSHING, ERR_FORMATTING, ERR_WRITING};
use crate::writers::{FlWriteMode, LogWriter};
use crate::FormatFunction;

#[cfg(feature = "async")]
use crossbeam::{
    channel::{self, SendError, Sender},
    queue::ArrayQueue,
};
use log::Record;
use std::io::{BufWriter, Write};
#[cfg(feature = "async")]
use std::sync::Arc;
use std::sync::Mutex;
#[cfg(feature = "async")]
use std::thread::JoinHandle;

// `StdErrWriter` writes logs to stderr.
pub(crate) struct StdErrWriter {
    format: FormatFunction,
    writer: ErrWriter,
}
enum ErrWriter {
    Unbuffered(std::io::Stderr),
    Buffered(Mutex<BufWriter<std::io::Stderr>>),
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
    fn new(_bufsize: usize, pool_capa: usize, msg_capa: usize) -> Self {
        let (sender, receiver) = channel::unbounded::<Vec<u8>>();
        let a_pool = Arc::new(ArrayQueue::new(pool_capa));
        let t_pool = Arc::clone(&a_pool);

        let mo_thread_handle = Mutex::new(Some(
            std::thread::Builder::new()
                .name("flexi_logger-async_stderr".to_string())
                .spawn(move || {
                    let mut stderr = std::io::stderr();
                    loop {
                        match receiver.recv() {
                            Err(_) => break,
                            Ok(mut message) => {
                                match message.as_ref() {
                                    ASYNC_FLUSH => {
                                        stderr
                                            .flush()
                                            .unwrap_or_else(|e| write_err(ERR_FLUSHING, &e));
                                    }
                                    ASYNC_SHUTDOWN => {
                                        break;
                                    }
                                    _ => {
                                        stderr
                                            .write_all(&message)
                                            .unwrap_or_else(|e| write_err(ERR_WRITING, &e));
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
                .unwrap(),
        )); // yes, let's panic if the thread can't be spawned
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

impl StdErrWriter {
    pub(crate) fn new(format: FormatFunction, flwritemode: &FlWriteMode) -> Self {
        match flwritemode {
            FlWriteMode::DontBuffer => Self {
                format,
                writer: ErrWriter::Unbuffered(std::io::stderr()),
            },
            FlWriteMode::Buffer(capacity) => Self {
                format,
                writer: ErrWriter::Buffered(Mutex::new(BufWriter::with_capacity(
                    *capacity,
                    std::io::stderr(),
                ))),
            },
            #[cfg(feature = "async")]
            FlWriteMode::BufferAsync(bufsize, pool_capa, msg_capa) => Self {
                format,
                writer: ErrWriter::Async(AsyncHandle::new(*bufsize, *pool_capa, *msg_capa)),
            },
        }
    }
}
impl LogWriter for StdErrWriter {
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
            #[cfg(feature = "async")]
            ErrWriter::Async(handle) => {
                let mut buffer = handle.pop_buffer();
                (self.format)(&mut buffer, now, record)
                    .unwrap_or_else(|e| write_err(ERR_FORMATTING, &e));
                buffer
                    .write_all(b"\n")
                    .unwrap_or_else(|e| write_err(ERR_WRITING, &e));
                handle.send(buffer).unwrap();
                Ok(())
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
            #[cfg(feature = "async")]
            ErrWriter::Async(handle) => {
                let mut buffer = handle.pop_buffer();
                buffer.extend(ASYNC_FLUSH);
                handle.send(buffer).ok();
                Ok(())
            }
        }
    }

    fn shutdown(&self) {
        #[cfg(feature = "async")]
        if let ErrWriter::Async(handle) = &self.writer {
            let mut buffer = handle.pop_buffer();
            buffer.extend(ASYNC_SHUTDOWN);
            handle.send(buffer).unwrap();
            if let Ok(ref mut o_th) = handle.mo_thread_handle.lock() {
                o_th.take().and_then(|th| th.join().ok());
            }
        }
    }
}
