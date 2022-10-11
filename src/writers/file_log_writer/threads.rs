use {
    super::State,
    crate::{
        writers::file_log_writer::remove_or_compress_too_old_logfiles_impl, Cleanup, FileSpec,
    },
    std::{
        sync::{
            mpsc::Sender,
            {Arc, Mutex},
        },
        thread::{Builder as ThreadBuilder, JoinHandle},
    },
};

#[cfg(feature = "async")]
use {
    crate::util::{eprint_err, eprint_msg, ErrorCode, ASYNC_FLUSH, ASYNC_SHUTDOWN},
    crossbeam_channel::{self, Sender as CrossbeamSender},
    crossbeam_queue::ArrayQueue,
};

const CLEANER: &str = "flexi_logger-fs-cleanup";
#[cfg(feature = "async")]
const ASYNC_WRITER: &str = "flexi_logger-fs-async_writer";
#[cfg(feature = "async")]
const ASYNC_FLUSHER: &str = "flexi_logger-fs-async_flusher";

pub(crate) enum MessageToCleanupThread {
    Act,
    Die,
}
pub(crate) fn start_cleanup_thread(
    cleanup: Cleanup,
    file_spec: FileSpec,
) -> Result<(Sender<MessageToCleanupThread>, JoinHandle<()>), std::io::Error> {
    let (sender, receiver) = std::sync::mpsc::channel();
    let builder = ThreadBuilder::new().name(CLEANER.to_string());
    #[cfg(not(feature = "dont_minimize_extra_stacks"))]
    let builder = builder.stack_size(512 * 1024);
    Ok((
        sender,
        builder.spawn(move || {
            while let Ok(MessageToCleanupThread::Act) = receiver.recv() {
                remove_or_compress_too_old_logfiles_impl(&cleanup, &file_spec).ok();
            }
        })?,
    ))
}

pub(super) fn start_sync_flusher(am_state: Arc<Mutex<State>>, flush_interval: std::time::Duration) {
    let builder = std::thread::Builder::new().name("flexi_logger-flusher".to_string());
    #[cfg(not(feature = "dont_minimize_extra_stacks"))]
    let builder = builder.stack_size(128);
    builder.spawn(move || {
        let (_tx, rx) = std::sync::mpsc::channel::<()>();
            loop {
                rx.recv_timeout(flush_interval).ok();
                (*am_state).lock().map_or_else(
                    |_e| (),
                    |mut state| {
                        state.flush().ok();
                    },
                );
            }
        })
        .unwrap(/* yes, let's panic if the thread can't be spawned */);
}

#[cfg(feature = "async")]
pub(super) fn start_async_fs_writer(
    am_state: Arc<Mutex<State>>,
    message_capa: usize,
    a_pool: Arc<ArrayQueue<Vec<u8>>>,
) -> (CrossbeamSender<Vec<u8>>, Mutex<Option<JoinHandle<()>>>) {
    let (sender, receiver) = crossbeam_channel::unbounded::<Vec<u8>>();
    (
        sender,
        Mutex::new(Some(
            std::thread::Builder::new()
                .name(ASYNC_WRITER.to_string())
                .spawn(move || loop {
                    match receiver.recv() {
                        Err(_) => break,
                        Ok(mut message) => {
                            let mut state = am_state.lock().unwrap(/* ok */);
                            match message.as_ref() {
                                ASYNC_FLUSH => {
                                    state.flush().unwrap_or_else(|e| {
                                        eprint_err(ErrorCode::Flush, "flushing failed", &e);
                                    });
                                }
                                ASYNC_SHUTDOWN => {
                                    state.shutdown();
                                    break;
                                }
                                _ => {
                                    state.write_buffer(&message).unwrap_or_else(|e| {
                                        eprint_err(ErrorCode::Write, "writing failed", &e);
                                    });
                                }
                            }
                            if message.capacity() <= message_capa {
                                message.clear();
                                a_pool.push(message).ok();
                            }
                        }
                    }
                })
                .expect("Couldn't spawn flexi_logger-async_file_log_writer"),
        )),
    )
}

#[cfg(feature = "async")]
pub(super) fn start_async_fs_flusher(
    async_writer: CrossbeamSender<Vec<u8>>,
    flush_interval: std::time::Duration,
) {
    let builder = std::thread::Builder::new().name(ASYNC_FLUSHER.to_string());
    #[cfg(not(feature = "dont_minimize_extra_stacks"))]
    let builder = builder.stack_size(128);
    builder.spawn(move || {
            let (_tx, rx) = std::sync::mpsc::channel::<()>();
            loop {
                if let Err(std::sync::mpsc::RecvTimeoutError::Disconnected) =
                    rx.recv_timeout(flush_interval)
                {
                    eprint_msg(ErrorCode::Flush, "Flushing unexpectedly stopped working");
                    break;
                }

                async_writer.send(ASYNC_FLUSH.to_vec()).ok();
            }
        })
        .unwrap(/* yes, let's panic if the thread can't be spawned */);
}
