use super::{builder::FileLogWriterBuilder, config::FileLogWriterConfig, state::State};
#[cfg(feature = "async")]
use crate::util::{ASYNC_FLUSH, ASYNC_SHUTDOWN};
use crate::{
    util::{buffer_with, eprint_err, io_err, ErrorCode},
    LogfileSelector,
};
use crate::{DeferredNow, FlexiLoggerError, FormatFunction};
use log::Record;
#[cfg(feature = "async")]
use std::thread::JoinHandle;
use std::{
    io::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
};
#[cfg(feature = "async")]
use {crossbeam_channel::Sender, crossbeam_queue::ArrayQueue};

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

        if flush_interval.as_secs() != 0 || flush_interval.subsec_nanos() != 0 {
            super::state::start_sync_flusher(Arc::clone(&am_state), flush_interval);
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
            .finish_non_exhaustive()
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
        let a_pool = Arc::new(ArrayQueue::new(pool_capa));

        let (sender, mo_thread_handle) = super::state::start_async_fs_writer(
            Arc::clone(&am_state),
            message_capa,
            Arc::clone(&a_pool),
        );

        if flush_interval != std::time::Duration::from_secs(0) {
            super::state::start_async_fs_flusher(sender.clone(), flush_interval);
        }

        Self {
            am_state,
            sender,
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
            eprint_err(ErrorCode::Format, "formatting failed", &e);
            e
        })?;
        buffer.write_all(self.line_ending).map_err(|e| {
            eprint_err(ErrorCode::Write, "writing failed", &e);
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
            .finish_non_exhaustive()
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
                state.write_buffer(buffer).map(|()| buffer.len())
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
                            eprint_err(ErrorCode::Format, "formatting failed", &e);
                        });
                        buffer
                            .write_all(handle.line_ending)
                            .unwrap_or_else(|e| eprint_err(ErrorCode::Write, "writing failed", &e));
                        handle
                            .am_state
                            .lock()
                            .expect("state_handle.am_state is poisoned")
                            .write_buffer(&buffer)
                            .unwrap_or_else(|e| eprint_err(ErrorCode::Write, "writing failed", &e));
                        buffer.clear();
                    }
                    Err(_e) => {
                        // We arrive here in the rare cases of recursive logging
                        // (e.g. log calls in Debug or Display implementations)
                        // we print the inner calls, in chronological order, before finally the
                        // outer most message is printed
                        let mut tmp_buf = Vec::<u8>::with_capacity(200);
                        (handle.format_function)(&mut tmp_buf, now, record).unwrap_or_else(|e| {
                            eprint_err(ErrorCode::Format, "formatting failed", &e);
                        });
                        let mut state_guard = handle
                            .am_state
                            .lock()
                            .expect("state_handle.am_state is poisoned");
                        let state = &mut *state_guard;
                        tmp_buf
                            .write_all(state.config().line_ending)
                            .unwrap_or_else(|e| eprint_err(ErrorCode::Write, "writing failed", &e));
                        state
                            .write_buffer(&tmp_buf)
                            .unwrap_or_else(|e| eprint_err(ErrorCode::Write, "writing failed", &e));
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

    pub(super) fn reopen_outputfile(&self) -> Result<(), FlexiLoggerError> {
        let mut state = match self {
            StateHandle::Sync(handle) => handle.am_state.lock(),
            #[cfg(feature = "async")]
            StateHandle::Async(handle) => handle.am_state.lock(),
        }
        .map_err(|_| FlexiLoggerError::Poison)?;
        Ok(state.reopen_outputfile()?)
    }

    pub(super) fn rotate(&self) -> Result<(), FlexiLoggerError> {
        let mut state = match self {
            StateHandle::Sync(handle) => handle.am_state.lock(),
            #[cfg(feature = "async")]
            StateHandle::Async(handle) => handle.am_state.lock(),
        }
        .map_err(|_| FlexiLoggerError::Poison)?;
        state.mount_next_linewriter_if_necessary(true)
    }

    pub(crate) fn config(&self) -> Result<FileLogWriterConfig, FlexiLoggerError> {
        let state = match self {
            StateHandle::Sync(handle) => handle.am_state.lock(),
            #[cfg(feature = "async")]
            StateHandle::Async(handle) => handle.am_state.lock(),
        }
        .map_err(|_| FlexiLoggerError::Poison)?;

        Ok(state.config().clone())
    }

    pub(super) fn existing_log_files(
        &self,
        selector: &LogfileSelector,
    ) -> Result<Vec<PathBuf>, FlexiLoggerError> {
        let state = match self {
            StateHandle::Sync(handle) => handle.am_state.lock(),
            #[cfg(feature = "async")]
            StateHandle::Async(handle) => handle.am_state.lock(),
        }
        .map_err(|_| FlexiLoggerError::Poison)?;

        Ok(state.existing_log_files(selector))
    }

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
