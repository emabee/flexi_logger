use log;
use log::Record;
use file_log_writer::FileLogWriter;
use log_writer::LogWriter;

/// DefaultWriter writes logs to stderr or to a FileLogWriter, and in the latter case
/// can duplicate some messages to stdout.
pub struct DefaultWriter {
    log_to_file: bool,
    duplicate_error: bool,
    duplicate_info: bool,
    w: FileLogWriter,
}
impl LogWriter for DefaultWriter {
    fn write(&self, record: &Record) {
        if self.log_to_file {
            if self.duplicate_error && record.level() == log::Level::Error
                || self.duplicate_info
                    && (record.level() == log::Level::Error || record.level() == log::Level::Warn
                        || record.level() == log::Level::Info)
            {
                println!("{}", (self.w.config().format)(record));
            }
            self.w.write(record);
        } else {
            eprintln!("{}", (self.w.config().format)(record));
        }
    }
}
