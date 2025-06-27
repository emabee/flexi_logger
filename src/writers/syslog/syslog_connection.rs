use super::connection::Connection;

#[cfg(unix)]
use std::path::Path;
use std::{
    io::Result as IoResult,
    net::{TcpStream, ToSocketAddrs, UdpSocket},
};

/// Implements the connection to the syslog.
///
/// Choose one of the factory methods that matches your environment,
/// depending on how the syslog is managed on your system,
/// how you can access it and with which protocol you can write to it.
///
/// Is required to instantiate a [`SyslogWriter`](crate::writers::SyslogWriter).
#[allow(clippy::module_name_repetitions)]
pub struct SyslogConnection(Connection);
impl SyslogConnection {
    /// Returns a `Syslog` that connects via unix datagram to the specified path.
    ///
    /// # Errors
    ///
    /// Any kind of I/O error can occur.
    #[cfg_attr(docsrs, doc(cfg(unix)))]
    #[cfg(unix)]
    pub fn try_datagram<P: AsRef<Path>>(path: P) -> IoResult<Self> {
        let ud = std::os::unix::net::UnixDatagram::unbound()?;
        ud.connect(&path)?;
        Ok(SyslogConnection(Connection::Datagram(ud)))
    }

    /// Returns a `Syslog` that connects via unix stream to the specified path.
    ///
    /// # Errors
    ///
    /// Any kind of I/O error can occur.
    #[cfg_attr(docsrs, doc(cfg(unix)))]
    #[cfg(unix)]
    pub fn try_stream<P: AsRef<Path>>(path: P) -> IoResult<Self> {
        Ok(SyslogConnection(Connection::Stream(
            std::os::unix::net::UnixStream::connect(path)?,
        )))
    }

    /// Returns a `Syslog` that delegates on the POSIX-standard [`syslog` C function]
    /// provided by the platform's standard C library to send logs.
    ///
    /// This is ideal for portably sending logs to whatever system logging service
    /// may be available, which might not always be listening on a socket and be
    /// configured with varying policies for log delivery and storage.
    ///
    /// The value of `SyslogLineHeader` is ignored when using this connection type,
    /// as the `syslog` function entirely handles the delivery protocol and formatting
    /// of log messages.
    ///
    /// Because the `syslog` function has its own internal and opaque process-wide
    /// state, this connection assumes that no code external to the logger will
    /// attempt to set up the `syslog` function with different parameters.
    ///
    /// For reference, on Linux with `musl` or `glibc`, the `syslog` function
    /// communicates with a daemon listening on the `/dev/log` Unix stream socket
    /// using the RFC 3164 protocol. This daemon is typically `systemd-journald`,
    /// `syslogd`, `rsyslogd`, or `syslog-ng`. FreeBSD uses the `syslogd` daemon
    /// listening on the `/var/run/log` Unix datagram socket. Older macOS versions
    /// used the `/var/run/syslog` Unix stream socket, while newer versions (10.x+)
    /// forward logs to the `OSLog` framework (a.k.a. unified logging system) and lack
    /// a `syslogd`-compatible daemon that listens on a socket. Other systems and
    /// standard C libraries may have different implementations of such a function.
    ///
    /// [`syslog` C function]: https://pubs.opengroup.org/onlinepubs/9799919799/functions/syslog.html
    #[cfg_attr(docsrs, doc(cfg(unix)))]
    #[cfg(unix)]
    #[must_use]
    pub fn syslog_call() -> Self {
        SyslogConnection(Connection::SyslogCall)
    }

    /// Returns a `Syslog` that sends the log lines via TCP to the specified address.
    ///
    /// # Errors
    ///
    /// `std::io::Error` if opening the stream fails.
    pub fn try_tcp<T: ToSocketAddrs>(server: T) -> IoResult<Self> {
        Ok(SyslogConnection(Connection::Tcp(TcpStream::connect(
            server,
        )?)))
    }

    /// Returns a `Syslog` that sends the log via the fragile UDP protocol from local
    /// to server.
    ///
    /// # Errors
    ///
    /// `std::io::Error` if opening the stream fails.
    pub fn try_udp<T: ToSocketAddrs>(local: T, server: T) -> IoResult<Self> {
        let socket = UdpSocket::bind(local)?;
        socket.connect(server)?;
        Ok(SyslogConnection(Connection::Udp(socket)))
    }

    pub(super) fn into_inner(self) -> Connection {
        self.0
    }
}
