use crate::{writers::log_writer::LogWriter, DeferredNow};
use std::io::{Error as IoError, ErrorKind, Result as IoResult, Write};
use std::net::{TcpStream, ToSocketAddrs, UdpSocket};
#[cfg(target_family = "unix")]
use std::path::Path;
use std::sync::Mutex;

/// Syslog Facility.
///
/// See [RFC 5424](https://datatracker.ietf.org/doc/rfc5424).
#[derive(Copy, Clone, Debug)]
pub enum SyslogFacility {
    /// kernel messages.
    Kernel = 0 << 3,
    /// user-level messages.
    UserLevel = 1 << 3,
    /// mail system.
    MailSystem = 2 << 3,
    /// system daemons.
    SystemDaemons = 3 << 3,
    /// security/authorization messages.
    Authorization = 4 << 3,
    /// messages generated internally by syslogd.
    SyslogD = 5 << 3,
    /// line printer subsystem.
    LinePrinter = 6 << 3,
    /// network news subsystem.
    News = 7 << 3,
    /// UUCP subsystem.
    Uucp = 8 << 3,
    /// clock daemon.
    Clock = 9 << 3,
    /// security/authorization messages.
    Authorization2 = 10 << 3,
    /// FTP daemon.
    Ftp = 11 << 3,
    /// NTP subsystem.
    Ntp = 12 << 3,
    /// log audit.
    LogAudit = 13 << 3,
    /// log alert.
    LogAlert = 14 << 3,
    /// clock daemon (note 2).
    Clock2 = 15 << 3,
    /// local use 0  (local0).
    LocalUse0 = 16 << 3,
    /// local use 1  (local1).
    LocalUse1 = 17 << 3,
    /// local use 2  (local2).
    LocalUse2 = 18 << 3,
    /// local use 3  (local3).
    LocalUse3 = 19 << 3,
    /// local use 4  (local4).
    LocalUse4 = 20 << 3,
    /// local use 5  (local5).
    LocalUse5 = 21 << 3,
    /// local use 6  (local6).
    LocalUse6 = 22 << 3,
    /// local use 7  (local7).
    LocalUse7 = 23 << 3,
}

/// Syslog severity.
///
/// See [RFC 5424](https://datatracker.ietf.org/doc/rfc5424).
#[derive(Debug)]
pub enum SyslogSeverity {
    /// System is unusable.
    Emergency = 0,
    /// Action must be taken immediately.
    Alert = 1,
    /// Critical conditions.
    Critical = 2,
    /// Error conditions.
    Error = 3,
    /// Warning conditions
    Warning = 4,
    /// Normal but significant condition
    Notice = 5,
    /// Informational messages.
    Info = 6,
    /// Debug-level messages.
    Debug = 7,
}

/// Signature for a custom mapping function that maps the rust log levels to
/// values of the syslog Severity.
pub type LevelToSyslogSeverity = fn(level: log::Level) -> SyslogSeverity;

fn default_mapping(level: log::Level) -> SyslogSeverity {
    match level {
        log::Level::Error => SyslogSeverity::Error,
        log::Level::Warn => SyslogSeverity::Warning,
        log::Level::Info => SyslogSeverity::Info,
        log::Level::Debug | log::Level::Trace => SyslogSeverity::Debug,
    }
}

enum SyslogType {
    Rfc5424 {
        hostname: String,
        message_id: String,
    },
    Rfc3164,
}

