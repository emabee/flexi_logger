use std::{
    io::{Result as IoResult, Write},
    net::{TcpStream, UdpSocket},
};

// Writable and flushable connection to the syslog backend.
#[derive(Debug)]
pub(super) enum Connection {
    /// Sends log lines to the syslog via a
    /// [UnixStream](https://doc.rust-lang.org/std/os/unix/net/struct.UnixStream.html).
    #[cfg_attr(docsrs, doc(cfg(unix)))]
    #[cfg(unix)]
    Stream(std::os::unix::net::UnixStream),

    /// Sends log lines to the syslog via a
    /// [UnixDatagram](https://doc.rust-lang.org/std/os/unix/net/struct.UnixDatagram.html).
    #[cfg_attr(docsrs, doc(cfg(unix)))]
    #[cfg(unix)]
    Datagram(std::os::unix::net::UnixDatagram),

    /// Sends log lines to the local syslog using the `syslog` C function.
    #[cfg_attr(docsrs, doc(cfg(unix)))]
    #[cfg(unix)]
    SyslogCall,

    /// Sends log lines to the syslog via UDP.
    Udp(UdpSocket),

    /// Sends log lines to the syslog via TCP.
    Tcp(TcpStream),
}

impl Write for Connection {
    fn write(&mut self, buf: &[u8]) -> IoResult<usize> {
        match *self {
            #[cfg(unix)]
            Self::Datagram(ref ud) => {
                // todo: reconnect if conn is broken
                ud.send(buf)
            }
            #[cfg(unix)]
            Self::Stream(ref mut w) => {
                // todo: reconnect if conn is broken
                w.write(buf)
                    .and_then(|sz| w.write_all(&[0; 1]).map(|()| sz))
            }
            #[cfg(unix)]
            Self::SyslogCall => Ok(buf.len()), // `syslog` needs a priority value, which has to be provided by an upper layer
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
            #[cfg(unix)]
            Self::Datagram(_) => Ok(()),

            #[cfg(unix)]
            Self::Stream(ref mut w) => w.flush(),

            #[cfg(unix)]
            Self::SyslogCall => Ok(()),

            Self::Udp(_) => Ok(()),

            Self::Tcp(ref mut w) => w.flush(),
        }
    }
}
