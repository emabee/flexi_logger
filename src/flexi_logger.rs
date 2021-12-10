use crate::filter::LogLineFilter;
use crate::primary_writer::PrimaryWriter;
use crate::util::{eprint_err, eprint_msg, ERRCODE};
use crate::writers::LogWriter;
use crate::LogSpecification;

#[cfg(feature = "textfilter")]
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, RwLock};

// Implements log::Log to plug into the log crate.
//
// Delegates the real logging to the configured PrimaryWriter and optionally to other writers.
// The `PrimaryWriter` is either a `StdErrWriter` or an `ExtendedFileWriter`.
// An ExtendedFileWriter logs to a file, by delegating to a FileWriter,
// and can additionally duplicate log lines to stderr.
pub(crate) struct FlexiLogger {
    log_specification: Arc<RwLock<LogSpecification>>,
    primary_writer: Arc<PrimaryWriter>,
    other_writers: Arc<HashMap<String, Box<dyn LogWriter>>>,
    filter: Option<Box<dyn LogLineFilter + Send + Sync>>,
    use_utc: bool,
}

impl FlexiLogger {
    pub fn new(
        log_specification: Arc<RwLock<LogSpecification>>,
        primary_writer: Arc<PrimaryWriter>,
        other_writers: Arc<HashMap<String, Box<dyn LogWriter>>>,
        filter: Option<Box<dyn LogLineFilter + Send + Sync>>,
        use_utc: bool,
    ) -> Self {
        Self {
            log_specification,
            primary_writer,
            other_writers,
            filter,
            use_utc,
        }
    }

    fn primary_enabled(&self, level: log::Level, module: &str) -> bool {
        self.log_specification
            .read()
            .map_err(|e| eprint_err(ERRCODE::Poison, "rwlock on log spec is poisoned", &e))
            .unwrap()
            .enabled(level, module)
    }
}

impl log::Log for FlexiLogger {
    //  If other writers are configured and the metadata target addresses them correctly,
    //      - we should determine if the metadata-level is digested by any of the writers
    //        (including the primary writer)
    //  else we fall back to default behavior:
    //      Return true if
    //      - target is filled with module path and level is accepted by log specification
    //      - target is filled with crap and ???
    //
    // Caveat:
    // Rocket e.g. sets target explicitly to several fantasy names;
    // these hopefully do not collide with any of the modules in the log specification;
    // since they do not conform with the {} syntax expected by flexi_logger, they're treated as
    // module names.
    fn enabled(&self, metadata: &log::Metadata) -> bool {
        let target = metadata.target();
        let level = metadata.level();

        if !self.other_writers.is_empty() && target.starts_with('{') {
            // at least one other writer is configured _and_ addressed
            let targets: Vec<&str> = target[1..(target.len() - 1)].split(',').collect();
            for t in targets {
                if t != "_Default" {
                    match self.other_writers.get(t) {
                        None => eprint_msg(ERRCODE::WriterSpec, &format!("bad writer spec: {}", t)),
                        Some(writer) => {
                            if level < writer.max_log_level() {
                                return true;
                            }
                        }
                    }
                }
            }
        }

        self.primary_enabled(level, target)
    }

    fn log(&self, record: &log::Record) {
        let target = record.metadata().target();
        let mut now = crate::DeferredNow::new(self.use_utc);
        let special_target_is_used = target.starts_with('{');
        if special_target_is_used {
            let mut use_default = false;
            let targets: Vec<&str> = target[1..(target.len() - 1)].split(',').collect();
            for t in targets {
                if t == "_Default" {
                    use_default = true;
                } else {
                    match self.other_writers.get(t) {
                        None => eprint_msg(ERRCODE::WriterSpec, &format!("bad writer spec: {}", t)),
                        Some(writer) => {
                            writer.write(&mut now, record).unwrap_or_else(|e| {
                                eprint_err(
                                    ERRCODE::Write,
                                    &format!("writing log line to custom writer \"{}\" failed", t),
                                    &e,
                                );
                            });
                        }
                    }
                }
            }
            if !use_default {
                return;
            }
        }

        let effective_target = if special_target_is_used {
            record.module_path().unwrap_or_default()
        } else {
            target
        };
        if !self.primary_enabled(record.level(), effective_target) {
            return;
        }

        #[cfg(feature = "textfilter")]
        {
            // closure that we need below
            let check_text_filter = |text_filter: Option<&Regex>| {
                text_filter.map_or(true, |filter| filter.is_match(&*record.args().to_string()))
            };

            if !check_text_filter(
                self.log_specification.read().as_ref().unwrap(/* expose this? */).text_filter(),
            ) {
                return;
            }
        }

        #[allow(clippy::option_if_let_else)]
        if let Some(ref filter) = self.filter {
            filter.write(&mut now, record, &(*self.primary_writer))
        } else {
            self.primary_writer.write(&mut now, record)
        }
        .unwrap_or_else(|e| {
            eprint_err(ERRCODE::Write, "writing log line failed", &e);
        });
    }

    fn flush(&self) {
        self.primary_writer.flush().unwrap_or_else(|e| {
            eprint_err(ERRCODE::Flush, "flushing primary writer failed", &e);
        });
        for writer in self.other_writers.values() {
            writer.flush().unwrap_or_else(|e| {
                eprint_err(ERRCODE::Flush, "flushing custom writer failed", &e);
            });
        }
    }
}
