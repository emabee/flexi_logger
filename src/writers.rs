//! Contains the trait [`LogWriter`] for extending `flexi_logger`
//! with additional log writers,
//! and two concrete implementations
//! for writing to files ([`FileLogWriter`])
//! or to the syslog ([`SyslogWriter`]).
//! You can also use your own implementations of [`LogWriter`].
//!
//! Such log writers can be used in two ways:
//!
//! * You can influence to which output stream normal log messages will be written,
//!   i.e. those from normal log macro calls without explicit target specification.
//!   By default, the logs are sent to stderr. With one of the methods
//!
//!   * [`Logger::log_to_stdout`](crate::Logger::log_to_stdout)
//!   * [`Logger::log_to_file`](crate::Logger::log_to_file)
//!   * [`Logger::log_to_writer`](crate::Logger::log_to_writer)
//!   * [`Logger::log_to_file_and_writer`](crate::Logger::log_to_file_and_writer)
//!   * [`Logger::do_not_log`](crate::Logger::do_not_log)
//!
//!   you can specify a different log target. See there for more details.
//!
//!   Normal log calls will only be written to the chosen output channel if they match the current
//!   [log specification](crate::LogSpecification).
//!
//! * You can register additional log writers under a target name with
//!   [`Logger::add_writer()`](crate::Logger::add_writer), and address these log writers by
//!   specifying the target name in calls to the
//!   [log macros](https://docs.rs/log/latest/log/macro.log.html).
//!
//!   A log call with a target value that has the form `{Name1,Name2,...}`, i.e.,
//!   a comma-separated list of target names, within braces, is not sent to the default logger,
//!   but to the loggers specified explicitly in the list.
//!   In such a list you can again specify the default logger with the target name `_Default`.
//!
//!   These log calls will not be affected by the value of `flexi_logger`'s log specification;
//!   they will always be written, as you might want it for alerts or auditing.
//!
//!   In the following example we define an alert writer, and a macro to facilitate using it
//!   (and avoid using the explicit target specification in the macro call), and
//!   show some example calls.
//!
//!   ```rust
//!   use log::*;
//!
//!   use flexi_logger::{FileSpec,Logger};
//!   use flexi_logger::writers::FileLogWriter;
//!
//!   // Configure a FileLogWriter for alert messages
//!   pub fn alert_logger() -> Box<FileLogWriter> {
//!       Box::new(FileLogWriter::builder(
//!           FileSpec::default()
//!   #           .directory("log_files/writers_mod_docu")
//!               .discriminant("Alert")
//!               .suffix("alerts")
//!           )
//!           .print_message()
//!           .try_build()
//!           .unwrap())
//!   }
//!
//!   // Define a macro for writing messages to the alert log and to the normal log
//!   #[macro_use]
//!   mod macros {
//!       #[macro_export]
//!       macro_rules! alert_error {
//!           ($($arg:tt)*) => (
//!               error!(target: "{Alert,_Default}", $($arg)*);
//!           )
//!       }
//!   }
//!
//!   fn main() {
//!       Logger::try_with_env_or_str("info")
//!           .expect("LogSpecification String has errors")
//!           .print_message()
//!           .log_to_file(FileSpec::default())
//!   #       .log_to_file(FileSpec::default().directory("log_files/writers_mod_docu"))
//!           .add_writer("Alert", alert_logger())
//!           .start()
//!           .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
//!
//!
//!       // Explicitly send logs to different loggers
//!       error!(target : "{Alert}", "This is only an alert");
//!       error!(target : "{Alert,_Default}", "This is an alert and log message");
//!
//!       // Nicer: use the explicit macro
//!       alert_error!("This is another alert and log message");
//!
//!       // Standard log macros write only to the normal log
//!       error!("This is a normal error message");
//!       warn!("This is a warning");
//!       info!("This is an info message");
//!       debug!("This is a debug message - you will not see it");
//!       trace!("This is a trace message - you will not see it");
//!   }
//!
//!   ```
//!

pub(crate) mod file_log_writer;
mod log_writer;

#[cfg(feature = "syslog_writer")]
#[cfg_attr(docsrs, doc(cfg(feature = "syslog_writer")))]
mod syslog_writer;

#[cfg(feature = "syslog_writer")]
#[cfg_attr(docsrs, doc(cfg(feature = "syslog_writer")))]
pub use self::syslog_writer::{
    LevelToSyslogSeverity, Syslog, SyslogFacility, SyslogSeverity, SyslogWriter,
};

pub use self::file_log_writer::{
    ArcFileLogWriter, FileLogWriter, FileLogWriterBuilder, FileLogWriterConfig, FileLogWriterHandle,
};
pub use self::log_writer::LogWriter;
