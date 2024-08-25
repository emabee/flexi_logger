mod builder;
mod connection;
mod facility;
mod formats;
mod line;
mod severity;
mod syslog_connection;
mod writer;

#[allow(clippy::module_name_repetitions)]
pub use self::{
    builder::SyslogWriterBuilder,
    facility::SyslogFacility,
    formats::{syslog_default_format, syslog_format_with_thread},
    line::SyslogLineHeader,
    severity::{LevelToSyslogSeverity, SyslogSeverity},
    syslog_connection::SyslogConnection,
    writer::SyslogWriter,
};
