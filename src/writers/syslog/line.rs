use std::io::{Error as IoError, ErrorKind, Result as IoResult, Write};
#[cfg(unix)]
use std::{ffi::CStr, sync::Mutex};

use crate::{DeferredNow, FormatFunction};

use super::{LevelToSyslogSeverity, SyslogFacility};

#[cfg(unix)]
static POSIX_SYSLOG_STATE: Mutex<PosixSyslogState> = Mutex::new(PosixSyslogState {
    idents_stack: vec![],
    buf: vec![],
});

#[cfg(unix)]
struct PosixSyslogState {
    idents_stack: Vec<&'static CStr>,
    buf: Vec<u8>,
}

/// Defines the format of the header of a syslog line.
pub enum SyslogLineHeader {
    /// Line header according to RFC 5424.
    Rfc5424(String),
    /// Line header according to RFC 3164.
    Rfc3164,
}
pub(crate) struct LineWriter {
    header: SyslogLineHeader,
    hostname: Option<String>,
    process: String,
    pid: u32,
    format: FormatFunction,
    determine_severity: LevelToSyslogSeverity,
    facility: SyslogFacility,
}
impl LineWriter {
    pub(crate) fn new(
        header: SyslogLineHeader,
        determine_severity: LevelToSyslogSeverity,
        facility: SyslogFacility,
        process: String,
        pid: u32,
        format: FormatFunction,
    ) -> IoResult<LineWriter> {
        Ok(LineWriter {
            hostname: if matches!(header, SyslogLineHeader::Rfc5424(_)) {
                Some(get_hostname()?)
            } else {
                None
            },
            header,
            process,
            pid,
            format,
            determine_severity,
            facility,
        })
    }

    pub(crate) fn write_to_syslog_socket_buffer(
        &self,
        buffer: &mut dyn Write,
        now: &mut DeferredNow,
        record: &log::Record,
    ) -> IoResult<()> {
        // See [RFC 5424](https://datatracker.ietf.org/doc/rfc5424#page-8).
        let severity = (self.determine_severity)(record.level());

        match self.header {
            SyslogLineHeader::Rfc3164 => {
                write!(
                    buffer,
                    "<{pri}>{timestamp} {tag}[{procid}]: ",
                    pri = self.facility as u8 | severity as u8,
                    timestamp = now.format_rfc3164(),
                    tag = self.process,
                    procid = self.pid
                )?;
                (self.format)(buffer, now, record)?;
            }
            SyslogLineHeader::Rfc5424(ref message_id) => {
                #[allow(clippy::write_literal)]
                write!(
                    buffer,
                    "<{pri}>{version} {timestamp} {hostname} {appname} {procid} {msgid} ",
                    pri = self.facility as u8 | severity as u8,
                    version = "1",
                    timestamp = now.format_rfc3339(),
                    hostname = self.hostname.as_deref().unwrap_or("<unknown_hostname>"),
                    appname = self.process,
                    procid = self.pid,
                    msgid = message_id,
                )?;
                write_key_value_pairs(buffer, record)?;
                (self.format)(buffer, now, record)?;
            }
        }
        Ok(())
    }

