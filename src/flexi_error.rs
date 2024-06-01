use crate::log_specification::LogSpecification;
// use std::backtrace::Backtrace;
use thiserror::Error;

/// Describes errors in the initialization of `flexi_logger`.
#[non_exhaustive]
#[derive(Error, Debug)]
pub enum FlexiLoggerError {
    /// Chosen reset not possible.
    #[error("Chosen reset not possible")]
    Reset,

    /// Method not possible because duplication is not possible.
    #[error("Method not possible because duplication is not possible")]
    NoDuplication,

    /// Method not possible because no file logger is configured.
    #[error("Method not possible because no file logger is configured")]
    NoFileLogger,

    /// Log file cannot be written because the specified path is not a directory.
    #[error("Log file cannot be written because the specified path is not a directory")]
    OutputBadDirectory,

    /// Log file cannot be written because the specified path is a directory.
    #[error("Log file cannot be written because the specified path is a directory")]
    OutputBadFile,

    /// Spawning the cleanup thread failed.
    ///
    /// This error can safely be avoided with `Logger::cleanup_in_background_thread(false)`.
    #[error("Spawning the cleanup thread failed.")]
    OutputCleanupThread(std::io::Error),

    /// Log cannot be written, e.g. because the configured output directory is not accessible.
    #[error(
        "Log cannot be written, e.g. because the configured output directory is not accessible"
    )]
    OutputIo(#[from] std::io::Error),

    /// Parsing the configured logspec toml-file failed.
    #[error("Parsing the configured logspec toml-file failed")]
    #[cfg(feature = "specfile_without_notification")]
    #[cfg_attr(docsrs, doc(cfg(feature = "specfile")))]
    SpecfileToml(#[from] toml::de::Error),

    /// Specfile cannot be accessed or created.
    #[error("Specfile cannot be accessed or created")]
    #[cfg(feature = "specfile_without_notification")]
    #[cfg_attr(docsrs, doc(cfg(feature = "specfile")))]
    SpecfileIo(std::io::Error),

    /// Specfile has an unsupported extension.
    #[error("Specfile has an unsupported extension")]
    #[cfg(feature = "specfile_without_notification")]
    #[cfg_attr(docsrs, doc(cfg(feature = "specfile")))]
    SpecfileExtension(&'static str),

    /// Invalid level filter.
    #[error("Invalid level filter")]
    LevelFilter(String),

    /// Failed to parse log specification.
    ///
    /// The String contains a description of the error, the second parameter
    /// contains the resulting [`LogSpecification`] object
    #[error("Failed to parse log specification: {0}")]
    Parse(String, LogSpecification),

    /// Logger initialization failed.
    #[error("Logger initialization failed")]
    Log(#[from] log::SetLoggerError),

    /// Some synchronization object is poisoned.
    #[error("Some synchronization object is poisoned")]
    Poison,

    /// Palette parsing failed
    #[error("Palette parsing failed")]
    Palette(#[from] std::num::ParseIntError),

    /// Logger is shut down.
    #[cfg(feature = "async")]
    #[cfg_attr(docsrs, doc(cfg(feature = "async")))]
    #[error("Logger is shut down")]
    Shutdown(#[from] crossbeam_channel::SendError<Vec<u8>>),

    /// Tracing initialization failed.
    #[cfg(feature = "trc")]
    #[cfg_attr(docsrs, doc(cfg(feature = "trc")))]
    #[error("Tracing initialization failed")]
    TracingSetup(#[from] tracing::subscriber::SetGlobalDefaultError),
}

impl From<std::convert::Infallible> for FlexiLoggerError {
    fn from(_other: std::convert::Infallible) -> FlexiLoggerError {
        unreachable!("lkjl,mnkjiu")
    }
}
