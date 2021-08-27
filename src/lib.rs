// only enables the `doc_cfg` feature when the `docsrs` configuration attribute is defined
#![cfg_attr(docsrs, feature(doc_cfg))]
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

pub mod code_examples;
pub mod filter;
mod util;
pub mod writers;

pub use crate::deferred_now::DeferredNow;
pub use crate::file_spec::FileSpec;
pub use crate::flexi_error::FlexiLoggerError;
pub use crate::formats::*;
pub use crate::log_specification::{LogSpecBuilder, LogSpecification, ModuleFilter};
pub use crate::logger::{Duplicate, Logger, WriteMode};
pub use crate::logger_handle::LoggerHandle;
pub use crate::parameters::{Age, Cleanup, Criterion, Naming};

/// Default buffer capacity (8k), when buffering is used.
pub const DEFAULT_BUFFER_CAPACITY: usize = 8 * 1024;

/// Default flush interval (1s), when flushing is used.
pub const DEFAULT_FLUSH_INTERVAL: std::time::Duration = std::time::Duration::from_secs(1);

/// Default size of the message pool that is used with [`WriteMode::Async`];
/// a higher value could further reduce allocations during log file rotation and cleanup.
pub const DEFAULT_POOL_CAPA: usize = 50;

/// Default capacity for the message buffers that are used with [`WriteMode::Async`];
/// a higher value reduces allocations when longer log lines are used.
pub const DEFAULT_MESSAGE_CAPA: usize = 200;

/// Re-exports from log crate
pub use log::{Level, LevelFilter, Record};
