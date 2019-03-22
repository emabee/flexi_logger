use crate::writers::log_writer::LogWriter;
use std::cell::RefCell;
use std::io::Error as IoError;
use std::io::Result as IoResult;
use std::io::{BufWriter, ErrorKind, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs, UdpSocket};
use std::os::unix::net::{UnixDatagram, UnixStream};
use std::path::Path;
use std::sync::Mutex;

/// Syslog Severity.
///
/// See [RFC 5424](https://datatracker.ietf.org/doc/rfc5424).
#[doc(hidden)]
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

/// Syslog Facility.
///
/// See [RFC 5424](https://datatracker.ietf.org/doc/rfc5424).
#[derive(Copy, Clone)]
#[doc(hidden)]
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

/// Signature for a custom mapping function that maps the rust log levels to
/// values of the syslog Severity.
#[doc(hidden)]
pub type LevelToSyslogSeverity = fn(log::Level) -> SyslogSeverity;

fn default_mapping(level: log::Level) -> SyslogSeverity {
    match level {
        log::Level::Error => SyslogSeverity::Error,
        log::Level::Warn => SyslogSeverity::Error,
        log::Level::Info => SyslogSeverity::Error,
        log::Level::Debug => SyslogSeverity::Error,
        log::Level::Trace => SyslogSeverity::Error,
    }
}

/// Writes log messages to the syslog.
///
/// See [RFC 5424](https://datatracker.ietf.org/doc/rfc5424).
#[doc(hidden)]
pub struct SyslogWriter {
    hostname: String,
    process: String,
    pid: i32,
    facility: SyslogFacility,
    message_id: String,
    map_loglevel_to_severity: LevelToSyslogSeverity,
    syslog: Mutex<RefCell<Syslog>>,
}
impl SyslogWriter {
    pub fn try_path<P: AsRef<Path>>(
        path: P,
        facility: SyslogFacility,
        message_id: String,
        map_loglevel_to_severity: Option<LevelToSyslogSeverity>,
    ) -> IoResult<Box<SyslogWriter>> {
        Ok(Box::new(SyslogWriter::try_new(
            Syslog::try_path(path)?,
            facility,
            message_id,
            map_loglevel_to_severity,
        )?))
    }

    pub fn try_udp<T: ToSocketAddrs>(
        local: T,
        server: T,
        facility: SyslogFacility,
        message_id: String,
        map_loglevel_to_severity: Option<LevelToSyslogSeverity>,
    ) -> IoResult<Box<SyslogWriter>> {
        Ok(Box::new(SyslogWriter::try_new(
            Syslog::try_udp(local, server)?,
            facility,
            message_id,
            map_loglevel_to_severity,
        )?))
    }

    pub fn try_tcp<T: ToSocketAddrs>(
        server: T,
        facility: SyslogFacility,
        message_id: String,
        map_loglevel_to_severity: Option<LevelToSyslogSeverity>,
    ) -> IoResult<Box<SyslogWriter>> {
        Ok(Box::new(SyslogWriter::try_new(
            Syslog::try_tcp(server)?,
            facility,
            message_id,
            map_loglevel_to_severity,
        )?))
    }

    // Factory method.
    //
    // Regarding the parameters `facility` and `message_id`,
    // see [RFC 5424](https://datatracker.ietf.org/doc/rfc5424).
    fn try_new(
        syslog: Syslog,
        facility: SyslogFacility,
        message_id: String,
        map_loglevel_to_severity: Option<LevelToSyslogSeverity>,
    ) -> IoResult<SyslogWriter> {
        Ok(SyslogWriter {
            hostname: hostname::get_hostname().unwrap_or_else(|| "<unknown_hostname>".to_string()),
            process: std::env::args()
                .next()
                .ok_or_else(|| IoError::new(ErrorKind::Other, "<no progname>".to_owned()))?,
            pid: procinfo::pid::stat_self()?.pid,
            facility,
            message_id,
            map_loglevel_to_severity: map_loglevel_to_severity.unwrap_or_else(|| default_mapping),
            syslog: Mutex::new(RefCell::new(syslog)),
        })
    }
}

impl LogWriter for SyslogWriter {
    fn write(&self, record: &log::Record) -> IoResult<()> {
        let mr_syslog = self.syslog.lock().unwrap();
        let mut syslog = mr_syslog.borrow_mut();

        let severity = (self.map_loglevel_to_severity)(record.level());
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

pub(crate) enum Syslog {
    Local(UnixDatagram),
    UnixStream(BufWriter<UnixStream>),
    Udp(UdpSocket, SocketAddr),
    Tcp(BufWriter<TcpStream>),
}
impl Syslog {
    pub fn try_path<P: AsRef<Path>>(path: P) -> IoResult<Syslog> {
        let ud = UnixDatagram::unbound()?;
        match ud.connect(&path) {
            Ok(()) => Ok(Syslog::Local(ud)),
            Err(ref e) if e.raw_os_error() == Some(libc::EPROTOTYPE) => Ok(Syslog::UnixStream(
                BufWriter::new(UnixStream::connect(path)?),
            )),
            Err(e) => Err(e),
        }
    }

    pub fn try_udp<T: ToSocketAddrs>(local: T, server: T) -> IoResult<Syslog> {
        server
            .to_socket_addrs()
            .and_then(|mut addrs_iter| {
                addrs_iter.next().ok_or_else(|| {
                    IoError::new(ErrorKind::Other, "Server address resolution failed")
                })
            })
            .and_then(|server_addr| {
                UdpSocket::bind(local)
                    .and_then(|local_socket| Ok(Syslog::Udp(local_socket, server_addr)))
            })
    }

    pub fn try_tcp<T: ToSocketAddrs>(server: T) -> IoResult<Syslog> {
        TcpStream::connect(server).and_then(|tcpstream| Ok(Syslog::Tcp(BufWriter::new(tcpstream))))
    }
}

impl Write for Syslog {
    fn write(&mut self, message: &[u8]) -> IoResult<usize> {
        match *self {
            Syslog::Local(ref ud) => ud.send(&message[..]),
            Syslog::UnixStream(ref mut w) => w
                .write(&message[..])
                .and_then(|sz| w.write_all(&[0; 1]).map(|_| sz)),
            Syslog::Udp(ref socket, ref addr) => socket.send_to(&message[..], addr),
            Syslog::Tcp(ref mut w) => w.write(&message[..]),
        }
    }

    fn flush(&mut self) -> IoResult<()> {
        match *self {
            Syslog::Local(_) => Ok(()),
            Syslog::UnixStream(ref mut w) => w.flush(),
            Syslog::Udp(_, _) => Ok(()),
            Syslog::Tcp(ref mut w) => w.flush(),
        }
    }
}
