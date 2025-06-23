use super::{
    connection::Connection, line::LineWriter, LevelToSyslogSeverity, SyslogConnection,
    SyslogFacility, SyslogLineHeader, SyslogWriterBuilder,
};
use crate::{writers::log_writer::LogWriter, DeferredNow, FormatFunction};
#[cfg(test)]
use std::io::BufRead;
use std::{
    io::{Result as IoResult, Write},
    sync::Mutex,
};

/// A configurable [`LogWriter`] implementation that writes log messages to the syslog.
///
/// Only available with optional crate feature `syslog_writer`.
///
/// See the [writers](crate::writers) module for guidance how to use additional log writers.
#[allow(clippy::module_name_repetitions)]
pub struct SyslogWriter {
    line_writer: LineWriter,
    m_conn_state: SyslogConnectionState,
    max_log_level: log::LevelFilter,
    #[cfg(test)]
    validation_buffer: Mutex<std::io::Cursor<Vec<u8>>>,
}
impl SyslogWriter {
    /// Instantiate the builder for the `SysLogWriter`.
    #[must_use]
    pub fn builder(
        syslog: SyslogConnection,
        syslog_line_header: SyslogLineHeader,
        syslog_facility: SyslogFacility,
    ) -> SyslogWriterBuilder {
        SyslogWriterBuilder::new(syslog, syslog_line_header, syslog_facility)
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn new(
        pid: u32,
        process: String,
        syslog_line_header: SyslogLineHeader,
        facility: SyslogFacility,
        determine_severity: LevelToSyslogSeverity,
        syslog_connection: SyslogConnection,
        max_log_level: log::LevelFilter,
        format: FormatFunction,
    ) -> IoResult<SyslogWriter> {
        Ok(SyslogWriter {
            line_writer: LineWriter::new(
                syslog_line_header,
                determine_severity,
                facility,
                process,
                pid,
                format,
            )?,
            m_conn_state: match syslog_connection.into_inner() {
                #[cfg(unix)]
                Connection::SyslogCall => SyslogConnectionState::SyslogCall,
                conn => SyslogConnectionState::SocketConnection(Mutex::new(ConnectorAndBuffer {
                    conn,
                    buf: Vec::with_capacity(200),
                })),
            },
            max_log_level,
            #[cfg(test)]
            validation_buffer: Mutex::new(std::io::Cursor::new(Vec::new())),
        })
    }
}
impl LogWriter for SyslogWriter {
    fn write(&self, now: &mut DeferredNow, record: &log::Record) -> IoResult<()> {
        if record.level() > self.max_log_level {
            return Ok(());
        }

        match &self.m_conn_state {
            SyslogConnectionState::SocketConnection(conn_state) => {
                let mut conn_state = conn_state
                    .lock()
                    .map_err(|_| crate::util::io_err("SyslogWriter is poisoned"))?;

                conn_state.buf.clear();

                self.line_writer
                    .write_to_syslog_socket_buffer(&mut conn_state.buf, now, record)?;

                #[cfg(test)]
                {
                    let mut valbuf = self.validation_buffer.lock().unwrap();
                    valbuf.write_all(&conn_state.buf)?;
                    valbuf.write_all(b"\n")?;
                }

                // we _have_ to buffer above because each write here generates a syslog entry
                let conn_state = &mut *conn_state;
                conn_state.conn.write_all(&conn_state.buf)
            }
            #[cfg(unix)]
            SyslogConnectionState::SyslogCall => {
                self.line_writer.write_with_syslog_call(now, record)
            }
        }
    }

    fn flush(&self) -> IoResult<()> {
        match &self.m_conn_state {
            SyslogConnectionState::SocketConnection(conn_state) => conn_state
                .lock()
                .map_err(|_| crate::util::io_err("SyslogWriter is poisoned"))?
                .conn
                .flush(),
            #[cfg(unix)]
            SyslogConnectionState::SyslogCall => Ok(()),
        }
    }

    fn max_log_level(&self) -> log::LevelFilter {
        self.max_log_level
    }

    fn shutdown(&self) {
        self.line_writer.shutdown();
    }

