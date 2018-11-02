use crate::log_specification::LogSpecification;
use log;
#[cfg(feature = "specfile")]
use notify;
use std::error::Error;
use std::fmt;
use std::io;
#[cfg(feature = "specfile")]
use toml;

/// Describes errors in the initialization of `flexi_logger`.
#[derive(Debug)]
pub enum FlexiLoggerError {
    /// Log file cannot be written because the specified path is not a directory.
    BadDirectory,
    /// Log cannot be written because the configured output directory is not accessible.
    Io(io::Error),
    /// Error with fs-notifications for the specfile
    #[cfg(feature = "specfile")]
    Notify(notify::Error),
    /// The configured logspec file cannot be read
    #[cfg(feature = "specfile")]
    Toml(toml::de::Error),
    /// Invalid level filter
    LevelFilter(String),
    /// Some error occured during parsing
    Parse(Vec<String>, LogSpecification),
    /// Logger initialization failed.
    Log(log::SetLoggerError),
}

impl fmt::Display for FlexiLoggerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FlexiLoggerError::BadDirectory => Ok(()),
            FlexiLoggerError::Io(ref err) => fmt::Display::fmt(err, f),
            FlexiLoggerError::LevelFilter(ref s) => f.write_str(s),
            #[cfg(feature = "specfile")]
            FlexiLoggerError::Notify(ref err) => fmt::Display::fmt(err, f),
            #[cfg(feature = "specfile")]
            FlexiLoggerError::Toml(ref err) => fmt::Display::fmt(err, f),
            FlexiLoggerError::Parse(ref vec, ref logspec) => {
                for s in vec {
                    f.write_str(&format!("parse error: \'{}\', ", s))?;
                }
                f.write_str(&format!("resulting logspec: {:?}", logspec))?;
                Ok(())
            }
            FlexiLoggerError::Log(ref err) => fmt::Display::fmt(err, f),
        }
    }
}

impl Error for FlexiLoggerError {
    fn description(&self) -> &str {
        match *self {
            FlexiLoggerError::BadDirectory => "not a directory",
            FlexiLoggerError::Io(ref err) => err.description(),
            FlexiLoggerError::LevelFilter(_) => "invalid level filter",
            #[cfg(feature = "specfile")]
            FlexiLoggerError::Notify(ref err) => err.description(),
            #[cfg(feature = "specfile")]
            FlexiLoggerError::Toml(ref err) => err.description(),
            FlexiLoggerError::Parse(_, _) => "Error during parsing",
            FlexiLoggerError::Log(ref err) => err.description(),
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            FlexiLoggerError::BadDirectory
            | FlexiLoggerError::LevelFilter(_)
            | FlexiLoggerError::Parse(_, _) => None,
            FlexiLoggerError::Io(ref err) => Some(err),
            #[cfg(feature = "specfile")]
            FlexiLoggerError::Notify(ref err) => Some(err),
            #[cfg(feature = "specfile")]
            FlexiLoggerError::Toml(ref err) => Some(err),
            FlexiLoggerError::Log(ref err) => Some(err),
        }
    }
}

impl From<log::SetLoggerError> for FlexiLoggerError {
    fn from(err: log::SetLoggerError) -> FlexiLoggerError {
        FlexiLoggerError::Log(err)
    }
}
impl From<io::Error> for FlexiLoggerError {
    fn from(err: io::Error) -> FlexiLoggerError {
        FlexiLoggerError::Io(err)
    }
}
#[cfg(feature = "specfile")]
impl From<toml::de::Error> for FlexiLoggerError {
    fn from(err: toml::de::Error) -> FlexiLoggerError {
        FlexiLoggerError::Toml(err)
    }
}
#[cfg(feature = "specfile")]
impl From<notify::Error> for FlexiLoggerError {
    fn from(err: notify::Error) -> FlexiLoggerError {
        FlexiLoggerError::Notify(err)
    }
}
