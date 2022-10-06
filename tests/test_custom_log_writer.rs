mod test_utils;

use flexi_logger::writers::LogWriter;
use flexi_logger::{default_format, DeferredNow, FormatFunction, Logger};
use log::*;
use std::sync::Mutex;
use termcolor::{Buffer, WriteColor};

const COUNT: u8 = 2;

#[test]
fn test_custom_log_writer() {
    if let Some(value) = test_utils::dispatch(COUNT) {
        work(value)
    }
}

fn work(value: u8) {
    let mut logger = Logger::try_with_str("info").unwrap();
    match value {
        0 => {
            logger = logger.log_to_writer(Box::new(CustomWriter {
                data: Mutex::new(Buffer::ansi()),
                format: default_format,
                mode: 0,
            }));
        }
        1 => {
            logger = logger.log_to_writer(Box::new(CustomWriter {
                data: Mutex::new(Buffer::ansi()),
                format: default_format,
                mode: 1,
            }));
            logger = logger.format(custom_format);
        }
        COUNT..=u8::MAX => unreachable!("asAS"),
    }
    let handle = logger
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");

    handle.validate_logs(&[
        (
            "ERROR",
            "test_custom_log_writer",
            "This is an error message",
        ),
        ("WARN", "test_custom_log_writer", "This is a warning"),
        ("INFO", "test_custom_log_writer", "This is an info message"),
    ]);
}

pub struct CustomWriter {
    data: Mutex<Buffer>,
    format: FormatFunction,
    mode: u8,
}

impl LogWriter for CustomWriter {
    fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        let mut data = self.data.lock().unwrap();
        (self.format)(&mut *data, now, record)
    }

    fn flush(&self) -> std::io::Result<()> {
        Ok(())
    }

    fn format(&mut self, format: FormatFunction) {
        self.format = format;
    }

    fn max_log_level(&self) -> log::LevelFilter {
        log::LevelFilter::Trace
    }

    fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) {
        let data = self.data.lock().unwrap();
        let expected_data = match self.mode {
            0 => expected
                .iter()
                .fold(Vec::new(), |mut acc, (level, module, message)| {
                    acc.extend(format!("{} [{}] {}", level, module, message).bytes());
                    acc
                }),
            1 => expected
                .iter()
                .fold(Vec::new(), |mut acc, (level, _module, message)| {
                    acc.extend(format!("{}: {}", level, message).bytes());
                    acc
                }),
            COUNT..=u8::MAX => {
                unreachable!("sadadsd")
            }
        };
        assert_eq!(
            String::from_utf8_lossy(data.as_slice()),
            String::from_utf8_lossy(&expected_data)
        );
    }
}

fn custom_format(
    writer: &mut dyn WriteColor,
    _now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    // Only write the message and the level, without the module
    write!(writer, "{}: {}", record.level(), &record.args())
}
