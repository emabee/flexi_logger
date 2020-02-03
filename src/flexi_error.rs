use crate::log_specification::LogSpecification;
use log;
use std::error::Error;
use std::fmt;

/// Describes errors in the initialization of `flexi_logger`.
#[derive(Debug)]
pub enum FlexiLoggerError {
    /// Log file cannot be written because the specified path is not a directory.
    BadDirectory,
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
    /// Some error occured during parsing.
    Parse(Vec<String>, LogSpecification),
    /// Logger initialization failed.
    Log(log::SetLoggerError),
}

impl fmt::Display for FlexiLoggerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Self::BadDirectory => Ok(()),
            Self::Io(ref err) => fmt::Display::fmt(err, f),
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
        }
    }
}

impl Error for FlexiLoggerError {
    fn description(&self) -> &str {
        match *self {
            Self::BadDirectory => "not a directory",
            Self::Io(ref err) => err.description(),
            Self::LevelFilter(_) => "invalid level filter",
            #[cfg(feature = "specfile")]
            Self::Notify(ref err) => err.description(),
            #[cfg(feature = "specfile")]
            Self::Toml(ref err) => err.description(),
            Self::Parse(_, _) => "Error during parsing",
            Self::Log(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&dyn Error> {
        match *self {
            Self::BadDirectory | Self::LevelFilter(_) | Self::Parse(_, _) => None,
            Self::Io(ref err) => Some(err),
            #[cfg(feature = "specfile")]
            Self::Notify(ref err) => Some(err),
            #[cfg(feature = "specfile")]
            Self::Toml(ref err) => Some(err),
            Self::Log(ref err) => Some(err),
        }
    }
}

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
