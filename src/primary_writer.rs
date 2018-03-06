use flexi_error::FlexiLoggerError;
use log_config::LogConfig;
use log;
use log::Record;
use writers::FileLogWriter;
use writers::LogWriter;

// Writes either to stderr or to a file.
#[allow(unknown_lints)]
#[allow(large_enum_variant)]
pub enum PrimaryWriter {
    EnvWriter(EnvWriter),
    ExtendedFileWriter(ExtendedFileWriter),
}
impl PrimaryWriter {
    // Factory method
    pub fn new(config: LogConfig) -> Result<PrimaryWriter, FlexiLoggerError> {
        if config.log_to_file {
            Ok(PrimaryWriter::ExtendedFileWriter(
                ExtendedFileWriter::new(config)?,
            ))
        } else {
            Ok(PrimaryWriter::EnvWriter(EnvWriter::new(&config)))
        }
    }

    // write out a log line
    pub fn write(&self, record: &Record) {
        match *self {
            PrimaryWriter::EnvWriter(ref w) => w.write(record),
            PrimaryWriter::ExtendedFileWriter(ref w) => w.write(record),
        }
    }

    // Flushes any buffered records.
    pub fn flush(&self) {
        match *self {
            PrimaryWriter::EnvWriter(ref w) => w.flush(),
            PrimaryWriter::ExtendedFileWriter(ref w) => w.flush(),
        }
    }

    pub fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) -> bool {
        match *self {
            PrimaryWriter::EnvWriter(_) => false,
            PrimaryWriter::ExtendedFileWriter(ref w) => w.validate_logs(expected),
        }
    }
}

/// `EnvWriter` writes logs to stderr.
pub struct EnvWriter {
    format: fn(&Record) -> String,
}

impl EnvWriter {
    fn new(config: &LogConfig) -> EnvWriter {
        EnvWriter {
            format: config.format,
        }
    }
    fn write(&self, record: &Record) {
        eprintln!("{}", (self.format)(record));
    }

    fn flush(&self) {}
}

/// `ExtendedFileWriter` writes logs to stderr or to a `FileLogWriter`, and in the latter case
/// can duplicate some messages to stdout.
pub struct ExtendedFileWriter {
    duplicate_error: bool,
    duplicate_info: bool,
    w: FileLogWriter,
}
impl ExtendedFileWriter {
    pub fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) -> bool {
        self.w.validate_logs(expected)
    }

    fn new(config: LogConfig) -> Result<ExtendedFileWriter, FlexiLoggerError> {
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

        Ok(ExtendedFileWriter {
            duplicate_error: config.duplicate_error,
            duplicate_info: config.duplicate_info,
            w: builder.instantiate()?,
        })
    }

    fn write(&self, record: &Record) {
        if self.duplicate_error && record.level() == log::Level::Error
            || self.duplicate_info
                && (record.level() == log::Level::Error || record.level() == log::Level::Warn
                    || record.level() == log::Level::Info)
        {
            println!("{}", (self.w.format())(record));
        }
        self.w.write(record);
    }

    fn flush(&self) {
        self.w.flush();
    }
}
