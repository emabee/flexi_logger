use super::{builder::FileLogWriterBuilder, state::State};
#[cfg(feature = "async")]
use crate::util::eprint_msg;
use crate::util::{buffer_with, eprint_err, io_err, ERRCODE};
#[cfg(feature = "async")]
use crate::util::{ASYNC_FLUSH, ASYNC_SHUTDOWN};
use crate::DeferredNow;
use crate::FlexiLoggerError;
use crate::FormatFunction;
#[cfg(feature = "async")]
use crossbeam::{
    channel::{self, Sender},
    queue::ArrayQueue,
};
use log::Record;
use std::io::Write;
use std::sync::{mpsc, Arc, Mutex};
#[cfg(feature = "async")]
use std::thread::JoinHandle;

#[derive(Debug)]
pub(super) enum StateHandle {
    Sync(SyncHandle),
    #[cfg(feature = "async")]
    Async(AsyncHandle),
}

pub(super) struct SyncHandle {
    am_state: Arc<Mutex<State>>,
    format_function: FormatFunction,
    line_ending: &'static [u8],
}
impl SyncHandle {
    fn new(state: State, format_function: FormatFunction) -> Self {
        let line_ending = state.config().line_ending;
        let flush_interval = state.config().write_mode.get_flush_interval();
        let am_state = Arc::new(Mutex::new(state));
        // Create a flusher if needed
        if flush_interval != std::time::Duration::from_secs(0) {
            let t_am_state = Arc::clone(&am_state);
            std::thread::Builder::new()
                .name("flexi_logger-flusher".to_string())
                .stack_size(128)
                .spawn(move || {
                    let (_sender, receiver): (
                        mpsc::Sender<()>,
                        mpsc::Receiver<()>,
                    ) = mpsc::channel();
                    loop {
                        receiver.recv_timeout(flush_interval).ok();
                        (*t_am_state).lock().map_or_else(
                            |_e| (),
                            |mut state| {
                                state.flush().ok();
                            },
                        );
                    }
                })
                .unwrap(/* yes, let's panic if the thread can't be spawned */);
        }
        Self {
            am_state,
            format_function,
            line_ending,
        }
    }
}
impl std::fmt::Debug for SyncHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        f.debug_struct("SyncHandle")
            .field("am_state", &self.am_state)
            .field("format", &"<..>")
            .field("line_ending", &self.line_ending)
            .finish()
    }
}

#[cfg(feature = "async")]
pub(super) struct AsyncHandle {
    am_state: Arc<Mutex<State>>,
    sender: Sender<Vec<u8>>,
    mo_thread_handle: Mutex<Option<JoinHandle<()>>>,
    a_pool: Arc<ArrayQueue<Vec<u8>>>,
    message_capa: usize,
    format_function: FormatFunction,
    line_ending: &'static [u8],
}
#[cfg(feature = "async")]
impl AsyncHandle {
    fn new(
        pool_capa: usize,
        message_capa: usize,
        state: State,
        format_function: FormatFunction,
    ) -> Self {
        let flush_interval = state.config().write_mode.get_flush_interval();
        let line_ending = state.config().line_ending;
        let am_state = Arc::new(Mutex::new(state));
        let (async_sender, receiver) = channel::unbounded::<Vec<u8>>();
        let a_pool = Arc::new(ArrayQueue::new(pool_capa));

        let t_state = Arc::clone(&am_state);
        let t_pool = Arc::clone(&a_pool);

        let mo_thread_handle = Mutex::new(Some(
            std::thread::Builder::new()
                .name("flexi_logger-async_file_log_writer".to_string())
                .spawn(move || loop {
                    match receiver.recv() {
                        Err(_) => break,
                        Ok(mut message) => {
                            let mut state = t_state.lock().unwrap(/* ok */);
                            match message.as_ref() {
                                ASYNC_FLUSH => {
                                    state.flush().unwrap_or_else(|e| {
                                        eprint_err(ERRCODE::Flush, "flushing failed", &e);
                                    });
                                }
                                ASYNC_SHUTDOWN => {
                                    state.shutdown();
                                    break;
                                }
                                _ => {
                                    state.write_buffer(&message).unwrap_or_else(|e| {
                                        eprint_err(ERRCODE::Write, "writing failed", &e);
                                    });
                                }
                            }
                            if message.capacity() <= message_capa {
                                message.clear();
                                t_pool.push(message).ok();
                            }
                        }
                    }
                })
                .expect("Couldn't spawn flexi_logger-async_file_log_writer"),
        ));

        if flush_interval != std::time::Duration::from_secs(0) {
            let cloned_async_sender = async_sender.clone();
            std::thread::Builder::new()
                .name("flexi_logger-flusher".to_string())
                .stack_size(128)
                .spawn(move || {
                    let (_sender, receiver): (
                        mpsc::Sender<()>,
                        mpsc::Receiver<()>,
                    ) = mpsc::channel();
                    loop {
                        if let Err(mpsc::RecvTimeoutError::Disconnected) =
                            receiver.recv_timeout(flush_interval)
                        {
                            eprint_msg(ERRCODE::Flush, "Flushing unexpectedly stopped working");
                            break;
                        }

                        cloned_async_sender.send(ASYNC_FLUSH.to_vec()).ok();
                    }
                })
                .unwrap(/* yes, let's panic if the thread can't be spawned */);
        }

        Self {
            am_state,
            sender: async_sender,
            mo_thread_handle,
            a_pool,
            message_capa,
            format_function,
            line_ending,
        }
    }

