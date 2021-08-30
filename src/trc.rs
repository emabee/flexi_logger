//! ## Use `flexi_logger` functionality with [`tracing`](https://docs.rs/tracing/latest/tracing/).
//!
//! [`tracing`](https://docs.rs/tracing/latest/tracing/) is an alternative to
//! [`log`](https://docs.rs/log/latest/log/).
//! It has a similar base architecture, but is optimized for supporting async apps,
//! which adds complexity due to the need to manage contexts.
//! [`tracing-subscriber`](https://docs.rs/tracing/latest/tracing-subscriber/)
//! facilitates contributing "backends", and is used in the example below to plug
//! `flexi_logger`-functionality into `tracing`.
//!
//! **The content of this module is a first attempt to support such an integration.
//! Every feedback is highly appreciated.**
//!
//! ### Example
//!
//! The following example uses a `FileLogWriter` as trace writer,
//! and `flexi_logger`'s specfile handling to adapt `tracing` dynamically,
//! while your program is running.
//! The code is a bit cumbersome, maybe there are (oor will be) easier ways to achieve the same.
//!
//! Precondition: add these entries to your `Cargo.toml`:
//! ```toml
//! flexi_logger = {version = "0.19", features = ["trc"]}
//! tracing = "0.1"
//! tracing-subscriber = {version = "0.2.20", features = ["env-filter"]}
//! ```
//!
//! ```rust,ignore
//! # #[cfg(feature = "specfile_without_notification")]
//! # {
//! # use std::error::Error;
//! use flexi_logger::{
//!     trc::{subscribe_to_specfile, BasicLogSpecSubscriber, LogSpecAsFilter},
//!     writers::FileLogWriter,
//!     Age, Cleanup, Criterion, FileSpec, LogSpecification, Naming, WriteMode,
//! };
//!
//! use tracing::{debug, info, trace, warn};
//! use tracing_subscriber::FmtSubscriber;
//!
//! # fn main() -> Result<(), Box<dyn Error>> {
//! // Prepare a `FileLogWriter` and a handle to it, and keep the handle alive
//! // until the program ends (it will flush and shutdown the `FileLogWriter` when dropped).
//! // For the `FileLogWriter`, use the settings that fit your needs
//! let (file_writer, _fw_handle) = FileLogWriter::builder(FileSpec::default())
//!     .rotate(
//!         // If the program runs long enough,
//!         Criterion::Age(Age::Day), // - create a new file every day
//!         Naming::Timestamps,       // - let the rotated files have a timestamp in their name
//!         Cleanup::KeepLogFiles(7), // - keep at most seven log files
//!     )
//!     .write_mode(WriteMode::Async)
//!     .try_build_with_handle()
//!     .unwrap();
//!
//! // Set up subscriber that makes use of the file writer, with some hardcoded initial log spec
//! let initial_logspec = LogSpecification::info();
//! let subscriber_builder = FmtSubscriber::builder()
//!     .with_writer(move || file_writer.clone())
//!     .with_env_filter(LogSpecAsFilter(initial_logspec.clone()))
//!     .with_filter_reloading();
//!
//! // Set up specfile tracking and subscribe
//! let reload_handle = Box::new(subscriber_builder.reload_handle());
//! subscribe_to_specfile(
//!     "trcspecfile.toml",
//!     BasicLogSpecSubscriber::new(
//!         Box::new(move |logspec| reload_handle.reload(LogSpecAsFilter(logspec)).unwrap()),
//!         initial_logspec,
//!     ),
//! )
//! .unwrap();
//!
//! // Get ready to trace
//! tracing::subscriber::set_global_default(subscriber_builder.finish())
//!     .expect("setting default subscriber failed");
//!
//! // now do what you really want to do...
//! # Ok(())}}
//! ```
//!

pub use crate::logger_handle::LogSpecSubscriber;
use crate::{FlexiLoggerError, LogSpecification};
use std::path::Path;
use tracing_subscriber::EnvFilter;

/// Helper struct for using [`LogSpecification`] as filter in `tracing`.
pub struct LogSpecAsFilter(pub LogSpecification);

impl From<LogSpecAsFilter> for EnvFilter {
    fn from(my_filter: LogSpecAsFilter) -> Self {
        Self::new(my_filter.0.to_string())
    }
}

/// Allows registering a `LogSpecSubscriber` to a specfile.
///
/// Every update to the specfile will be noticed (via crate `notify`),
/// the file will be re-read, and the `LogSpecSubscriber` will be updated.
///
/// # Errors
///
/// Several variants of [`FlexiLoggerError`] can occur.
#[allow(clippy::missing_panics_doc)]
#[cfg(feature = "specfile_without_notification")]
pub fn subscribe_to_specfile<P: AsRef<Path>, H: LogSpecSubscriber>(
    specfile: P,
    subscriber: H,
) -> Result<(), FlexiLoggerError> {
    crate::logger::subscribe_to_specfile(specfile, subscriber)
}

/// Helper struct that can be registered in
/// [`subscribe_to_specfile`](fn.subscribe_to_specfile.html) to get
/// informed about updates to the specfile,
/// and can be registered in `tracing` to forward such updates.
#[cfg(feature = "specfile_without_notification")]
pub struct BasicLogSpecSubscriber {
    initial_logspec: LogSpecification,
    update: Box<(dyn Fn(LogSpecification) + Send + Sync)>,
}
impl BasicLogSpecSubscriber {
    /// Factory method.
    ///
    /// # Parameters
    /// `initial_logspec`: used to initialize the logspec file if it does not yet exist
    ///
    /// update: Closure that implements the update of the log specification to some consumer
    #[must_use]
    pub fn new(
        update: Box<(dyn Fn(LogSpecification) + Send + Sync)>,
        initial_logspec: LogSpecification,
    ) -> Self {
        Self {
            initial_logspec,
            update,
        }
    }
}
#[cfg(feature = "specfile_without_notification")]
impl LogSpecSubscriber for BasicLogSpecSubscriber {
    fn set_new_spec(&mut self, logspec: LogSpecification) -> Result<(), FlexiLoggerError> {
        (self.update)(logspec);
        Ok(())
    }

    fn initial_spec(&self) -> Result<LogSpecification, FlexiLoggerError> {
        Ok(self.initial_logspec.clone())
    }
}
