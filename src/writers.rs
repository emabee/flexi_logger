//! Describes how to extend `flexi_logger` with additional log writers
//! (implementations of the trait [`LogWriter`]), and contains two ready-to-use log writers,
//! one for writing to files ([`FileLogWriter`]), one for writing to the syslog ([`SyslogWriter`]).
//!
//! Log writers can be used in two ways:
//!
//! * _Default output channel:_ <br>
//!   You can influence to which output stream normal log messages will be written,
//!   i.e. those from log macro calls without explicit target specification
//!   (like in `log::error!("File not found")`).
//!
//!   With one of the methods
//!
//!   * [`Logger::log_to_stderr`](crate::Logger::log_to_stderr) (default)
//!   * [`Logger::log_to_stdout`](crate::Logger::log_to_stdout)
//!   * [`Logger::log_to_file`](crate::Logger::log_to_file)
//!   * [`Logger::log_to_writer`](crate::Logger::log_to_writer)
//!   * [`Logger::log_to_file_and_writer`](crate::Logger::log_to_file_and_writer)
//!   * [`Logger::do_not_log`](crate::Logger::do_not_log)
//!
//!   you can change the default output channel. The fourth and the fifth of these methods
//!   take log writers as input. See their documentation for more details.
//!
//!   Messages will only be written to the default output channel
//!   if they match the current [log specification](crate::LogSpecification).
//!
//!   <br>
//!
//! * _Additional output channels:_ <br>
//!   You can register additional log writers under a _target name_ with
//!   [`Logger::add_writer()`](crate::Logger::add_writer), and address these log writers by
//!   specifying the _target name_ in calls to the
//!   [log macros](https://docs.rs/log/latest/log/macro.log.html).
//!
//!   The message of a log call with a _target value_ that has the form `{Name1,Name2,...}`, i.e.,
//!   a comma-separated list of _target names_, within braces, is not sent to the default output
//!   channel, but to the loggers specified explicitly in the list. In such a list
//!   you can also specify the default output channel with the built-in target name `_Default`.
//!
//!   Log calls that are directed to an additional output channel will not be affected by
//!   the value of `flexi_logger`'s log specification;
//!   they will always be handed over to the respective `LogWriter`,
//!   as you might want it for alerts or auditing.
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
mod syslog;

#[cfg(feature = "syslog_writer")]
#[cfg_attr(docsrs, doc(cfg(feature = "syslog_writer")))]
pub use self::syslog::{
    syslog_default_format, syslog_format_with_thread, LevelToSyslogSeverity, SyslogConnection,
    SyslogFacility, SyslogLineHeader, SyslogSeverity, SyslogWriter, SyslogWriterBuilder,
};

pub use self::file_log_writer::{
    ArcFileLogWriter, FileLogWriter, FileLogWriterBuilder, FileLogWriterConfig, FileLogWriterHandle,
};
pub use self::log_writer::LogWriter;
