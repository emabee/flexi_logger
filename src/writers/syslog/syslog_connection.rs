use super::connection::Connection;
#[cfg(target_family = "unix")]
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
    #[cfg_attr(docsrs, doc(cfg(target_family = "unix")))]
    #[cfg(target_family = "unix")]
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
    #[cfg_attr(docsrs, doc(cfg(target_family = "unix")))]
    #[cfg(target_family = "unix")]
    pub fn try_stream<P: AsRef<Path>>(path: P) -> IoResult<Self> {
        Ok(SyslogConnection(Connection::Stream(
            std::os::unix::net::UnixStream::connect(path)?,
        )))
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
