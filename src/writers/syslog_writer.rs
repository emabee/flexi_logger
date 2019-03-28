use crate::writers::log_writer::LogWriter;
use std::cell::RefCell;
use std::io::Error as IoError;
use std::io::Result as IoResult;
use std::io::{BufWriter, ErrorKind, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
#[cfg(target_os = "linux")]
use std::path::Path;
use std::sync::Mutex;

/// Syslog Facility.
///
/// See [RFC 5424](https://datatracker.ietf.org/doc/rfc5424).
#[derive(Copy, Clone)]
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

/// SyslogConnector Severity.
///
/// See [RFC 5424](https://datatracker.ietf.org/doc/rfc5424).
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

/// A configurable `LogWriter` implementation that writes log messages to the syslog
/// (see [RFC 5424](https://datatracker.ietf.org/doc/rfc5424)).
///
/// See the [module description](index.html) for guidance how to use additional log writers.
pub struct SyslogWriter {
    hostname: String,
    process: String,
    pid: u32,
    facility: SyslogFacility,
    message_id: String,
    determine_severity: LevelToSyslogSeverity,
    syslog: Mutex<RefCell<SyslogConnector>>,
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
    /// `syslog`: A [SyslogConnector](enum.SyslogConnector.html).

    pub fn try_new(
        facility: SyslogFacility,
        determine_severity: Option<LevelToSyslogSeverity>,
        message_id: String,
        syslog: SyslogConnector,
    ) -> IoResult<Box<SyslogWriter>> {
        Ok(Box::new(SyslogWriter {
            hostname: hostname::get_hostname().unwrap_or_else(|| "<unknown_hostname>".to_owned()),
            process: std::env::args()
                .next()
                .ok_or_else(|| IoError::new(ErrorKind::Other, "<no progname>".to_owned()))?,
            pid: std::process::id(),
            facility,
            message_id,
            determine_severity: determine_severity.unwrap_or_else(|| default_mapping),
            syslog: Mutex::new(RefCell::new(syslog)),
        }))
    }
}

impl LogWriter for SyslogWriter {
    fn write(&self, record: &log::Record) -> IoResult<()> {
        let mr_syslog = self.syslog.lock().unwrap();
        let mut syslog = mr_syslog.borrow_mut();

        let severity = (self.determine_severity)(record.level());
        write!(
            syslog,
            "<{}> {} 1 {} {} {} {} - {}",
            self.facility as u8 | severity as u8,
            chrono::Utc::now().to_rfc3339(), // or Local?
            self.hostname,
            self.process,
            self.pid,
            self.message_id,
            &record.args()
        )
    }

    fn flush(&self) -> IoResult<()> {
        let mr_syslog = self.syslog.lock().unwrap();
        let mut syslog = mr_syslog.borrow_mut();
        syslog.flush()?;
        Ok(())
    }
}

/// Helper struct that connects to the syslog and implements Write.
///
/// Is used in [SyslogWriter::try_new()](struct.SyslogWriter.html#method.try_new).
pub enum SyslogConnector {
    /// Sends log lines to the syslog via a
    /// [UnixStream](https://doc.rust-lang.org/std/os/unix/net/struct.UnixStream.html).
    #[cfg(target_os = "linux")]
    Stream(BufWriter<std::os::unix::net::UnixStream>),

    /// Sends log lines to the syslog via a
    /// [UnixDatagram](https://doc.rust-lang.org/std/os/unix/net/struct.UnixDatagram.html).
    #[cfg(target_os = "linux")]
    Datagram(std::os::unix::net::UnixDatagram),

    /// Sends log lines to the syslog via UDP.
    ///
    /// Due to UDP being fragile, is discouraged unless for local communication.
    Udp(UdpSocket, SocketAddr),

    /// Sends log lines to the syslog via TCP.
    Tcp(BufWriter<TcpStream>),
}
impl SyslogConnector {
    /// Returns a SyslogConnector::Datagram to the specified path.
    #[cfg(target_os = "linux")]
    pub fn try_datagram<P: AsRef<Path>>(path: P) -> IoResult<SyslogConnector> {
        let ud = std::os::unix::net::UnixDatagram::unbound()?;
        ud.connect(&path)?;
        Ok(SyslogConnector::Datagram(ud))
    }

    /// Returns a SyslogConnector::Stream to the specified path.
    #[cfg(target_os = "linux")]
    pub fn try_stream<P: AsRef<Path>>(path: P) -> IoResult<SyslogConnector> {
        Ok(SyslogConnector::Stream(BufWriter::new(
            std::os::unix::net::UnixStream::connect(path)?,
        )))
    }

    /// Returns a SyslogConnector which sends the log lines via TCP to the specified address.
    pub fn try_tcp<T: ToSocketAddrs>(server: T) -> IoResult<SyslogConnector> {
        Ok(SyslogConnector::Tcp(BufWriter::new(TcpStream::connect(
            server,
        )?)))
    }

    /// Returns a SyslogConnector which sends log via the fragile UDP protocol from local to server.
    pub fn try_udp<T: ToSocketAddrs>(local: T, server: T) -> IoResult<SyslogConnector> {
        Ok(SyslogConnector::Udp(
            UdpSocket::bind(local)?,
            server.to_socket_addrs()?.next().ok_or_else(|| {
                IoError::new(ErrorKind::Other, "Server address resolution failed")
            })?,
        ))
    }
}

impl Write for SyslogConnector {
    fn write(&mut self, message: &[u8]) -> IoResult<usize> {
        match *self {
            #[cfg(target_os = "linux")]
            SyslogConnector::Datagram(ref ud) => {
                // fixme: reconnect of conn is broken
                ud.send(&message[..])
            }
            #[cfg(target_os = "linux")]
            SyslogConnector::Stream(ref mut w) => {
                // fixme: reconnect of conn is broken
                w.write(&message[..])
                    .and_then(|sz| w.write_all(&[0; 1]).map(|_| sz))
            }
            SyslogConnector::Tcp(ref mut w) => {
                // fixme: reconnect of conn is broken
                w.write(&message[..])
            }
            SyslogConnector::Udp(ref socket, ref addr) => socket.send_to(&message[..], addr),
        }
    }

    fn flush(&mut self) -> IoResult<()> {
        match *self {
            #[cfg(target_os = "linux")]
            SyslogConnector::Datagram(_) => Ok(()),

            #[cfg(target_os = "linux")]
            SyslogConnector::Stream(ref mut w) => w.flush(),

            SyslogConnector::Udp(_, _) => Ok(()),

            SyslogConnector::Tcp(ref mut w) => w.flush(),
        }
    }
}
