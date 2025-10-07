//! ## Use `flexi_logger` functionality with [`tracing`](https://docs.rs/tracing/latest/tracing/).
//!
//! [`tracing`](https://docs.rs/tracing/latest/tracing/) is an alternative to
//! [`log`](https://docs.rs/log/latest/log/).
//! It has a similar base architecture, but is optimized for supporting async apps,
//! which adds complexity due to the need to manage contexts.
//! [`tracing-subscriber`](https://docs.rs/tracing/latest/tracing-subscriber/)
//! facilitates contributing "backends", and is used by [`setup_tracing`] to plug
//! `flexi_logger`-functionality into `tracing`.
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
    update: Box<dyn Fn(LogSpecification) + Send + Sync>,
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
        update: Box<dyn Fn(LogSpecification) + Send + Sync>,
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

/// Rereads the specfile if it was updated and forwards the update to `tracing`'s filter.
pub struct SpecFileNotifier {
    _watcher: Option<Debouncer<RecommendedWatcher>>,
}

/// Set up tracing to write into the specified `FileLogWriter`,
/// and to use the (optionally) specified specfile.
///
/// Not that the `FileLogWriter`'s formatting are bypassed and not be used,
/// instead the `tracing` formatting will be used, which can be configured via `config: FormatConfig`.
///
/// The returned handles must be kept alive and should be dropped at the very end of the program.
///
/// ### Example
///
/// Use [`FileLogWriter`]([crate::FileLogWriter) as trace writer,
/// with support for dynamically adapting trace levels while your program is running.
///
/// ```rust
/// # #[cfg(feature = "specfile_without_notification")]
/// # {
/// use std::{error::Error, path::PathBuf};
/// use flexi_logger::{
///     trc::FormatConfig,
///     writers::FileLogWriter,
///     Age, Cleanup, Criterion, FileSpec, LogSpecification, Naming, WriteMode,
/// };
///
/// # fn main() -> Result<(), Box<dyn Error>> {
///
/// // Drop the keep-alive-handles only in the shutdown of your program
/// let _keep_alive_handles = flexi_logger::trc::setup_tracing(
///     LogSpecification::info(),
///     Some(&PathBuf::from("trcspecfile.toml")),
///     FileLogWriter::builder(FileSpec::default())
///         .rotate(
///             Criterion::Age(Age::Day),
///             Naming::Timestamps,
///             Cleanup::KeepLogFiles(7),
///         )
///         .write_mode(WriteMode::Async),
///         &FormatConfig::default()
///            .with_file(true),
///)?;
///
/// tracing::debug!("now we start doing what we really wanted to do...");
/// # Ok(())}}
/// ```
///
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
    config: &FormatConfig,
) -> Result<(FileLogWriterHandle, SpecFileNotifier), FlexiLoggerError> {
    let (file_writer, fw_handle) = flwb.try_build_with_handle()?;

    // Set up subscriber that makes use of the file writer, with some hardcoded initial log spec.
    // Ugly code duplication here, due to "sophisticated" use of types in `tracing-subscriber`
    if config.with_time {
        let subscriber_builder = FmtSubscriber::builder()
            .with_writer(move || file_writer.clone())
            .with_ansi(config.with_ansi)
            .with_env_filter(EnvFilter::from_default_env())
            .with_file(config.with_file)
            .with_filter_reloading()
            .with_level(config.with_level)
            .with_line_number(config.with_line_number)
            .with_target(config.with_target)
            .with_thread_ids(config.with_thread_ids)
            .with_thread_names(config.with_thread_names)
            .with_thread_ids(config.with_thread_ids)
            .with_thread_names(config.with_thread_names)
            .with_env_filter(LogSpecAsFilter(initial_logspec.clone()))
            .with_filter_reloading();
        // Set up specfile watching
        let spec_file_notifier = SpecFileNotifier {
            _watcher: match o_specfile {
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
            },
        };

        // Get ready to trace
        tracing::subscriber::set_global_default(subscriber_builder.finish())?;

        Ok((fw_handle, spec_file_notifier))
    } else {
        let subscriber_builder = FmtSubscriber::builder()
            .without_time()
            .with_writer(move || file_writer.clone())
            .with_ansi(config.with_ansi)
            .with_env_filter(EnvFilter::from_default_env())
            .with_file(config.with_file)
            .with_filter_reloading()
            .with_level(config.with_level)
            .with_line_number(config.with_line_number)
            .with_target(config.with_target)
            .with_thread_ids(config.with_thread_ids)
            .with_thread_names(config.with_thread_names)
            .with_thread_ids(config.with_thread_ids)
            .with_thread_names(config.with_thread_names)
            .with_env_filter(LogSpecAsFilter(initial_logspec.clone()))
            .with_filter_reloading();
        // Set up specfile watching
        let spec_file_notifier = SpecFileNotifier {
            _watcher: match o_specfile {
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
            },
        };

        // Get ready to trace
        tracing::subscriber::set_global_default(subscriber_builder.finish())?;
        Ok((fw_handle, spec_file_notifier))
    }
}

