#![warn(missing_docs)]

//! A logger that can write the log to standard error or to a fresh file or to a sequence of files
//! in a configurable folder,
//! and allows custom logline formats, and whose log specification can be changed at runtime.
//!
//! It had started as an extended copy of [env_logger](http://crates.io/crates/env_logger/).
//!
//! # Usage
//!
//! Add `flexi_logger` to the dependencies in your project's `Cargo.toml`.
//!
//! ```toml
//! [dependencies]
//! flexi_logger = "0.6"
//! log = "*"
//! ```
//!
//! and this to your crate root:
//!
//! ```text
//! extern crate flexi_logger;
//! #[macro_use]
//! extern crate log;
//! ```
//!
//! The latter is needed because flexi_logger plugs into the standard Rust logging facade given
//! by the [log crate](https://crates.io/crates/log),
//! and you use the ```log``` macros to write log lines from your code.
//!
//! Early in the start-up of your program, call something like
//!
//! ```text
//!    use flexi_logger::Logger;
//!
//!    Logger::with_str("modx::mody = info")
//!        // ... your configuration options go here ...
//!        .start()
//!        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
//! ```
//!
//! The configuration options allow e.g. to
//!
//! *  decide whether you want to write your logs to stderr or to a file,
//! *  configure the filenames and the folder in which the log files are created,
//! *  specify the line format for the log lines
//!
//! See [Logger](struct.Logger.html) for a full description of all configuration options.
//!

extern crate chrono;
extern crate glob;
extern crate log;
extern crate regex;

mod deprecated;
mod flexi_error;
mod flexi_logger;
mod flexi_writer;
mod formats;
mod logger;
mod log_config;
mod log_specification;

pub use log::{LogLevel, LogLevelFilter, LogRecord};

#[allow(deprecated)]
pub use deprecated::{init, LogOptions};

pub use formats::*;
pub use log_specification::{LogSpecification, LogSpecBuilder};

pub use log_config::LogConfig;
pub use logger::Logger;
pub use flexi_logger::{FlexiLogger, ReconfigurationHandle};
pub use flexi_error::FlexiLoggerError;
