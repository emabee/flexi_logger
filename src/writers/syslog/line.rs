use std::io::{Error as IoError, ErrorKind, Result as IoResult, Write};

use crate::{DeferredNow, FormatFunction};

use super::{LevelToSyslogSeverity, SyslogFacility};

/// Defines the format of the header of a syslog line.
pub enum SyslogLineHeader {
    /// Line header according to RFC 5424.
    Rfc5424(String),
    /// Line header according to RFC 3164.
    Rfc3164,
}
pub(crate) struct LineWriter {
    header: SyslogLineHeader,
    hostname: String,
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
        const UNKNOWN_HOSTNAME: &str = "<unknown_hostname>";
        // FIXME
        Ok(LineWriter {
            header,
            hostname: hostname::get().map_or_else(
                |_| Ok(UNKNOWN_HOSTNAME.to_owned()),
                |s| {
                    s.into_string().map_err(|_| {
                        IoError::new(
                            ErrorKind::InvalidData,
                            "Hostname contains non-UTF8 characters".to_owned(),
                        )
                    })
                },
            )?,
            process,
            pid,
            format,
            determine_severity,
            facility,
        })
    }

    pub(crate) fn write_syslog_entry(
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
                    hostname = self.hostname,
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
}

// Helpers for printing key-value pairs
fn write_key_value_pairs(
    w: &mut dyn std::io::Write,
    record: &log::Record<'_>,
) -> Result<(), std::io::Error> {
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