    fn write(&self, now: &mut DeferredNow, record: &Record) -> Result<(), std::io::Error> {
        let mut buffer = self.pop_buffer();
        (self.format_function)(&mut buffer, now, record).map_err(|e| {
            eprint_err(ERRCODE::Format, "formatting failed", &e);
            e
        })?;
        buffer.write_all(self.line_ending).map_err(|e| {
            eprint_err(ERRCODE::Write, "writing failed", &e);
            e
        })?;
        self.sender.send(buffer).map_err(|_e| io_err("Send"))
    }
    fn pop_buffer(&self) -> Vec<u8> {
        self.a_pool
            .pop()
            .unwrap_or_else(|| Vec::with_capacity(self.message_capa))
    }
}
#[cfg(feature = "async")]
impl std::fmt::Debug for AsyncHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        f.debug_struct("AsyncHandle")
            .field("am_state", &self.am_state)
            .field("sender", &self.sender)
            .field("mo_thread_handle", &self.mo_thread_handle)
            .field("a_pool", &self.a_pool)
            .field("message_capa", &self.message_capa)
            .field("format", &"<..>")
            .field("line_ending", &self.line_ending)
            .finish()
    }
}

impl StateHandle {
    // produce a StateHandle::Sync, optionally with an own flusher-thread
    pub(super) fn new_sync(state: State, format_function: FormatFunction) -> StateHandle {
        StateHandle::Sync(SyncHandle::new(state, format_function))
    }

    // produce a StateHandle::Async with its writer-thread, and optionally an own flusher-thread
    #[cfg(feature = "async")]
    pub(super) fn new_async(
        pool_capa: usize,
        message_capa: usize,
        state: State,
        format_function: FormatFunction,
    ) -> Self {
        Self::Async(AsyncHandle::new(
            pool_capa,
            message_capa,
            state,
            format_function,
        ))
    }

    pub(super) fn current_filename(&self) -> std::path::PathBuf {
        match self {
            StateHandle::Sync(handle) => handle.am_state.lock(),
            #[cfg(feature = "async")]
            StateHandle::Async(handle) => handle.am_state.lock(),
        }
        .expect("state_handle.am_state is poisoned")
        .current_filename()
    }

    pub(super) fn format_function(&self) -> FormatFunction {
        match self {
            StateHandle::Sync(handle) => handle.format_function,
            #[cfg(feature = "async")]
            StateHandle::Async(handle) => handle.format_function,
        }
    }

    pub(super) fn plain_write(&self, buffer: &[u8]) -> std::result::Result<usize, std::io::Error> {
        match self {
            StateHandle::Sync(handle) => {
                let mut state_guard = handle.am_state.lock().map_err(|_e| io_err("Poison"))?;
                let state = &mut *state_guard;
                state.write_buffer(buffer).map(|_| buffer.len())
            }
            #[cfg(feature = "async")]
            StateHandle::Async(handle) => {
                handle
                    .sender
                    .send(buffer.to_owned())
                    .map_err(|_e| io_err("Send"))?;
                Ok(buffer.len())
            }
        }
    }

