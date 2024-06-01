// only enables the `doc_cfg` feature when the `docsrs` configuration attribute is defined
#![cfg_attr(docsrs, feature(doc_cfg))]
#![deny(missing_docs)]
#![deny(clippy::all)]
#![deny(clippy::pedantic)]
#![forbid(unsafe_code)]
//! A flexible and easy-to-use logger that writes logs to stderr and/or to files
//! or other output streams.
//!
//! To read the log specification from an environment variable and get the log written to `stderr`,
//! start `flexi_logger` e.g. like this:
//! ```rust
//! flexi_logger::Logger::try_with_env().unwrap().start().unwrap();
//! ```
//!
//! See
//!
//! * The builder [`Logger`] for a full description of all configuration options,
//! * module [`code_examples`] for various concrete examples of `flexi_logger` initialization
//! * the module [`writers`] for the usage of additional log writers,
//! * and [the README](https://crates.io/crates/flexi_logger) for how to get started.
//!
//! There are configuration options to e.g.
//!
//! * decide whether you want to write your logs to stderr or to a file,
//! * configure the path and the filenames of the log files,
//! * use file rotation,
//! * specify the line format for the log lines,
//! * apply a stateful filter before log lines are really written,
//! * define additional log output streams, e.g for alert or security messages,
//! * support changing the log specification while the program is running,
//!
//! `flexi_logger` uses a similar syntax as [`env_logger`](http://crates.io/crates/env_logger/)
//! for specifying which logs should really be written (but is more graceful with the syntax,
//! and can provide error information).
//!
//! By default, i.e. if feature `colors` is not switched off, the log lines that appear on your
//! terminal are coloured. In case the chosen colors don't fit to your terminal's color theme,
//! you can adapt the colors to improve readability.
//! See the documentation of method [`Logger::set_palette`]
//! for a description how this can be done.

mod deferred_now;
mod file_spec;
mod flexi_error;
mod flexi_logger;
mod formats;
mod log_specification;
mod logger;
mod logger_handle;
mod parameters;
mod primary_writer;
mod threads;
#[cfg(feature = "trc")]
#[cfg_attr(docsrs, doc(cfg(feature = "trc")))]
pub mod trc;
mod write_mode;

pub mod code_examples;
pub mod filter;
mod util;
pub mod writers;

pub mod error_info;

pub use crate::deferred_now::DeferredNow;
pub use crate::file_spec::FileSpec;
pub use crate::flexi_error::FlexiLoggerError;
pub use crate::formats::*;
pub use crate::log_specification::{LogSpecBuilder, LogSpecification, ModuleFilter};
pub use crate::logger::{Duplicate, ErrorChannel, Logger};
pub use crate::logger_handle::{LogfileSelector, LoggerHandle};
pub use crate::parameters::{Age, Cleanup, Criterion, Naming};
pub(crate) use crate::write_mode::EffectiveWriteMode;
pub use crate::write_mode::{WriteMode, DEFAULT_BUFFER_CAPACITY, DEFAULT_FLUSH_INTERVAL};
#[cfg(feature = "async")]
pub use crate::write_mode::{DEFAULT_MESSAGE_CAPA, DEFAULT_POOL_CAPA};

/// Re-exports from log crate
pub use log::{Level, LevelFilter, Record};

/// Shortest form to get started.
///
/// `flexi_logger::init();`.
///
/// Equivalent to
/// ```rust
/// # use flexi_logger::{Logger,LogSpecification};
///     Logger::try_with_env_or_str("info")
///        .unwrap_or_else(|_e| Logger::with(LogSpecification::info()))
///        .log_to_stderr()
///        .start()
///        .ok();
/// ```
/// that means,
///
/// - you configure the log specification via the environment variable `RUST_LOG`,
///   or use the default log specification (`'info'`)
/// - logs are directly written to `stderr`, without any buffering, so implicitly dropping the
///   `LogHandle` (which is returned from `Logger::start()`) is ok.
pub fn init() {
    Logger::try_with_env_or_str("info")
        .unwrap_or_else(|_e| Logger::with(LogSpecification::info()))
        .log_to_stderr()
        .start()
        .ok();
}