/// A configurable [`LogWriter`] implementation that writes log messages to the syslog
/// (see [RFC 5424](https://datatracker.ietf.org/doc/rfc5424)).
///
/// Only available with optional crate feature `syslog_writer`.
///
/// See the [writers](crate::writers) module for guidance how to use additional log writers.
pub struct SyslogWriter {
    process: String,
    pid: u32,
    syslog_type: SyslogType,
    facility: SyslogFacility,
    determine_severity: LevelToSyslogSeverity,
    m_conn_buf: Mutex<ConnectorAndBuffer>,
    max_log_level: log::LevelFilter,
}
impl SyslogWriter {
    /// Returns a configured boxed instance.
    ///
    /// ## Parameters
    ///
    /// `facility`: An value representing a valid syslog facility value according to RFC 5424.
    ///
    /// `determine_severity`: (optional) A function that maps the rust log levels
    /// to the syslog severities. If None is given, a trivial default mapping is used, which
    /// should be good enough in most cases.
    ///
    /// `message_id`: The value being used as syslog's MSGID, which
    /// should identify the type of message. The value itself
    /// is a string without further semantics. It is intended for filtering
    /// messages on a relay or collector.
    ///
    /// `syslog`: A [`Syslog`](crate::writers::Syslog).
    ///
    /// # Errors
    ///
    /// `std::io::Error`
    pub fn try_new(
        facility: SyslogFacility,
        determine_severity: Option<LevelToSyslogSeverity>,
        max_log_level: log::LevelFilter,
        message_id: String,
        syslog: Syslog,
    ) -> IoResult<Box<Self>> {
        const UNKNOWN_HOSTNAME: &str = "<unknown_hostname>";

        let hostname = hostname::get().map_or_else(
            |_| Ok(UNKNOWN_HOSTNAME.to_owned()),
            |s| {
                s.into_string().map_err(|_| {
                    IoError::new(
                        ErrorKind::InvalidData,
                        "Hostname contains non-UTF8 characters".to_owned(),
                    )
                })
            },
        )?;

        let process = std::env::args().next().ok_or_else(|| {
            IoError::new(
                ErrorKind::Other,
                "Can't infer app name as no env args are present".to_owned(),
            )
        })?;

        Ok(Box::new(Self {
            pid: std::process::id(),
            process,
            syslog_type: SyslogType::Rfc5424 {
                hostname,
                message_id,
            },
            facility,
            max_log_level,
            // shorter variants with unwrap_or() or unwrap_or_else() don't work
            // with either current clippy or old rustc:
            determine_severity: match determine_severity {
                Some(f) => f,
                None => default_mapping,
            },
            m_conn_buf: Mutex::new(ConnectorAndBuffer {
                conn: syslog.into_inner(),
                buf: Vec::with_capacity(200),
            }),
        }))
    }

    /// Returns a configured boxed instance.
    ///
    /// ## Parameters
    ///
    /// `facility`: An value representing a valid syslog facility value according to RFC 5424.
    ///
    /// `determine_severity`: (optional) A function that maps the rust log levels
    /// to the syslog severities. If None is given, a trivial default mapping is used, which
    /// should be good enough in most cases.
    ///
    /// `message_id`: The value being used as syslog's MSGID, which
    /// should identify the type of message. The value itself
    /// is a string without further semantics. It is intended for filtering
    /// messages on a relay or collector.
    ///
    /// `syslog`: A [`Syslog`](crate::writers::Syslog).
    ///
    /// # Errors
    ///
    /// `std::io::Error`
    pub fn try_new_bsd(
        facility: SyslogFacility,
        determine_severity: Option<LevelToSyslogSeverity>,
        max_log_level: log::LevelFilter,
        syslog: Syslog,
    ) -> IoResult<Box<Self>> {
        let process = std::env::args().next().ok_or_else(|| {
            IoError::new(
                ErrorKind::Other,
                "Can't infer app name as no env args are present".to_owned(),
            )
        })?;

        Ok(Box::new(Self {
            pid: std::process::id(),
            process,
            syslog_type: SyslogType::Rfc3164,
            facility,
            max_log_level,
            // shorter variants with unwrap_or() or unwrap_or_else() don't work
            // with either current clippy or old rustc:
            determine_severity: match determine_severity {
                Some(f) => f,
                None => default_mapping,
            },
            m_conn_buf: Mutex::new(ConnectorAndBuffer {
                conn: syslog.into_inner(),
                buf: Vec::with_capacity(200),
            }),
        }))
    }
}

impl LogWriter for SyslogWriter {
    fn write(&self, now: &mut DeferredNow, record: &log::Record) -> IoResult<()> {
        let mut conn_buf_guard = self
            .m_conn_buf
            .lock()
            .map_err(|_| crate::util::io_err("SyslogWriter is poisoned"))?;
        let cb = &mut *conn_buf_guard;
        let severity = (self.determine_severity)(record.level());

        // See [RFC 5424](https://datatracker.ietf.org/doc/rfc5424#page-8).
        cb.buf.clear();

        match &self.syslog_type {
            SyslogType::Rfc3164 => {
                write!(
                    cb.buf,
                    "<{pri}>{timestamp} {tag}[{procid}]: {msg}",
                    pri = self.facility as u8 | severity as u8,
                    timestamp = now.format_rfc3164(),
                    tag = self.process,
                    procid = self.pid,
                    msg = &record.args()
                )?;
            }
            SyslogType::Rfc5424 {
                hostname,
                message_id,
            } => {
                #[allow(clippy::write_literal)]
                write!(
                    cb.buf,
                    "<{pri}>{version} {timestamp} {hostname} {appname} {procid} {msgid} - {msg}",
                    pri = self.facility as u8 | severity as u8,
                    version = "1",
                    timestamp = now.format_rfc3339(),
                    hostname = hostname,
                    appname = self.process,
                    procid = self.pid,
                    msgid = message_id,
                    msg = &record.args()
                )?;
            }
        }
        // we _have_ to buffer because each write here generates a syslog entry
        cb.conn.write_all(&cb.buf)
    }

