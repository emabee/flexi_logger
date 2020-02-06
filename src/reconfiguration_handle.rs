#[cfg(feature = "specfile")]
use crate::flexi_error::FlexiLoggerError;
use crate::log_specification::LogSpecification;
use crate::primary_writer::PrimaryWriter;
use crate::writers::LogWriter;
use std::borrow::Borrow;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

/// Allows reconfiguring the logger programmatically.
///
/// # Example
///
/// Obtain the `ReconfigurationHandle` (using `.start()`):
/// ```rust
/// # use flexi_logger::{Logger, LogSpecBuilder};
/// let mut log_handle = Logger::with_str("info")
///     // ... your logger configuration goes here, as usual
///     .start()
///     .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
///
/// // ...
/// ```
///
/// You can permanently exchange the log specification programmatically, anywhere in your code:
///
/// ```rust
/// # use flexi_logger::{Logger, LogSpecBuilder};
/// # let mut log_handle = Logger::with_str("info")
/// #     .start()
/// #     .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
/// // ...
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
/// #        .start()
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
#[derive(Clone)]
pub struct ReconfigurationHandle {
    spec: Arc<RwLock<LogSpecification>>,
    spec_stack: Vec<LogSpecification>,
    primary_writer: Arc<PrimaryWriter>,
    other_writers: Arc<HashMap<String, Box<dyn LogWriter>>>,
}
impl ReconfigurationHandle {
    pub(crate) fn new(
        spec: Arc<RwLock<LogSpecification>>,
        primary_writer: Arc<PrimaryWriter>,
        other_writers: Arc<HashMap<String, Box<dyn LogWriter>>>,
    ) -> Self {
        Self {
            spec,
            spec_stack: Vec::default(),
            primary_writer,
            other_writers,
        }
    }

    #[cfg(feature = "specfile")]
    pub(crate) fn current_spec(&self) -> Arc<RwLock<LogSpecification>> {
        Arc::clone(&self.spec)
    }

    // If the specfile exists, read the file and update the log_spec from it;
    // otherwise try to create the file, with the current spec as content, under the specified name.
    #[cfg(feature = "specfile")]
    pub(crate) fn synchronize_with_specfile(
        &mut self,
        specfile: &std::path::PathBuf,
    ) -> Result<(), FlexiLoggerError> {
        if specfile
            .extension()
            .unwrap_or_else(|| std::ffi::OsStr::new(""))
            .to_str()
            .unwrap_or("")
            != "toml"
        {
            return Err(FlexiLoggerError::Parse(
                vec!["[flexi_logger] only spec files with suffix toml are supported".to_owned()],
                LogSpecification::off(),
            ));
        }

        if std::path::Path::is_file(specfile) {
            self.set_new_spec(LogSpecification::try_from_file(&specfile).map_err(|e| {
                eprintln!(
                    "[flexi_logger] reading the log specification file failed with {:?}",
                    e
                );
                e
            })?);
            Ok(())
        } else {
            if let Some(specfolder) = specfile.parent() {
                std::fs::DirBuilder::new()
                    .recursive(true)
                    .create(specfolder)
                    .map_err(|e| {
                        eprintln!(
                            "[flexi_logger] cannot create the folder for the logspec file \
                             under the specified name {:?}, caused by: {}",
                            &specfile, e
                        );
                        e
                    })?;
            }

            let mut file = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(specfile)
                .map_err(|e| {
                    eprintln!(
                        "[flexi_logger] cannot create an initial logspec file \
                         under the specified name {:?}, caused by: {}",
                        &specfile, e
                    );
                    e
                })?;
            self.current_spec()
                .read()
                .map_err(|_e| FlexiLoggerError::Poison)?
                .to_toml(&mut file)?;
            Ok(())
        }
    }

    //
    pub(crate) fn reconfigure(&self, mut max_level: log::LevelFilter) {
        for w in self.other_writers.as_ref().values() {
            max_level = std::cmp::max(max_level, w.max_log_level());
        }
        log::set_max_level(max_level);
    }

    /// Replaces the active `LogSpecification`.
    pub fn set_new_spec(&mut self, new_spec: LogSpecification) {
        let max_level = new_spec.max_level();
        self.spec.write().unwrap(/* catch and expose error? */).update_from(new_spec);
        self.reconfigure(max_level);
    }

    /// Tries to replace the active `LogSpecification` with the result from parsing the given String.
    pub fn parse_new_spec(&mut self, spec: &str) {
        self.set_new_spec(LogSpecification::parse(spec).unwrap_or_else(|e| {
            eprintln!(
                "[flexi_logger] ReconfigurationHandle::parse_new_spec(): failed with {}",
                e
            );
            LogSpecification::off()
        }))
    }

    /// Replaces the active `LogSpecification` and pushes the previous one to a Stack.
    pub fn push_temp_spec(&mut self, new_spec: LogSpecification) {
        self.spec_stack
            .push(self.spec.read().unwrap(/* catch and expose error? */).clone());
        self.set_new_spec(new_spec);
    }

    /// Tries to replace the active `LogSpecification` with the result from parsing the given String
    ///  and pushes the previous one to a Stack.
    pub fn parse_and_push_temp_spec(&mut self, new_spec: &str) {
        self.spec_stack
            .push(self.spec.read().unwrap(/* catch and expose error? */).clone());
        self.set_new_spec(LogSpecification::parse(new_spec).unwrap_or_else(|e| {
            eprintln!(
                "[flexi_logger] ReconfigurationHandle::parse_new_spec(): failed with {}, \
                 falling back to empty log spec",
                e
            );
            LogSpecification::off()
        }));
    }

    /// Reverts to the previous `LogSpecification`, if any.
    pub fn pop_temp_spec(&mut self) {
        if let Some(previous_spec) = self.spec_stack.pop() {
            self.set_new_spec(previous_spec);
        }
    }

    /// Shutdown all participating writers.
    ///
    /// This method is supposed to be called at the very end of your program, in case you use
    /// your own writers, or if you want to securely shutdown the cleanup-thread of the
    /// `FileLogWriter`. If you use a [`Cleanup`](enum.Cleanup.html) strategy with zipping,
    /// and your process terminates
    /// without correctly shutting down the cleanup-thread, then you might stop the cleanup-thread
    /// while it is zipping a log file, which can leave unexpected files in the filesystem.
    ///
    /// See also [`LogWriter::shutdown`](writers/trait.LogWriter.html#method.shutdown).
    pub fn shutdown(&self) {
        if let PrimaryWriter::MultiWriter(writer) = &*self.primary_writer {
            writer.shutdown();
        }
        for writer in self.other_writers.values() {
            writer.shutdown();
        }
    }

    // Allows checking the logs written so far to the writer
    #[doc(hidden)]
    pub fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) {
        Borrow::<PrimaryWriter>::borrow(&self.primary_writer).validate_logs(expected)
    }
}
