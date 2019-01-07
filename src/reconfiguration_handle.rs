use crate::log_specification::LogSpecification;
use crate::primary_writer::PrimaryWriter;

use std::borrow::Borrow;
use std::sync::Arc;
use std::sync::RwLock;

/// Allows reconfiguring the logger programmatically.
///
/// # Example
///
/// Obtain the `ReconfigurationHandle` (using `.start_reconfigurable()` instead of `.start()`):
/// ```rust
/// # use flexi_logger::{Logger, LogSpecBuilder};
/// let mut log_handle = Logger::with_str("info")
///     // ... your logger configuration goes here, as usual
///     .start_reconfigurable()
///     .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
///
/// // ...
/// ```
///
/// You can permanently exchange the log specification programmatically:
///
/// ```rust
/// # use flexi_logger::{Logger, LogSpecBuilder};
/// # let mut log_handle = Logger::with_str("info")
/// #     .start_reconfigurable()
/// #     .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
/// log_handle.parse_new_spec("warn");
/// // ...
/// ```
///
/// However, when debugging, you often want to modify the log spec only temporarily, for  
/// one or few method calls only; this is easier done with the following method, because
/// it allows switching back to the previous spec:
///
/// ```rust
/// # use flexi_logger::{Logger, LogSpecBuilder};
/// #    let mut log_handle = Logger::with_str("info")
/// #        .start_reconfigurable()
/// #        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
/// log_handle.parse_and_push_temp_spec("trace");
/// // ...
/// // critical calls
/// // ...
///
/// log_handle.pop_temp_spec();
/// // Continue with the log spec you had before.
/// // ...
/// ```

pub struct ReconfigurationHandle {
    spec: Arc<RwLock<LogSpecification>>,
    spec_stack: Vec<LogSpecification>,
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
        guard.reconfigure(LogSpecification::parse(spec).unwrap_or_else(|e| {
            eprintln!("ReconfigurationHandle::parse_new_spec(): failed with {}", e);
            LogSpecification::off()
        }));
    }

    /// Allows temporarily pushing a new LogSpecification for the current logger.
    pub fn push_temp_spec(&mut self, new_spec: LogSpecification) {
        let mut guard = self.spec.write().unwrap(/* not sure if we should expose this */);
        self.spec_stack.push(guard.clone());
        guard.reconfigure(new_spec);
    }

    /// Allows temporarily pushing a new LogSpecification for the current logger.
    pub fn parse_and_push_temp_spec(&mut self, new_spec: &str) {
        let mut guard = self.spec.write().unwrap(/* not sure if we should expose this */);
        let new_spec = LogSpecification::parse(new_spec).unwrap_or_else(|e| {
            eprintln!(
                "ReconfigurationHandle::parse_new_spec(): failed with {}, \
                 falling back to empty log spec",
                e
            );
            LogSpecification::off()
        });
        self.spec_stack.push(guard.clone());
        guard.reconfigure(new_spec);
    }

    /// Allows pushing a new LogSpecification for the current logger.
    /// It will automatically be popped once the returned guard is dropped.
    pub fn pop_temp_spec(&mut self) {
        let mut guard = self.spec.write().unwrap(/* not sure if we should expose this */);
        if let Some(new_spec) = self.spec_stack.pop() {
            guard.reconfigure(new_spec);
        }
    }

    // Allows checking the logs written so far to the writer
    #[doc(hidden)]
    pub fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) -> bool {
        Borrow::<PrimaryWriter>::borrow(&self.primary_writer).validate_logs(expected)
    }
}

pub(crate) fn reconfiguration_handle(
    spec: Arc<RwLock<LogSpecification>>,
    primary_writer: Arc<PrimaryWriter>,
) -> ReconfigurationHandle {
    ReconfigurationHandle {
        spec,
        spec_stack: Default::default(),
        primary_writer,
    }
}
