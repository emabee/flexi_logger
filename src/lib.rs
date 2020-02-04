#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![allow(clippy::unused_self)]
#![allow(clippy::needless_doctest_main)]

//! A flexible and easy-to-use logger that writes logs to stderr and/or to files
//! or other output streams.
//!
//! To read the log specification from an environment variable and get the log written to `stderr`,
//! start `flexi_logger` e.g. like this:
//! ```rust
//! flexi_logger::Logger::with_env().start().unwrap();
//! ```
//!
//! See
//!
//! * [Logger](struct.Logger.html) for a full description of all configuration options,
//! * and the [writers](writers/index.html) module for the usage of additional log writers,
//! * and [the homepage](https://crates.io/crates/flexi_logger) for how to get started.
//!
//! There are configuration options to e.g.
//!
//! * decide whether you want to write your logs to stderr or to a file,
//! * configure the path and the filenames of the log files,
//! * use file rotation,
//! * specify the line format for the log lines,
//! * define additional log output streams, e.g for alert or security messages,
//! * support changing the log specification while the program is running,
//!
//! `flexi_logger` uses a similar syntax as [`env_logger`](http://crates.io/crates/env_logger/)
//! for specifying which logs should really be written (but is more graceful with the syntax,
//! and can provide error information).

mod deferred_now;
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

pub use crate::deferred_now::DeferredNow;
pub use crate::flexi_error::FlexiLoggerError;
pub use crate::formats::*;
pub use crate::log_specification::{LogSpecBuilder, LogSpecification, ModuleFilter};
pub use crate::logger::{Age, Cleanup, Criterion, Duplicate, LogTarget, Logger, Naming};
pub use crate::reconfiguration_handle::ReconfigurationHandle;

/// Function type for Format functions.
///
/// If you want to write the log lines in your own format,
/// implement a function with this signature and provide it to one of the methods
/// [`Logger::format()`](struct.Logger.html#method.format),
/// [`Logger::format_for_files()`](struct.Logger.html#method.format_for_files),
/// or [`Logger::format_for_stderr()`](struct.Logger.html#method.format_for_stderr).
///
/// Checkout the code of the provided [format functions](index.html#functions)
/// if you want to start with a template.
///
/// ## Parameters
///
/// - `write`: the output stream
///
/// - `now`: the timestamp that you should use if you want a timestamp to appear in the log line
///
/// - `record`: the log line's content and metadata, as provided by the log crate's macros.
///
pub type FormatFunction = fn(
    write: &mut dyn std::io::Write,
    now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error>;
