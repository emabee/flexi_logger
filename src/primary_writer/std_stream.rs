use std::io::{Error as IoError, Stderr, StderrLock, Stdout, StdoutLock, Write};

// Abstraction over stdout and stderr
pub(crate) enum StdStream {
    Out(Stdout),
    Err(Stderr),
}
impl<'a> StdStream {
    pub(crate) fn deref_mut(&mut self) -> &mut dyn Write {
        match self {
            StdStream::Out(ref mut s) => s as &mut dyn Write,
            StdStream::Err(ref mut s) => s as &mut dyn Write,
        }
    }
    pub(crate) fn lock(&'a self) -> StdstreamLock<'a> {
        match self {
            StdStream::Out(ref s) => StdstreamLock::Out(s.lock()),
            StdStream::Err(ref s) => StdstreamLock::Err(s.lock()),
        }
    }
}
impl Write for StdStream {
    fn write(&mut self, buffer: &[u8]) -> std::result::Result<usize, IoError> {
        self.deref_mut().write(buffer)
    }
    fn flush(&mut self) -> std::result::Result<(), IoError> {
        self.deref_mut().flush()
    }
}

pub(crate) enum StdstreamLock<'a> {
    Out(StdoutLock<'a>),
    Err(StderrLock<'a>),
}
impl<'a> Write for StdstreamLock<'a> {
    fn write(&mut self, buffer: &[u8]) -> std::result::Result<usize, IoError> {
        match self {
            StdstreamLock::Out(l) => l.write(buffer),
            StdstreamLock::Err(l) => l.write(buffer),
        }
    }
    fn flush(&mut self) -> std::result::Result<(), IoError> {
        match self {
            StdstreamLock::Out(l) => l.flush(),
            StdstreamLock::Err(l) => l.flush(),
        }
    }
}
