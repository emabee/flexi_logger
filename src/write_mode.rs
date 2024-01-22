use std::time::Duration;

/// Default buffer capacity (8k), when buffering is used.
pub const DEFAULT_BUFFER_CAPACITY: usize = 8 * 1024;

/// Default flush interval (1s), when flushing is used.
pub const DEFAULT_FLUSH_INTERVAL: Duration = Duration::from_secs(1);

/// Default size of the message pool;
/// a higher value could further reduce allocations during log file rotation and cleanup.
#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
pub const DEFAULT_POOL_CAPA: usize = 50;

/// Default capacity for the message buffers;
/// a higher value reduces allocations when longer log lines are used.
#[cfg(feature = "async")]
#[cfg_attr(docsrs, doc(cfg(feature = "async")))]
pub const DEFAULT_MESSAGE_CAPA: usize = 200;

/// Describes whether the log output should be written synchronously or asynchronously,
/// and if and how I/O should be buffered and flushed.
///
/// Is used in [`Logger::write_mode`](struct.Logger.html#method.write_mode).
///
/// Buffering reduces the program's I/O overhead, and thus increases overall performance,
/// which can become relevant if logging is used heavily.
/// On the other hand, if logging is used with low frequency,
/// buffering can defer the appearance of log lines significantly,
/// so regular flushing is usually advisable with buffering.
///
/// **Note** that for all options except `Direct` you should keep the
/// [`LoggerHandle`](struct.LoggerHandle.html) alive
/// up to the very end of your program to ensure that all buffered log lines are flushed out
/// (which happens automatically when the [`LoggerHandle`](struct.LoggerHandle.html) is dropped)
/// before the program terminates.
/// [See here for an example](code_examples/index.html#choose-the-write-mode).
///
/// **Note** further that flushing uses an extra thread (with minimal stack).
///
/// The console is a slow output device (at least on Windows).
/// With `WriteMode::Async` it can happen that in phases with vast log output
/// the log lines appear significantly later than they were written.
/// Also, a final printing phase is possible at the end of the program when the logger handle
/// is dropped (and all output is flushed automatically).
///
/// `WriteMode::Direct` (i.e. without buffering) is the slowest option with all output devices,
/// showing that buffered I/O pays off. But it takes slightly more resources, especially
/// if you do not suppress flushing.
///
/// Using `log_to_stdout()` and then redirecting the output to a file makes things faster,
/// but is still significantly slower than writing to files directly.
///
#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum WriteMode {
    /// Do not buffer (default).
    ///
    /// Every log line is directly written to the output, without buffering.
    /// This allows seeing new log lines in real time, and does not need additional threads.
    Direct,

    /// Do not buffer and support `cargo test`'s capture.
    ///
    /// Much like `Direct`, just a bit slower, and allows
    /// `cargo test` to capture log output and print it only for failing tests.
    SupportCapture,

    /// Same as `BufferAndFlushWith` with default capacity ([`DEFAULT_BUFFER_CAPACITY`])
    /// and default interval ([`DEFAULT_FLUSH_INTERVAL`]).
    BufferAndFlush,

    /// Buffer and flush with given buffer capacity and flush interval.
    BufferAndFlushWith(
        /// Buffer capacity.
        usize,
        /// Flush interval.
        Duration,
    ),

    /// Same as `BufferDontFlushWith` with default capacity ([`DEFAULT_BUFFER_CAPACITY`]).
    BufferDontFlush,

    /// Buffer with given buffer capacity, but don't flush.
    ///
    /// This might be handy if you want to minimize I/O effort and don't want to create
    /// the extra thread for flushing and don't care if log lines appear with delay.
    BufferDontFlushWith(
        /// Buffer capacity.
        usize,
    ),

    /// Same as `AsyncWith`, using default values for all parameters.
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    #[cfg(feature = "async")]
    Async,

    /// Log lines are sent through an unbounded channel to an output thread, which
    /// does the I/O, and, if `log_to_file()` is chosen, also the rotation and the cleanup.
    ///
    /// Uses buffered output to reduce overhead, and a bounded message pool to reduce allocations.
    /// The log output is flushed regularly with the given interval.
    ///
    /// See [here](code_examples/index.html#choose-the-write-mode) for an example.
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    #[cfg(feature = "async")]
    AsyncWith {
        /// Capacity of the pool for the message buffers.
        pool_capa: usize,
        /// Capacity of an individual message buffer.
        message_capa: usize,
        /// The interval for flushing the output.
        ///
        /// With `Duration::ZERO` flushing is suppressed.
        flush_interval: Duration,
    },
}

