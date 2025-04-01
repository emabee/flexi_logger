use log::Record;

use crate::{
    formats::FormatFunction,
    util::{eprint_err, ErrorCode},
    writers::LogWriter,
    DeferredNow, FlexiLoggerError,
};
use std::{
    collections::VecDeque,
    sync::{Arc, Mutex},
    time::Instant,
};

/// Allows logging to a memory buffer with limited size.
pub struct BufferWriter {
    state: Arc<Mutex<State>>,
}
struct State {
    buffer: VecDeque<String>,
    size: usize,
    last_update: Instant,
    max_size: usize,
    format: FormatFunction,
}

/// Allows getting the current content of the memory buffer.
#[derive(Clone)]
pub struct Snapshot {
    /// The latest snapshot of the memory buffer.
    pub text: String,
    last_update: Instant,
}
impl Snapshot {
    /// Constructor.
    #[must_use]
    pub fn new() -> Self {
        Self {
            text: String::new(),
            last_update: Instant::now(),
        }
    }
}
impl Default for Snapshot {
    fn default() -> Self {
        Self::new()
    }
}

impl BufferWriter {
    /// Create a new instance.
    pub fn new(max_size: usize, format: FormatFunction) -> Self {
        Self {
            state: Arc::new(Mutex::new(State {
                max_size,
                format,
                buffer: VecDeque::new(),
                size: 0,
                last_update: Instant::now(),
            })),
        }
    }

    fn lock_inner(&self) -> Result<std::sync::MutexGuard<'_, State>, std::io::Error> {
        self.state
            .lock()
            .map_err(|e| std::io::Error::other(e.to_string()))
    }

    /// Updates a snapshot with the current buffer content.
    ///
    /// Does nothing if the snapshot is up-to-date.
    ///
    /// Returns whether the snapshot was updated.
    ///
    /// # Errors
    ///
    /// `FlexiLoggerError::Poison` if some mutex is poisoned.
    pub fn update_snapshot(&self, snapshot: &mut Snapshot) -> Result<bool, FlexiLoggerError> {
        let inner = self.lock_inner()?;
        if snapshot.last_update == inner.last_update {
            Ok(false)
        } else {
            snapshot.text.clear();
            for bufline in &inner.buffer {
                snapshot.text.push_str(bufline);
                snapshot.text.push('\n');
            }
            snapshot.last_update = inner.last_update;
            Ok(true)
        }
    }
}
impl LogWriter for BufferWriter {
    fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        let mut inner = self.lock_inner()?;

        let mut logline = Vec::<u8>::with_capacity(80);
        (inner.format)(&mut logline, now, record).inspect_err(|e| {
            eprint_err(ErrorCode::Format, "formatting failed", &e);
        })?;

        if !logline.is_empty() {
            while inner.size + logline.len() > inner.max_size {
                if let Some(line) = inner.buffer.pop_front() {
                    inner.size -= line.len();
                }
            }

            (inner)
                .buffer
                .push_back(String::from_utf8_lossy(&logline).to_string());
            inner.size += logline.len();
            inner.last_update = Instant::now();
        }
        Ok(())
    }

    fn flush(&self) -> std::io::Result<()> {
        // nothing to do
        Ok(())
    }
}