    #[cfg(unix)]
    pub(crate) fn write_with_syslog_call(
        &self,
        now: &mut DeferredNow,
        record: &log::Record,
    ) -> IoResult<()> {
        use std::{
            ffi::{CString, OsStr},
            os::unix::ffi::OsStrExt,
        };

        use nix::syslog::{openlog, syslog, LogFlags};

        let mut posix_syslog_state = POSIX_SYSLOG_STATE
            .lock()
            .map_err(|_| crate::util::io_err("LineWriter is poisoned"))?;

        // If the ident (process) to use does not match the globally last used
        // ident, we need to call `openlog` to set it process-wide. We use a stack
        // as a set of sorts because it's much more efficient for the extremely common
        // case of always using the same ident (i.e., only a syslog writer was set up)
        if Some(self.process.as_bytes())
            != posix_syslog_state.idents_stack.last().map(|s| s.to_bytes())
        {
            let ident_cstr = if let Some((i, _)) = posix_syslog_state
                .idents_stack
                .iter()
                .enumerate()
                .find(|(_, s)| s.to_bytes() == &*self.process.as_bytes())
            {
                // Reuse the pooled C ident string to avoid leaking it again
                posix_syslog_state.idents_stack.swap_remove(i)
            } else {
                Box::leak(
                    CString::new(&*self.process)
                        .map_err(|_| {
                            crate::util::io_err("SyslogWriter ident contains internal NUL bytes")
                        })?
                        .into_boxed_c_str(),
                )
            };

            posix_syslog_state.idents_stack.push(ident_cstr);

            // nix openlog bindings have a strange Linux-specific signature we have to work around.
            // More details: https://github.com/nix-rust/nix/pull/2537#discussion_r2163724906
            #[cfg(target_os = "linux")]
            openlog(Some(ident_cstr), LogFlags::LOG_PID, self.facility.to_nix())?;
            #[cfg(not(target_os = "linux"))]
            openlog(
                Some(OsStr::from_bytes(ident_cstr.to_bytes())),
                LogFlags::LOG_PID,
                self.facility.to_nix(),
            )?;
        }

        posix_syslog_state.buf.clear();
        (self.format)(&mut posix_syslog_state.buf, now, record)?;

        Ok(syslog(
            (self.determine_severity)(record.level()).to_nix(),
            OsStr::from_bytes(&posix_syslog_state.buf),
        )?)
    }

    pub(crate) fn shutdown(&self) {
        #[cfg(unix)]
        if let Ok(posix_syslog_state) = POSIX_SYSLOG_STATE.lock() {
            if !posix_syslog_state.idents_stack.is_empty() {
                nix::syslog::closelog();
            }
        }
    }
}

// Helpers for printing key-value pairs
#[allow(unused_variables)]
fn write_key_value_pairs(
    w: &mut dyn std::io::Write,
    record: &log::Record<'_>,
) -> Result<(), std::io::Error> {
    #[allow(unused_mut)]
    let mut kv_written = false;
    #[cfg(feature = "kv")]
    if record.key_values().count() > 0 {
        write!(w, "[log_kv ",)?;
        let mut kv_stream = KvStream(w, false);
        record.key_values().visit(&mut kv_stream).ok();
        write!(w, "] ")?;
        kv_written = true;
    }

    if !kv_written {
        write!(w, "- ")?;
    }
    Ok(())
}

#[cfg(feature = "kv")]
struct KvStream<'a>(&'a mut dyn std::io::Write, bool);
#[cfg(feature = "kv")]
impl<'kvs, 'a> log::kv::VisitSource<'kvs> for KvStream<'a>
where
    'kvs: 'a,
{
    fn visit_pair(
        &mut self,
        key: log::kv::Key<'kvs>,
        value: log::kv::Value<'kvs>,
    ) -> Result<(), log::kv::Error> {
        if self.1 {
            write!(self.0, " ")?;
        }
        write!(self.0, "{key}=\"{value:?}\"")?;
        self.1 = true;
        Ok(())
    }
}

fn get_hostname() -> IoResult<String> {
    // Even though the `hostname` crate provides a cross-platform way to get the hostname,
    // it may also introduce version conflicts on the `libc` crate pulled by `nix`, so let's
    // just use `nix` directly when possible, which also has the advantage of reducing the
    // number of dependencies
    {
        #[cfg(not(unix))]
        {
            hostname::get()?.into_string()
        }
        #[cfg(unix)]
        {
            nix::unistd::gethostname()?.into_string()
        }
    }
    .map_err(|_| {
        IoError::new(
            ErrorKind::InvalidData,
            "Hostname contains non-UTF8 characters",
        )
    })
}
