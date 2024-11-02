use super::{
    line::SyslogLineHeader, severity::default_mapping, syslog_default_format,
    LevelToSyslogSeverity, SyslogConnection, SyslogFacility, SyslogWriter,
};
use crate::FormatFunction;
use std::io::{Error as IoError, ErrorKind, Result as IoResult};

#[allow(clippy::module_name_repetitions)]
/// Builder for the `SyslogWriter`.
///
/// Is created with [`SyslogWriter::builder`].
pub struct SyslogWriterBuilder {
    syslog_connection: SyslogConnection,
    syslog_line_header: SyslogLineHeader,
    syslog_facility: SyslogFacility,
    custom_process_name: Option<String>,
    determine_severity: LevelToSyslogSeverity,
    max_log_level: log::LevelFilter,
    format: FormatFunction,
}
impl SyslogWriterBuilder {
    #[must_use]
    pub(super) fn new(
        syslog: SyslogConnection,
        syslog_line_header: SyslogLineHeader,
        syslog_facility: SyslogFacility,
    ) -> SyslogWriterBuilder {
        SyslogWriterBuilder {
            syslog_connection: syslog,
            syslog_line_header,
            syslog_facility,
            custom_process_name: None,
            determine_severity: default_mapping,
            max_log_level: log::LevelFilter::Warn,
            format: syslog_default_format,
        }
    }

    /// Specify a custom process name, or unset it to revert back to name inference.
    #[must_use]
    pub fn custom_process_name(mut self, name: Option<&str>) -> Self {
        self.custom_process_name = name.map(Into::into);
        self
    }

    /// Use the given function to map the rust log levels to the syslog severities.
    /// By default a trivial mapping is used, which should be good enough in most cases.
    #[must_use]
    pub fn determine_severity(mut self, mapping: LevelToSyslogSeverity) -> Self {
        self.determine_severity = mapping;
        self
    }

    /// Specify up to which level log messages should be sent to the syslog.
    ///
    /// Default is: only warnings and errors.
    #[must_use]
    pub fn max_log_level(mut self, max_log_level: log::LevelFilter) -> Self {
        self.max_log_level = max_log_level;
        self
    }

    /// Use the given format function to write the message part of the syslog entries.
    ///
    /// By default, [`syslog_default_format`](crate::writers::syslog_default_format) is used.
    ///
    /// You can instead use [`syslog_format_with_thread`](crate::writers::syslog_format_with_thread)
    /// or your own `FormatFunction`
    /// (see the source code of the provided functions if you want to write your own).
    #[must_use]
    pub fn format(mut self, format: FormatFunction) -> Self {
        self.format = format;
        self
    }

    /// Returns a boxed instance of `SysLogWriter`.
    ///
    /// # Errors
    ///
    /// `std::io::Error` if the program's argument list is empty so that the process
    /// identifier for the syslog cannot be determined
    pub fn build(self) -> IoResult<Box<SyslogWriter>> {
        Ok(Box::new(SyslogWriter::new(
            std::process::id(),
            self.custom_process_name
                .or(std::env::args().next())
                .ok_or_else(|| {
                    IoError::new(
                        ErrorKind::Other,
                        "Can't provide a process name as no env args are present and \
                        no custom process name is set"
                            .to_owned(),
                    )
                })?,
            self.syslog_line_header,
            self.syslog_facility,
            self.determine_severity,
            self.syslog_connection,
            self.max_log_level,
            self.format,
        )?))
    }
}
