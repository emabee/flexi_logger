use crate::primary_writer::PrimaryWriter;
use crate::writers::LogWriter;
use crate::LogSpecification;

use log;
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
    other_writers: HashMap<String, Box<LogWriter>>,
}

impl FlexiLogger {
    pub fn new(
        log_specification: Arc<RwLock<LogSpecification>>,
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
        let guard = self.log_specification.read();
        guard.as_ref()
                    .unwrap(/* not sure if we should expose this */)
                    .enabled(level, target)
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
                            writer.write(record).unwrap_or_else(|e| {
                                eprintln!(
                                    "FlexiLogger: writing log line to custom_writer failed with {}",
                                    e
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

        if !check_text_filter(
            self.log_specification.read().as_ref().unwrap(/* expose this? */).text_filter(),
        ) {
            return;
        }

        self.primary_writer.write(record).unwrap_or_else(|e| {
            eprintln!(
                "FlexiLogger: writing log line to primary_writer failed with {}",
                e
            );
        });
    }

    fn flush(&self) {
        self.primary_writer.flush().unwrap_or_else(|e| {
            eprintln!("FlexiLogger: flushing primary_writer failed with {}", e);
        });
        for writer in self.other_writers.values() {
            writer.flush().unwrap_or_else(|e| {
                eprintln!("FlexiLogger: flushing custom_writer failed with {}", e);
            });
        }
    }
}
