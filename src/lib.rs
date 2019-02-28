#![warn(missing_docs)]

//! A flexible and easy-to-use logger that writes logs to stderr and/or to files.
//!
//! There are configuration options to e.g.
//!
//! * decide whether you want to write your logs to stderr or to a file,
//! * configure the path and the filenames of the log files,
//! * use file rotation,
//! * specify the line format for the log lines,
//! * define additional log streams, e.g for alert or security messages,
//! * support changing the log specification on the fly, while the program is running,
//!
//! `flexi_logger` uses a similar syntax as [`env_logger`](http://crates.io/crates/env_logger/)
//! for specifying which logs should really be written (but is more graceful with the syntax,
//! and can provide error information).
//!
//! See [Logger](struct.Logger.html) for a full description of all configuration options,
//! and the [writers](writers/index.html) module for the usage of additional log writers.
//!
//! See [the homepage](https://crates.io/crates/flexi_logger) for how to get started.

mod flexi_error;
mod flexi_logger;
mod formats;
mod log_specification;
mod logger;
mod primary_writer;
mod reconfiguration_handle;

pub mod writers;

/// Re-exports from log crate
pub use log::{Level, LevelFilter, Record};

pub use crate::flexi_error::FlexiLoggerError;
pub use crate::formats::*;
pub use crate::log_specification::{LogSpecBuilder, LogSpecification};
pub use crate::logger::{Duplicate, Logger};
pub use crate::reconfiguration_handle::ReconfigurationHandle;

use std::io;

/// Function type for Format functions.
pub type FormatFunction = fn(&mut io::Write, &Record) -> Result<(), io::Error>;
