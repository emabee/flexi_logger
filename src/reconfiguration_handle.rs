use log_specification::LogSpecification;
use primary_writer::PrimaryWriter;

use std::borrow::Borrow;
use std::sync::Arc;
use std::sync::RwLock;

/// Allows reconfiguring the logger while it is in use
/// (see [`Logger::start_reconfigurable()`](struct.Logger.html#method.start_reconfigurable) ).
///
/// # Example
///
/// Use the reconfigurability feature and build the log spec programmatically.
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
        guard
            .reconfigure(LogSpecification::parse(spec).unwrap_or_else(|_| LogSpecification::off()));
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
