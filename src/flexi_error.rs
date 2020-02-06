use crate::log_specification::LogSpecification;
use log;
use std::error::Error;
use std::fmt;

/// Describes errors in the initialization of `flexi_logger`.
#[derive(Debug)]
pub enum FlexiLoggerError {
    /// Log file cannot be written because the specified path is not a directory.
    BadDirectory,
    /// Spawning the cleanup thread failed.
    ///
    /// This error can safely be avoided with `Logger::cleanup_in_background_thread(false)`.
    CleanupThread(std::io::Error),
    /// Log cannot be written because the configured output directory is not accessible.
    Io(std::io::Error),
    /// Error with the filesystem notifications for the specfile.
    #[cfg(feature = "specfile")]
    Notify(notify::Error),
    /// The configured logspec file cannot be read.
    #[cfg(feature = "specfile")]
    Toml(toml::de::Error),
    /// Invalid level filter.
    LevelFilter(String),
    /// Some error occured during parsing as log specification.
    Parse(Vec<String>, LogSpecification),
    /// Logger initialization failed.
    Log(log::SetLoggerError),
    /// Some synchronization object is poisoned
    Poison,
}

impl fmt::Display for FlexiLoggerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::BadDirectory => f.write_str("Bad directory"),
            Self::CleanupThread(ref err) | Self::Io(ref err) => fmt::Display::fmt(err, f),
            Self::LevelFilter(ref s) => f.write_str(s),
            #[cfg(feature = "specfile")]
            Self::Notify(ref err) => fmt::Display::fmt(err, f),
            #[cfg(feature = "specfile")]
            Self::Toml(ref err) => fmt::Display::fmt(err, f),
            Self::Parse(ref vec, ref logspec) => {
                for s in vec {
                    f.write_str(&format!("parse error: \'{}\', ", s))?;
                }
                f.write_str(&format!("resulting logspec: {:?}", logspec))?;
                Ok(())
            }
            Self::Log(ref err) => fmt::Display::fmt(err, f),
            Self::Poison => fmt::Display::fmt("Some synchronization object is poisoned", f),
        }
    }
}

impl Error for FlexiLoggerError {}

impl From<log::SetLoggerError> for FlexiLoggerError {
    #[must_use]
    fn from(err: log::SetLoggerError) -> Self {
        Self::Log(err)
    }
}
impl From<std::io::Error> for FlexiLoggerError {
    #[must_use]
    fn from(err: std::io::Error) -> Self {
        Self::Io(err)
    }
}
impl From<glob::PatternError> for FlexiLoggerError {
    #[must_use]
    fn from(err: glob::PatternError) -> Self {
        Self::Io(std::io::Error::new(std::io::ErrorKind::Other, err))
    }
}

#[cfg(feature = "specfile")]
impl From<toml::de::Error> for FlexiLoggerError {
    #[must_use]
    fn from(err: toml::de::Error) -> Self {
        Self::Toml(err)
    }
}
#[cfg(feature = "specfile")]
impl From<notify::Error> for FlexiLoggerError {
    #[must_use]
    fn from(err: notify::Error) -> Self {
        Self::Notify(err)
    }
}