pub(crate) enum EffectiveWriteMode {
    Direct,
    #[allow(dead_code)] // introduced due to a bug in clippy, should be removed again
    BufferAndFlushWith(usize, Duration),
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    #[cfg(feature = "async")]
    AsyncWith {
        /// Capacity of the pool for the message buffers.
        pool_capa: usize,
        /// Capacity of an individual message buffer.
        message_capa: usize,
        /// The interval for flushing the output.
        ///
        /// With `Duration::ZERO` flushing is suppressed.
        flush_interval: Duration,
    },
    BufferDontFlushWith(usize),
}

impl WriteMode {
    pub(crate) fn inner(&self) -> EffectiveWriteMode {
        match *self {
            Self::Direct | Self::SupportCapture => EffectiveWriteMode::Direct,
            Self::BufferDontFlush => {
                EffectiveWriteMode::BufferDontFlushWith(DEFAULT_BUFFER_CAPACITY)
            }
            Self::BufferDontFlushWith(duration) => {
                EffectiveWriteMode::BufferDontFlushWith(duration)
            }
            Self::BufferAndFlush => EffectiveWriteMode::BufferAndFlushWith(
                DEFAULT_BUFFER_CAPACITY,
                DEFAULT_FLUSH_INTERVAL,
            ),
            Self::BufferAndFlushWith(bufsize, duration) => {
                EffectiveWriteMode::BufferAndFlushWith(bufsize, duration)
            }
            #[cfg(feature = "async")]
            Self::Async => EffectiveWriteMode::AsyncWith {
                pool_capa: DEFAULT_POOL_CAPA,
                message_capa: DEFAULT_MESSAGE_CAPA,
                flush_interval: DEFAULT_FLUSH_INTERVAL,
            },
            #[cfg(feature = "async")]
            Self::AsyncWith {
                pool_capa,
                message_capa,
                flush_interval,
            } => EffectiveWriteMode::AsyncWith {
                pool_capa,
                message_capa,
                flush_interval,
            },
        }
    }
    pub(crate) fn without_flushing(&self) -> WriteMode {
        match self {
            Self::Direct
            | Self::SupportCapture
            | Self::BufferDontFlush
            | Self::BufferDontFlushWith(_) => *self,
            Self::BufferAndFlush => Self::BufferDontFlush,
            Self::BufferAndFlushWith(bufsize, _) => Self::BufferDontFlushWith(*bufsize),
            #[cfg(feature = "async")]
            Self::Async => Self::AsyncWith {
                pool_capa: DEFAULT_POOL_CAPA,
                message_capa: DEFAULT_MESSAGE_CAPA,
                flush_interval: Duration::from_secs(0),
            },
            #[cfg(feature = "async")]
            Self::AsyncWith {
                pool_capa,
                message_capa,
                flush_interval: _,
            } => Self::AsyncWith {
                pool_capa: *pool_capa,
                message_capa: *message_capa,
                flush_interval: Duration::from_secs(0),
            },
        }
    }
    pub(crate) fn buffersize(&self) -> Option<usize> {
        match self.inner() {
            EffectiveWriteMode::Direct => None,
            EffectiveWriteMode::BufferAndFlushWith(bufsize, _)
            | EffectiveWriteMode::BufferDontFlushWith(bufsize) => Some(bufsize),
            #[cfg(feature = "async")]
            EffectiveWriteMode::AsyncWith {
                pool_capa: _,
                message_capa: _,
                flush_interval: _,
            } => None,
        }
    }
    pub(crate) fn get_flush_interval(&self) -> Duration {
        match self {
            Self::Direct
            | Self::SupportCapture
            | Self::BufferDontFlush
            | Self::BufferDontFlushWith(_) => Duration::from_secs(0),
            Self::BufferAndFlush => DEFAULT_FLUSH_INTERVAL,
            #[cfg(feature = "async")]
            Self::Async => DEFAULT_FLUSH_INTERVAL,
            Self::BufferAndFlushWith(_, flush_interval) => *flush_interval,
            #[cfg(feature = "async")]
            Self::AsyncWith {
                pool_capa: _,
                message_capa: _,
                flush_interval,
            } => *flush_interval,
        }
    }
}
