#![doc(html_logo_url = "http://www.rust-lang.org/logos/rust-logo-128x128-blk-v2.png",
       html_favicon_url = "http://www.rust-lang.org/favicon.ico",
       html_root_url = "http://doc.rust-lang.org/")]
#![warn(missing_docs)]

//! A logger that can write the log to standard error or to a fresh file in a configurable folder
//! and allows custom logline formats.
//! It had started as an extended copy of [env_logger](http://rust-lang.github.io/log/env_logger/).
//!
//! # Usage
//!
//! This crate is on [crates.io](https://crates.io/crates/flexi_logger) and
//! can be used by adding `flexi_logger` to the dependencies in your
//! project's `Cargo.toml`.
//!
//! ```toml
//! [dependencies]
//! flexi_logger = "0.5"
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
//! The latter is needed because flexi_logger plugs into the standard Rust logging facade given by the
//! [log crate](http://rust-lang.github.io/log/log/),
//! and you use the ```log``` macros to write log lines from your code.
//!
//! In flexi_logger's initialization, you can e.g.
//!
//! *  decide whether you want to write your logs to stderr (like with env_logger),
//!    or to a file,
//! *  configure the folder in which the log files are created,
//! *  provide the log-level-specification, i.e., the decision which log
//!    lines really should be written out, programmatically (if you don't want to
//!    use the environment variable RUST_LOG)
//! *  specify the line format for the log lines
//!
//! See function [init](fn.init.html) and structure [LogConfig](struct.LogConfig.html) for
//! a full description of all configuration options.

extern crate chrono;
extern crate glob;
extern crate log;
extern crate regex;

macro_rules! print_err {
    ($($arg:tt)*) => (
        {
            use std::io::prelude::*;
            if let Err(e) = write!(&mut ::std::io::stderr(), "{}\n", format_args!($($arg)*)) {
                panic!("Failed to write to stderr.\
                    \nOriginal error output: {}\
                    \nSecondary error writing to stderr: {}", format!($($arg)*), e);
            }
        }
    )
}


mod flexi_error;
mod flexi_logger;
mod flexi_writer;
mod formats;
mod log_options;

pub use log::{LogLevel, LogLevelFilter, LogRecord};
pub use formats::*;
pub use log_options::LogOptions;
pub use log_options::LogOptions as LogConfig;
pub use flexi_error::FlexiLoggerError;
pub use flexi_logger::{FlexiLogger, init};

/// Factory for a LogOptions object.
pub fn logger_options() -> LogOptions {
    log_options::LogOptions::new()
}