struct LogSpecAsFilter(pub LogSpecification);
impl From<LogSpecAsFilter> for EnvFilter {
    fn from(wrapped_logspec: LogSpecAsFilter) -> Self {
        Self::new(wrapped_logspec.to_trc_env_filter())
    }
}
impl LogSpecAsFilter {
    pub fn to_trc_env_filter(&self) -> String {
        let mut s = String::new();
        let mut write_comma = false;
        if let Some(last) = self.0.module_filters.last() {
            if last.module_name.is_none() {
                s.push_str(&last.level_filter.to_string().to_lowercase());
                write_comma = true;
            }
        }
        for mf in &self.0.module_filters {
            if let Some(ref name) = mf.module_name {
                if write_comma {
                    s.push(',');
                }
                s.push_str(&format!("{name}={}", mf.level_filter.to_string().to_lowercase()));
                write_comma = true;
            }
        }
        s
    }
}

/// Configuration for the `tracing` formatting.
///
/// Exposes the formatting capabilities of `tracing-subscriber::FmtSubscriber`.
/// These deviate from the formatting capabilities of `flexi_logger`.
#[allow(clippy::struct_excessive_bools)]
pub struct FormatConfig {
    with_ansi: bool,
    with_file: bool,
    with_level: bool,
    with_line_number: bool,
    with_target: bool,
    with_thread_ids: bool,
    with_thread_names: bool,
    with_time: bool,
}
impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            with_ansi: false,
            with_file: false,
            with_level: true,
            with_line_number: true,
            with_target: false,
            with_thread_ids: false,
            with_thread_names: false,
            with_time: true,
        }
    }
}
impl FormatConfig {
    /// Decides whether to use ANSI colors in the output.
    ///
    /// Defaults to `false`.
    #[must_use]
    pub fn with_ansi(mut self, with_ansi: bool) -> Self {
        self.with_ansi = with_ansi;
        self
    }
    /// Decides whether to include the file name in the output.
    ///
    /// Defaults to `false`.
    #[must_use]
    pub fn with_file(mut self, with_file: bool) -> Self {
        self.with_file = with_file;
        self
    }
    /// Decides whether to include the log level in the output.
    ///
    /// Defaults to `true`.
    #[must_use]
    pub fn with_level(mut self, with_level: bool) -> Self {
        self.with_level = with_level;
        self
    }
    /// Decides whether to include the line number in the output.
    ///
    /// Defaults to `true`.
    #[must_use]
    pub fn with_line_number(mut self, with_line_number: bool) -> Self {
        self.with_line_number = with_line_number;
        self
    }
    /// Decides whether to include the target in the output.
    ///
    /// Defaults to `false`.
    #[must_use]
    pub fn with_target(mut self, with_target: bool) -> Self {
        self.with_target = with_target;
        self
    }
    /// Decides whether to include the thread IDs in the output.
    ///
    /// Defaults to `false`.
    #[must_use]
    pub fn with_thread_ids(mut self, with_thread_ids: bool) -> Self {
        self.with_thread_ids = with_thread_ids;
        self
    }
    /// Decides whether to include the thread names in the output.
    ///  
    /// Defaults to `false`.
    #[must_use]
    pub fn with_thread_names(mut self, with_thread_names: bool) -> Self {
        self.with_thread_names = with_thread_names;
        self
    }
    /// Decides whether to include the time in the output.
    ///
    /// Defaults to `true`.
    #[must_use]
    pub fn with_time(mut self, with_time: bool) -> Self {
        self.with_time = with_time;
        self
    }
}
