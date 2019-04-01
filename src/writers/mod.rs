//! A trait for extending `flexi_logger` with additional log writers,
//! and two concrete implementations for writing to files or to the syslog.
//!
//! (See [`LogWriter`](trait.LogWriter.html), [`FileLogWriter`](struct.FileLogWriter.html),
//! and [`SyslogWriter`](struct.SyslogWriter.html) for details).
//!
//! Additional log writers can be used for sending log messages to additional log
//! ouput streams than stderr or the default log file, as for example to an alert file or the syslog.
//!
//! You can also use your own implementations of [`LogWriter`](trait.LogWriter.html).
//!
//! You register each additional log writer with
//! [`Logger.add_writer()`](../struct.Logger.html#method.add_writer) under a target name.
//! The target name is used subsequently in calls to the
//! [log macro](https://docs.rs/log/latest/log/macro.log.html)
//! for directing log messages to the desired writers.
//!
//! A log call with a target value that has the form `{Name1,Name2,...}`, i.e.,
//! a comma-separated list of target names, within braces, is not sent to the default logger,
//! but to the loggers specified explicitly in the list.
//! In such a list you can again specify the default logger with the target name `_Default`.
//!
//! In the following example we define an alert writer, and a macro to facilitate using it
//! (and avoid using the explicit target specification in the macro call), and
//! show some example calls.
//!
//! ```rust
//! use log::*;
//!
//! use flexi_logger::Logger;
//! use flexi_logger::writers::FileLogWriter;
//!
//! // Configure a FileLogWriter for alert messages
//! pub fn alert_logger() -> Box<FileLogWriter> {
//!     Box::new(FileLogWriter::builder()
//!         .discriminant("Alert")
//!         .suffix("alerts")
//!         .print_message()
//!         .instantiate()
//!         .unwrap())
//! }
//!
//! // Define a macro for writing messages to the alert log and to the normal log
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
//!     Logger::with_env_or_str("info")
//!         .print_message()
//!         .log_to_file()
//!         .add_writer("Alert", alert_logger())
//!         .start()
//!         .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
//!
//!
//!     // Explicitly send logs to different loggers
//!     error!(target : "{Alert}", "This is only an alert");
//!     error!(target : "{Alert,_Default}", "This is an alert and log message");
//!
//!     // Nicer: use the explicit macro
//!     alert_error!("This is another alert and log message");
//!
//!     // Standard log macros write only to the normal log
//!     error!("This is a normal error message");
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

#[cfg(feature = "syslog_writer")]
mod syslog_writer;

#[cfg(feature = "syslog_writer")]
pub use self::syslog_writer::{
    LevelToSyslogSeverity, SyslogConnector, SyslogFacility, SyslogSeverity, SyslogWriter,
};

pub use self::file_log_writer::{FileLogWriter, FileLogWriterBuilder};
pub use self::log_writer::LogWriter;
