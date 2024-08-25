use std::{
    io::{Result as IoResult, Write},
    net::{TcpStream, UdpSocket},
};

// Writable and flushable connection to the syslog backend.
#[derive(Debug)]
pub(super) enum Connection {
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

impl Write for Connection {
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
