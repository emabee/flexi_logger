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
//! **The content of this module is an attempt to support such an integration.
//! Feedback is highly appreciated.**
//!
//! ### Example
//!
//! The following code example uses two features of `flexi_logger`:
//!
//! * a fully configurable `FileLogWriter` as trace writer
//! * and `flexi_logger`'s specfile handling to adapt `tracing` dynamically,
//!   while your program is running.
//!
//! Precondition: add these entries to your `Cargo.toml`:
//! ```toml
//! flexi_logger = {version = "0.23", features = ["trc"]}
//! tracing = "0.1"
//! ```
//!
//! In this example, the interaction with `tracing` components is completely hidden,
//! for convenience.
//! If you want to influence `tracing` further, what might often be the case,
//! you need to copy the code of method `setup_tracing` into your program and modify it.
//!
//! Unfortunately, especially due to the use of closures in `tracing-subscriber`'s API,
//! it is not easy to provide a convenient _and_ flexible API for plugging `flexi_logger`
//! functionality into `tracing`.
//!
//! ```rust,ignore
//! # #[cfg(feature = "specfile_without_notification")]
//! # {
//! # use std::error::Error;
//! use flexi_logger::{
//!     writers::FileLogWriter,
//!     Age, Cleanup, Criterion, FileSpec, LogSpecification, Naming, WriteMode,
//! };
//!
//! # fn main() -> Result<(), Box<dyn Error>> {
//!
//! // Drop the keep-alive-handles only in the shutdown of your program
//! let _keep_alive_handles = flexi_logger::trc::setup_tracing(
//!     LogSpecification::info(),
//!     Some(&PathBuf::from("trcspecfile.toml")),
//!     FileLogWriter::builder(FileSpec::default())
//!         .rotate(
//!             Criterion::Age(Age::Day),
//!             Naming::Timestamps,
//!             Cleanup::KeepLogFiles(7),
//!         )
//!         .write_mode(WriteMode::Async),
//!)?;
//!
//! tracing::debug!("now we start doing what we really wanted to do...")
//! # Ok(())}}
//! ```
//!

pub use crate::logger_handle::LogSpecSubscriber;
use crate::{
    logger::{create_specfile_watcher, synchronize_subscriber_with_specfile},
    writers::{FileLogWriterBuilder, FileLogWriterHandle},
};
use crate::{FlexiLoggerError, LogSpecification};
use notify_debouncer_mini::{notify::RecommendedWatcher, Debouncer};
use std::path::{Path, PathBuf};
use tracing_subscriber::{EnvFilter, FmtSubscriber};

/// Allows registering a `LogSpecSubscriber` to a specfile.
///
/// Every update to the specfile will be noticed (via crate `notify`),
/// the file will be re-read, and the `LogSpecSubscriber` will be updated.
///
/// # Errors
///
/// Several variants of [`FlexiLoggerError`] can occur.
#[cfg(feature = "specfile_without_notification")]
#[cfg_attr(docsrs, doc(cfg(feature = "specfile")))]
pub fn subscribe_to_specfile<P: AsRef<Path>>(
    specfile: P,
    reloader: Box<dyn Fn(LogSpecification) + Send + Sync>,
    initial_logspec: LogSpecification,
) -> Result<Option<Debouncer<RecommendedWatcher>>, FlexiLoggerError> {
    let specfile = specfile.as_ref();
    let mut subscriber = TraceLogSpecSubscriber::new(reloader, initial_logspec);
    synchronize_subscriber_with_specfile(&mut subscriber, specfile)?;

    if cfg!(feature = "specfile") {
        Ok(Some(create_specfile_watcher(specfile, subscriber)?))
    } else {
        Ok(None)
    }
}

/// Helper struct that can be registered in
/// [`subscribe_to_specfile`](fn.subscribe_to_specfile.html) to get
/// informed about updates to the specfile,
/// and can be registered in `tracing` to forward such updates.
#[cfg(feature = "specfile_without_notification")]
struct TraceLogSpecSubscriber {
    initial_logspec: LogSpecification,
    update: Box<(dyn Fn(LogSpecification) + Send + Sync)>,
}
impl TraceLogSpecSubscriber {
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
impl LogSpecSubscriber for TraceLogSpecSubscriber {
    fn set_new_spec(&mut self, logspec: LogSpecification) -> Result<(), FlexiLoggerError> {
        (self.update)(logspec);
        Ok(())
    }

    fn initial_spec(&self) -> Result<LogSpecification, FlexiLoggerError> {
        Ok(self.initial_logspec.clone())
    }
}

#[allow(dead_code)] // not really appropriate, seems to be a bug in clippy
/// Rereads the specfile if it was updated and forwards the update to `tracing`'s filter.
pub struct SpecFileNotifier(Option<Debouncer<RecommendedWatcher>>);

/// Set up tracing to write into the specified `FileLogWriter`,
/// and to use the (optionally) specified specfile.
///
/// The returned handles must be kept alive and should be dropped at the very end of the program.
///
/// # Panics
///
/// # Errors
///
/// Various variants of `FlexiLoggerError` can occur.
pub fn setup_tracing(
    initial_logspec: LogSpecification,
    o_specfile: Option<&PathBuf>,
    flwb: FileLogWriterBuilder,
) -> Result<(FileLogWriterHandle, SpecFileNotifier), FlexiLoggerError> {
    let (file_writer, fw_handle) = flwb.try_build_with_handle()?;

    // Set up subscriber that makes use of the file writer, with some hardcoded initial log spec
    let subscriber_builder = FmtSubscriber::builder()
        .with_writer(move || file_writer.clone())
        .with_env_filter(LogSpecAsFilter(initial_logspec.clone()))
        .with_filter_reloading();

    // Set up specfile watching
    let spec_file_notifier = SpecFileNotifier(match o_specfile {
        Some(specfile) => {
            let reload_handle = Box::new(subscriber_builder.reload_handle());
            subscribe_to_specfile(
                specfile,
                Box::new(move |logspec| {
                    { reload_handle.reload(LogSpecAsFilter(logspec)) }.unwrap(/* OK */);
                }),
                initial_logspec,
            )?
        }
        None => None,
    });

    // Get ready to trace
    tracing::subscriber::set_global_default(subscriber_builder.finish())?;

    Ok((fw_handle, spec_file_notifier))
}
struct LogSpecAsFilter(pub LogSpecification);
impl From<LogSpecAsFilter> for EnvFilter {
    fn from(wrapped_logspec: LogSpecAsFilter) -> Self {
        Self::new(wrapped_logspec.0.to_string())
    }
}
