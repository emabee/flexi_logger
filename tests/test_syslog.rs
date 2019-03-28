#[cfg(target_os = "linux")]
#[cfg(feature = "syslog_writer")]
mod test {
    use flexi_logger::writers::{SyslogFacility, SyslogConnector, SyslogWriter};
    use flexi_logger::{detailed_format, Logger};
    use log::*;

    #[macro_use]
    mod macros {
        #[macro_export]
        macro_rules! syslog_error {
            ($($arg:tt)*) => (
                error!(target: "{Syslog,_Default}", $($arg)*);
            )
        }
    }

    #[test]
    fn test_syslog() -> std::io::Result<()> {
        let boxed_syslog_writer = SyslogWriter::try_new(
            SyslogFacility::LocalUse0,
            None,
            "JustForTest".to_owned(),
            SyslogConnector::try_udp("localhost:0",
            "localhost:514")?,
        )
        .unwrap();
        let log_handle = Logger::with_str("info")
            .format(detailed_format)
            .print_message()
            .log_to_file()
            .add_writer("Syslog", boxed_syslog_writer)
            .start()
            .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

        // Explicitly send logs to different loggers
        error!(target : "{Syslog}", "This is a syslog-relevant error message");
        error!(target : "{Syslog,_Default}", "This is a syslog- and log-relevant error message");

        // Nicer: use explicit macros
        syslog_error!("This is another syslog- and log-relevant error message");
        debug!("This is a warning message");
        debug!("This is a debug message - you must not see it!");
        trace!("This is a trace message - you must not see it!");

        // Verification:
        #[cfg_attr(rustfmt, rustfmt_skip)]
    log_handle.validate_logs(&[
        ("ERROR", "syslog", "a syslog- and log-relevant error message"),
        ("ERROR", "syslog", "another syslog- and log-relevant error message"),
    ]);
        // #[cfg_attr(rustfmt, rustfmt_skip)]
        // sec_handle.validate_logs(&[
        //     ("ERROR", "multi_logger", "security-relevant error"),
        //     ("ERROR", "multi_logger", "a security-relevant alert"),
        //     ("ERROR", "multi_logger", "security-relevant alert and log message"),
        //     ("ERROR", "multi_logger", "another security-relevant alert"),
        // ]);
        Ok(())
    }
}