    #[doc(hidden)]
    fn validate_logs(&self, _expected: &[(&'static str, &'static str, &'static str)]) {
        #[cfg(test)]
        {
            let write_cursor = self.validation_buffer.lock().unwrap();
            let mut reader = std::io::BufReader::new(&**write_cursor.get_ref());
            let mut buf = String::new();
            #[allow(clippy::used_underscore_binding)]
            for tuple in _expected {
                buf.clear();
                reader.read_line(&mut buf).unwrap();
                assert!(
                    buf.contains(tuple.0),
                    "Did not find tuple.0 = {} in {}",
                    tuple.0,
                    buf
                );
                assert!(buf.contains(tuple.1), "Did not find tuple.1 = {}", tuple.1);
                assert!(buf.contains(tuple.2), "Did not find tuple.2 = {}", tuple.2);
            }
            buf.clear();
            reader.read_line(&mut buf).unwrap();
            assert!(buf.is_empty(), "Found more log lines than expected: {buf} ",);
        }
    }
}

struct ConnectorAndBuffer {
    conn: Connection,
    buf: Vec<u8>,
}

enum SyslogConnectionState {
    SocketConnection(Mutex<ConnectorAndBuffer>),
    #[cfg(unix)]
    SyslogCall,
}

/////////////////////////////

#[cfg(test)]
mod test {

    use crate::{
        detailed_format,
        writers::{
            syslog_format_with_thread, SyslogConnection, SyslogFacility, SyslogLineHeader,
            SyslogWriter,
        },
        FileSpec, Logger,
    };
    use chrono::{DateTime, Local};
    use log::*;
    use std::path::PathBuf;

    #[doc(hidden)]
    #[macro_use]
    mod macros {
        #[macro_export]
        macro_rules! syslog1 {
            ($($arg:tt)*) => (
                error!(target: "{Syslog1,_Default}", $($arg)*);
            )
        }
        #[macro_export]
        macro_rules! syslog2 {
            ($($arg:tt)*) => (
                error!(target: "{Syslog2,_Default}", $($arg)*);
            )
        }
    }

    #[test]
    fn test_syslog() {
        let boxed_syslog_writer1 = SyslogWriter::builder(
            SyslogConnection::try_udp("127.0.0.1:5555", "127.0.0.1:514").unwrap(),
            SyslogLineHeader::Rfc5424("JustForTest".to_owned()),
            SyslogFacility::LocalUse0,
        )
        .max_log_level(log::LevelFilter::Trace)
        .build()
        .unwrap();

        let boxed_syslog_writer2 = SyslogWriter::builder(
            SyslogConnection::try_udp("127.0.0.1:5556", "127.0.0.1:514").unwrap(),
            SyslogLineHeader::Rfc3164,
            SyslogFacility::LocalUse0,
        )
        .max_log_level(log::LevelFilter::Trace)
        .format(syslog_format_with_thread)
        .build()
        .unwrap();

        let logger = Logger::try_with_str("info")
            .unwrap()
            .format(detailed_format)
            .log_to_file(FileSpec::default().suppress_timestamp().directory(dir()))
            .print_message()
            .add_writer("Syslog1", boxed_syslog_writer1)
            .add_writer("Syslog2", boxed_syslog_writer2)
            .start()
            .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

        // Explicitly send logs to different loggers
        error!(target : "{Syslog1}", "This is a syslog-relevant error msg");
        warn!(target : "{Syslog1}", "This is a syslog-relevant warn msg");
        info!(target : "{Syslog1}", "This is a syslog-relevant info msg");
        debug!(target : "{Syslog1}", "This is a syslog-relevant debug msg");
        trace!(target : "{Syslog1}", "This is a syslog-relevant trace msg");

        error!(target : "{Syslog1,_Default}", "This is a syslog- and log-relevant msg");

        error!(target : "{Syslog2}", "This is a syslog-relevant error msg");
        warn!(target : "{Syslog2}", "This is a syslog-relevant warn msg");
        info!(target : "{Syslog2}", "This is a syslog-relevant info msg");
        debug!(target : "{Syslog2}", "This is a syslog-relevant debug msg");
        trace!(target : "{Syslog2}", "This is a syslog-relevant trace msg");

        error!(target : "{Syslog2,_Default}", "This is a syslog- and log-relevant msg");

        // Nicer: use explicit macros
        syslog1!("This is another syslog- and log error msg");
        syslog2!("This is one more syslog- and log error msg");
        warn!("This is a warning message");
        debug!("This is a debug message - you must not see it!");
        trace!("This is a trace message - you must not see it!");

        // Verification:
        // this only validates the normal log target (file)
        logger.validate_logs(&[
            ("ERROR", "", "a syslog- and log-relevant msg"),
            ("ERROR", "", "a syslog- and log-relevant msg"),
            ("ERROR", "", "another syslog- and log error msg"),
            ("ERROR", "", "one more syslog- and log error msg"),
            ("WARN", "syslog", "This is a warning message"),
        ]);
        logger.validate_additional_logs(
            "Syslog1",
            &[
                ("<131>1", "JustForTest", "is a syslog-relevant error msg"),
                ("<132>1", "JustForTest", "is a syslog-relevant warn msg"),
                ("<134>1", "JustForTest", "is a syslog-relevant info msg"),
                ("<135>1", "JustForTest", "is a syslog-relevant debug msg"),
                ("<135>1", "JustForTest", "is a syslog-relevant trace msg"),
                ("<131>1", "JustForTest", "is a syslog- and log-relevant msg"),
                ("<131>1", "JustForTest", "is another syslog- and log error"),
            ],
        );
        logger.validate_additional_logs(
            "Syslog2",
            &[
                ("<131>", "]: [", "This is a syslog-relevant error msg"),
                ("<132>", "]: [", "This is a syslog-relevant warn msg"),
                ("<134>", "]: [", "This is a syslog-relevant info msg"),
                ("<135>", "]: [", "This is a syslog-relevant debug msg"),
                ("<135>", "]: [", "This is a syslog-relevant trace msg"),
                ("<131>", "]: [", "This is a syslog- and log-relevant msg"),
                ("<131>", "]: [", "This is one more syslog- and log error"),
            ],
        );
    }

    fn dir() -> PathBuf {
        let mut d = PathBuf::new();
        d.push("log_files");
        add_prog_name(&mut d);
        d.push(now_local().format(TS).to_string());
        d
    }
    fn add_prog_name(pb: &mut PathBuf) {
        let path = PathBuf::from(std::env::args().next().unwrap());
        let filename = path.file_stem().unwrap(/*ok*/).to_string_lossy();
        let (progname, _) = filename.rsplit_once('-').unwrap_or((&filename, ""));
        pb.push(progname);
    }
    #[must_use]
    pub fn now_local() -> DateTime<Local> {
        Local::now()
    }
    const TS: &str = "%Y-%m-%d_%H-%M-%S";
}
