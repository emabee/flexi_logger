use log;
use std::fmt;
use std::io;
use std::error::Error;


/// Describes errors in the initialization of `flexi_logger`.
#[derive(Debug)]
pub enum FlexiLoggerError {
    /// Log file cannot be written because the specified path is not a directory.
    BadDirectory,
    /// Log cannot be written because the configured output directory is not accessible.
    Io(io::Error),
    /// Logger initialization failed.
    Log(log::SetLoggerError),
}


impl fmt::Display for FlexiLoggerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            FlexiLoggerError::BadDirectory => Ok(()),
            FlexiLoggerError::Io(ref err) => err.fmt(f),
            FlexiLoggerError::Log(ref err) => err.fmt(f),
        }
    }
}

impl Error for FlexiLoggerError {
    fn description(&self) -> &str {
        match *self {
            FlexiLoggerError::BadDirectory => "not a directory", // ""
            FlexiLoggerError::Io(ref err) => err.description(),  // "Log cannot be written"
            FlexiLoggerError::Log(ref err) => err.description(), // "Logger initialization failed"
        }
    }

    fn cause(&self) -> Option<&Error> {
        match *self {
            FlexiLoggerError::BadDirectory => None,
            FlexiLoggerError::Io(ref err) => Some(err),
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
