extern crate flexi_logger;
#[macro_use]
extern crate log;

use flexi_logger::{detailed_format, LogWriter, Logger, Record};

fn main() {
    let sec_writer = Box::new(SecWriter::new());
    let handle = Logger::with_str("info")
        .format(detailed_format)
        .log_to_file()
        .add_writer("Sec", sec_writer)
//        .add_writer("dummy", dummy_writer)
        .start_reconfigurable()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    error!(target : "{Sec}", "This is a security-relevant error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");
    handle.validate_logs(&[("ERROR", "error"), ("WARN", "warning"), ("INFO", "info")]);
}

struct SecWriter {}
impl SecWriter {
    pub fn new() -> SecWriter {
        SecWriter {}
    }
}
impl LogWriter for SecWriter {
    fn write(&self, record: &Record) {
        println!("Security-Writer: {}", (detailed_format)(record));
    }
}
