use flexi_error::FlexiLoggerError;
use log_config::LogConfig;
use log;
use log::Record;
use writers::FileLogWriter;
use writers::LogWriter;

/// `DefaultLogWriter` writes logs to stderr or to a `FileLogWriter`, and in the latter case
/// can duplicate some messages to stdout.
pub struct DefaultLogWriter {
    log_to_file: bool,
    duplicate_error: bool,
    duplicate_info: bool,
    w: FileLogWriter,
}
impl DefaultLogWriter {
    pub fn new(config: LogConfig) -> Result<DefaultLogWriter, FlexiLoggerError> {
        let mut builder = FileLogWriter::builder().format(config.format);
        if config.print_message {
            builder = builder.print_message();
        }
        if let Some(suffix) = config.suffix {
            builder = builder.suffix(suffix);
        };
        if let Some(discriminant) = config.discriminant {
            builder = builder.discriminant(discriminant);
        }
        if let Some(directory) = config.directory {
            builder = builder.directory(directory);
        }
        if !config.timestamp {
            builder = builder.suppress_timestamp();
        }
        if let Some(rotate_over_size) = config.rotate_over_size {
            builder = builder.rotate_over_size(rotate_over_size);
        };
        if let Some(create_symlink) = config.create_symlink {
            builder = builder.create_symlink(create_symlink);
        };

        Ok(DefaultLogWriter {
            log_to_file: config.log_to_file,
            duplicate_error: config.duplicate_error,
            duplicate_info: config.duplicate_info,
            w: builder.instantiate()?,
        })
    }

    #[doc(hidden)]
    pub fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) -> bool {
        self.w.validate_logs(expected)
    }
}
impl LogWriter for DefaultLogWriter {
    fn write(&self, record: &Record) {
        if self.log_to_file {
            if self.duplicate_error && record.level() == log::Level::Error
                || self.duplicate_info
                    && (record.level() == log::Level::Error || record.level() == log::Level::Warn
                        || record.level() == log::Level::Info)
            {
                println!("{}", (self.w.format())(record));
            }
            self.w.write(record);
        } else {
            eprintln!("{}", (self.w.format())(record));
        }
    }

    fn flush(&self) {
        self.w.flush();
    }
}