    fn flush(&self) -> IoResult<()> {
        self.m_conn_buf
            .lock()
            .map_err(|_| crate::util::io_err("SyslogWriter is poisoned"))?
            .conn
            .flush()
    }

    fn max_log_level(&self) -> log::LevelFilter {
        self.max_log_level
    }
}

struct ConnectorAndBuffer {
    conn: SyslogConnector,
    buf: Vec<u8>,
}

/// Implements the connection to the syslog.
///
/// Choose one of the factory methods that matches your environment,
/// depending on how the syslog is managed on your system,  
/// how you can access it and with which protocol you can write to it.
///
/// Is required to instantiate a [`SyslogWriter`](crate::writers::SyslogWriter).
pub struct Syslog(SyslogConnector);
impl Syslog {
    /// Returns a Syslog implementation that connects via unix datagram to the specified path.
    ///
    /// # Errors
    ///
    /// Any kind of I/O error can occur.
    #[cfg_attr(docsrs, doc(cfg(target_family = "unix")))]
    #[cfg(target_family = "unix")]
    pub fn try_datagram<P: AsRef<Path>>(path: P) -> IoResult<Self> {
        let ud = std::os::unix::net::UnixDatagram::unbound()?;
        ud.connect(&path)?;
        Ok(Syslog(SyslogConnector::Datagram(ud)))
    }

    /// Returns a Syslog implementation that connects via unix stream to the specified path.
    ///
    /// # Errors
    ///
    /// Any kind of I/O error can occur.
    #[cfg_attr(docsrs, doc(cfg(target_family = "unix")))]
    #[cfg(target_family = "unix")]
    pub fn try_stream<P: AsRef<Path>>(path: P) -> IoResult<Self> {
        Ok(Syslog(SyslogConnector::Stream(
            std::os::unix::net::UnixStream::connect(path)?,
        )))
    }

    /// Returns a Syslog implementation that sends the log lines via TCP to the specified address.
    ///
    /// # Errors
    ///
    /// `std::io::Error` if opening the stream fails.
    pub fn try_tcp<T: ToSocketAddrs>(server: T) -> IoResult<Self> {
        Ok(Syslog(SyslogConnector::Tcp(TcpStream::connect(server)?)))
    }

    /// Returns a Syslog implementation that sends the log via the fragile UDP protocol from local
    /// to server.
    ///
    /// # Errors
    ///
    /// `std::io::Error` if opening the stream fails.
    pub fn try_udp<T: ToSocketAddrs>(local: T, server: T) -> IoResult<Self> {
        let socket = UdpSocket::bind(local)?;
        socket.connect(server)?;
        Ok(Syslog(SyslogConnector::Udp(socket)))
    }

    fn into_inner(self) -> SyslogConnector {
        self.0
    }
}

#[derive(Debug)]
enum SyslogConnector {
    // Sends log lines to the syslog via a
    // [UnixStream](https://doc.rust-lang.org/std/os/unix/net/struct.UnixStream.html).
    #[cfg_attr(docsrs, doc(cfg(target_family = "unix")))]
    #[cfg(target_family = "unix")]
    Stream(std::os::unix::net::UnixStream),

    // Sends log lines to the syslog via a
    // [UnixDatagram](https://doc.rust-lang.org/std/os/unix/net/struct.UnixDatagram.html).
    #[cfg_attr(docsrs, doc(cfg(target_family = "unix")))]
    #[cfg(target_family = "unix")]
    Datagram(std::os::unix::net::UnixDatagram),

    // Sends log lines to the syslog via UDP.
    //
    // UDP is fragile and thus discouraged except for local communication.
    Udp(UdpSocket),

    // Sends log lines to the syslog via TCP.
    Tcp(TcpStream),
}

impl Write for SyslogConnector {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        match *self {
            #[cfg(target_family = "unix")]
            Self::Datagram(ref ud) => {
                // todo: reconnect if conn is broken
                ud.send(buf)
            }
            #[cfg(target_family = "unix")]
            Self::Stream(ref mut w) => {
                // todo: reconnect if conn is broken
                w.write(buf)
                    .and_then(|sz| w.write_all(&[0; 1]).map(|()| sz))
            }
            Self::Tcp(ref mut w) => {
                // todo: reconnect if conn is broken
                let n = w.write(buf)?;
                Ok(w.write(b"\n")? + n)
            }
            Self::Udp(ref socket) => {
                // ??
                socket.send(buf)
            }
        }
    }

    fn flush(&mut self) -> IoResult<()> {
        match *self {
            #[cfg(target_family = "unix")]
            Self::Datagram(_) => Ok(()),

            #[cfg(target_family = "unix")]
            Self::Stream(ref mut w) => w.flush(),

            Self::Udp(_) => Ok(()),

            Self::Tcp(ref mut w) => w.flush(),
        }
    }
}
