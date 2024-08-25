/// Syslog severity.
///
/// See [RFC 5424](https://datatracker.ietf.org/doc/rfc5424).
#[derive(Debug)]
#[allow(clippy::module_name_repetitions)]
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
#[allow(clippy::module_name_repetitions)]
pub type LevelToSyslogSeverity = fn(level: log::Level) -> SyslogSeverity;

pub(crate) fn default_mapping(level: log::Level) -> SyslogSeverity {
    match level {
        log::Level::Error => SyslogSeverity::Error,
        log::Level::Warn => SyslogSeverity::Warning,
        log::Level::Info => SyslogSeverity::Info,
        log::Level::Debug | log::Level::Trace => SyslogSeverity::Debug,
    }
}
