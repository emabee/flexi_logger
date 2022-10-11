use {
    crate::{primary_writer::PrimaryWriter, writers::LogWriter, FlexiLoggerError},
    std::{
        collections::HashMap,
        sync::{
            mpsc::{channel, Receiver, Sender},
            Arc,
        },
        thread::Builder as ThreadBuilder,
    },
};

#[cfg(feature = "async")]
use {
    crate::{
        primary_writer::std_stream::StdStream,
        util::{eprint_err, ErrorCode, ASYNC_FLUSH, ASYNC_SHUTDOWN},
    },
    crossbeam_channel::Receiver as CrossbeamReceiver,
    crossbeam_queue::ArrayQueue,
    std::{sync::Mutex, thread::JoinHandle},
};

// no clue why we get a warning if this allow is omitted; if we omit the use, we get an error
#[allow(unused_imports)]
#[cfg(feature = "async")]
use std::io::Write;

#[cfg(feature = "async")]
const ASYNC_STD_WRITER: &str = "flexi_logger-async_std_writer";
const FLUSHER: &str = "flexi_logger-flusher";

// Used in Logger
pub(crate) fn start_flusher_thread(
    primary_writer: Arc<PrimaryWriter>,
    other_writers: Arc<HashMap<String, Box<dyn LogWriter>>>,
    flush_interval: std::time::Duration,
) -> Result<(), FlexiLoggerError> {
    let builder = ThreadBuilder::new().name(FLUSHER.to_string());
    #[cfg(not(feature = "dont_minimize_extra_stacks"))]
    let builder = builder.stack_size(128);

    builder.spawn(move || {
        let (_sender, receiver): (Sender<()>, Receiver<()>) = channel();
        loop {
            receiver.recv_timeout(flush_interval).ok();
            primary_writer.flush().ok();
            for w in other_writers.values() {
                w.flush().ok();
            }
        }
    })?;
    Ok(())
}

#[cfg(feature = "async")]
pub(crate) fn start_async_stdwriter(
    mut std_stream: StdStream,
    receiver: CrossbeamReceiver<std::vec::Vec<u8>>,
    t_pool: Arc<ArrayQueue<Vec<u8>>>,
    msg_capa: usize,
    #[cfg(test)] t_validation_buffer: Arc<Mutex<std::io::Cursor<Vec<u8>>>>,
) -> Mutex<Option<JoinHandle<()>>> {
    Mutex::new(Some(
        ThreadBuilder::new()
            .name(
                ASYNC_STD_WRITER.to_string()
            )
            .spawn(move || {
                loop {
                    match receiver.recv() {
                        Err(_) => break,
                        Ok(mut message) => {
                            match message.as_ref() {
                                ASYNC_FLUSH => {
                                    std_stream
                                        .deref_mut()
                                        .flush()
                                        .unwrap_or_else(
                                            |e| eprint_err(ErrorCode::Flush, "flushing failed", &e)
                                        );
                                }
                                ASYNC_SHUTDOWN => {
                                    break;
                                }
                                _ => {
                                    std_stream
                                        .deref_mut()
                                        .write_all(&message)
                                        .unwrap_or_else(
                                            |e| eprint_err(ErrorCode::Write,"writing failed", &e)
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
    ))
}