    #[allow(clippy::unnecessary_wraps)]
    #[inline]
    pub(super) fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        match &self {
            StateHandle::Sync(handle) => {
                buffer_with(|tl_buf| match tl_buf.try_borrow_mut() {
                    Ok(mut buffer) => {
                        (handle.format_function)(&mut *buffer, now, record).unwrap_or_else(|e| {
                            eprint_err(ERRCODE::Format, "formatting failed", &e);
                        });
                        buffer
                            .write_all(handle.line_ending)
                            .unwrap_or_else(|e| eprint_err(ERRCODE::Write, "writing failed", &e));
                        (&mut *handle
                            .am_state
                            .lock()
                            .expect("state_handle.am_state is poisoned"))
                            .write_buffer(&*buffer)
                            .unwrap_or_else(|e| eprint_err(ERRCODE::Write, "writing failed", &e));
                        buffer.clear();
                    }
                    Err(_e) => {
                        // We arrive here in the rare cases of recursive logging
                        // (e.g. log calls in Debug or Display implementations)
                        // we print the inner calls, in chronological order, before finally the
                        // outer most message is printed
                        let mut tmp_buf = Vec::<u8>::with_capacity(200);
                        (handle.format_function)(&mut tmp_buf, now, record).unwrap_or_else(|e| {
                            eprint_err(ERRCODE::Format, "formatting failed", &e);
                        });
                        let mut state_guard = handle
                            .am_state
                            .lock()
                            .expect("state_handle.am_state is poisoned");
                        let state = &mut *state_guard;
                        tmp_buf
                            .write_all(state.config().line_ending)
                            .unwrap_or_else(|e| eprint_err(ERRCODE::Write, "writing failed", &e));
                        state
                            .write_buffer(&tmp_buf)
                            .unwrap_or_else(|e| eprint_err(ERRCODE::Write, "writing failed", &e));
                    }
                });
            }
            #[cfg(feature = "async")]
            StateHandle::Async(handle) => handle.write(now, record)?,
        }
        Ok(())
    }

    #[inline]
    pub(super) fn flush(&self) -> std::io::Result<()> {
        match &self {
            StateHandle::Sync(handle) => {
                if let Ok(ref mut state) = handle.am_state.lock() {
                    state.flush()?;
                }
            }
            #[cfg(feature = "async")]
            StateHandle::Async(handle) => {
                let mut buffer = handle.pop_buffer();
                buffer.extend(ASYNC_FLUSH);
                handle.sender.send(buffer).ok();
            }
        }
        Ok(())
    }

    // Replaces parts of the configuration of the file log writer.
    pub(super) fn reset(&self, flwb: &FileLogWriterBuilder) -> Result<(), FlexiLoggerError> {
        let mut state = match self {
            StateHandle::Sync(handle) => handle.am_state.lock(),
            #[cfg(feature = "async")]
            StateHandle::Async(handle) => handle.am_state.lock(),
        }
        .map_err(|_| FlexiLoggerError::Poison)?;
        flwb.assert_write_mode((*state).config().write_mode)?;
        *state = flwb.try_build_state()?;
        Ok(())
    }

    #[doc(hidden)]
    pub(super) fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) {
        match self {
            StateHandle::Sync(handle) => handle.am_state.lock(),
            #[cfg(feature = "async")]
            StateHandle::Async(handle) => handle.am_state.lock(),
        }
        .map(|mut state| state.validate_logs(expected))
        .ok();
    }

    pub(super) fn shutdown(&self) {
        match &self {
            StateHandle::Sync(handle) => {
                // do nothing in case of poison errors
                if let Ok(ref mut state) = handle.am_state.lock() {
                    state.shutdown();
                }
            }
            #[cfg(feature = "async")]
            StateHandle::Async(handle) => {
                let mut buffer = handle.pop_buffer();
                buffer.extend(ASYNC_SHUTDOWN);
                handle.sender.send(buffer).ok();
                if let Ok(ref mut o_th) = handle.mo_thread_handle.lock() {
                    o_th.take().and_then(|th| th.join().ok());
                }
            }
        }
    }
}
