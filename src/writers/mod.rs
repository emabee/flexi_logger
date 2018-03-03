//! This module contains a trait for additional log writers,
//! and a configurable concrete implementation
//! for a log writer that writes to a file or a series of files.
//!
//! Additional log writers can be used to send log messages to other log
//! ouput streams than the default log file, as for example an alert file or the syslog.
//!
//! You register each additional log writer with
//! [`Logger.add_writer()`](../struct.Logger.html#method.add_writer) under a target name.
//! The target name is used subsequently in calls to the log macros for directing log
//! messages to the desired writer(s).
//!
//! A log call with a target value that has the form `{Name1,Name2,...}`, i.e.,
//! a comma-separated list of target names, within braces, is not sent to the default logger,
//! but to the loggers specified explicitly in the list.
//! In such a list you can again specify the default logger with the target name `_Default`.
//!
//! In the following example we define an alert writer, and a macro to facilitate using it, and
//! show some example calls.
//!
//! ```rust
//! extern crate flexi_logger;
//! #[macro_use]
//! extern crate log;
//!
//! use flexi_logger::Logger;
//! use flexi_logger::writers::FileLogWriter;
//!
//! pub fn alert_logger() -> Box<FileLogWriter> {
//!     Box::new(FileLogWriter::builder()
//!         .discriminant("Alert")
//!         .suffix("alerts")
//!         .print_message()
//!         .instantiate()
//!         .unwrap())
//! }
//!
//! #[macro_use]
//! mod macros {
//!     #[macro_export]
//!     macro_rules! alert_error {
//!         ($($arg:tt)*) => (
//!             error!(target: "{Alert,_Default}", $($arg)*);
//!         )
//!     }
//! }
//!
//! fn main() {
//!     // Write logs to the normal logging file, and alerts to a separate file
//!     Logger::with_env_or_str("info")
//!         .print_message()
//!         .log_to_file()
//!         .add_writer("Alert", alert_logger())
//!         .start()
//!         .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
//!
//!     error!("This is a normal error message");
//!
//!     // Explicitly send logs to different loggers
//!     error!(target : "{Alert}", "This is only an alert");
//!     error!(target : "{Alert,_Default}", "This is an alert and log message");
//!
//!     // Nicer: use a explicit macro
//!     alert_error!("This is another alert and log message");
//!
//!     warn!("This is a warning");
//!     info!("This is an info message");
//!     debug!("This is a debug message - you will not see it");
//!     trace!("This is a trace message - you will not see it");
//! }
//!
//! ```
//!

mod file_log_writer;
mod log_writer;

pub use self::log_writer::LogWriter;
pub use self::file_log_writer::{FileLogWriter, FileLogWriterBuilder};
