use log;
use LogSpecification;
use primary_writer::PrimaryWriter;
use writers::LogWriter;

use regex::Regex;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

pub enum LogSpec {
    STATIC(LogSpecification),
    DYNAMIC(Arc<RwLock<LogSpecification>>),
}

// Does the logging in the background, is normally not used directly.
//
// This struct is only used explicitly when you want to allow supporting multiple `FlexiLogger`
// instances in a single process.
pub struct FlexiLogger {
    log_specification: LogSpec,
    primary_writer: Arc<PrimaryWriter>,
    other_writers: HashMap<String, Box<LogWriter>>,
}

/// Allows reconfiguring the logger while it is in use
/// (see [`Logger::start_reconfigurable()`](struct.Logger.html#method.start_reconfigurable) ).
///
/// # Example
///
/// The following example shows how to use the reconfigurability feature.
///
/// ```rust
/// extern crate log;
/// extern crate flexi_logger;
/// use flexi_logger::{Logger, LogSpecBuilder};
/// use log::LevelFilter;
///
/// fn main() {
///     // Build the initial log specification
///     let mut builder = LogSpecBuilder::new();  // default is LevelFilter::Off
///     builder.default(LevelFilter::Info);
///     builder.module("karl", LevelFilter::Debug);
///
///     // Initialize Logger, keep builder alive
///     let mut logger_reconf_handle = Logger::with(builder.build())
///         // your logger configuration goes here, as usual
///         .start_reconfigurable()
///         .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
///
///     // ...
///
///     // Modify builder and update the logger
///     builder.default(LevelFilter::Error);
///     builder.remove("karl");
///     builder.module("emma", LevelFilter::Trace);
///
///     logger_reconf_handle.set_new_spec(builder.build());
///
///     // ...
/// }
/// ```
pub struct ReconfigurationHandle {
    spec: Arc<RwLock<LogSpecification>>,
    primary_writer: Arc<PrimaryWriter>,
}
impl ReconfigurationHandle {
    /// Allows specifying a new LogSpecification for the current logger.
    pub fn set_new_spec(&mut self, new_spec: LogSpecification) {
        let mut guard = self.spec.write().unwrap(/* not sure if we should expose this */);
        guard.reconfigure(new_spec);
    }

    /// Allows specifying a new LogSpecification for the current logger.
    pub fn parse_new_spec(&mut self, spec: &str) {
        let mut guard = self.spec.write().unwrap(/* not sure if we should expose this */);
        guard.reconfigure(LogSpecification::parse(spec));
    }

    #[doc(hidden)]
    /// Allows checking the logs written so far to the writer
    pub fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) -> bool {
        Borrow::<PrimaryWriter>::borrow(&self.primary_writer).validate_logs(expected)
    }
}

pub fn reconfiguration_handle(
    spec: Arc<RwLock<LogSpecification>>,
    primary_writer: Arc<PrimaryWriter>,
) -> ReconfigurationHandle {
    ReconfigurationHandle {
        spec,
        primary_writer,
    }
}

impl FlexiLogger {
    pub fn new(
        log_specification: LogSpec,
        primary_writer: Arc<PrimaryWriter>,
        other_writers: HashMap<String, Box<LogWriter>>,
    ) -> FlexiLogger {
        FlexiLogger {
            log_specification,
            primary_writer,
            other_writers,
        }
    }
    // Implementation of Log::enabled() with easier testable signature
    fn fl_enabled(&self, level: log::Level, target: &str) -> bool {
        match self.log_specification {
            LogSpec::STATIC(ref ls) => ls.enabled(level, target),
            LogSpec::DYNAMIC(ref locked_ls) => {
                let guard = locked_ls.read();
                guard.as_ref()
                    .unwrap(/* not sure if we should expose this */)
                    .enabled(level, target)
            }
        }
    }
}

impl log::Log for FlexiLogger {
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        self.fl_enabled(metadata.level(), metadata.target())
    }

    fn log(&self, record: &log::Record) {
        let target = record.metadata().target();
        if target.starts_with('{') {
            let mut use_default = false;
            let targets: Vec<&str> = target[1..(target.len() - 1)].split(',').collect();
            for t in targets {
                if t == "_Default" {
                    use_default = true;
                } else {
                    match self.other_writers.get(t) {
                        None => eprintln!("bad writer spec: {}", t),
                        Some(writer) => {
                            writer.write(record);
                        }
                    }
                }
            }
            if !use_default {
                return;
            }
        }

        if !self.enabled(record.metadata()) {
            return;
        }

        // closure that we need below
        let check_text_filter = |text_filter: &Option<Regex>| {
            if let Some(filter) = text_filter.as_ref() {
                filter.is_match(&*record.args().to_string())
            } else {
                true
            }
        };

        if !match self.log_specification {
            LogSpec::STATIC(ref ls) => check_text_filter(ls.text_filter()),
            LogSpec::DYNAMIC(ref locked_ls) => {
                let guard = locked_ls.read();
                check_text_filter(
                    guard.as_ref().unwrap(/* not sure if we should expose this */).text_filter(),
                )
            }
        } {
            return;
        }

        self.primary_writer.write(record);
    }

    fn flush(&self) {
        self.primary_writer.flush();
        for writer in self.other_writers.values() {
            writer.flush();
        }
    }
}
