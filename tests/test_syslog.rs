mod test_utils;

#[cfg(feature = "syslog_writer")]
mod test {

    use flexi_logger::writers::{Syslog, SyslogFacility, SyslogWriter};
    use flexi_logger::{detailed_format, FileSpec, Logger};
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
            log::LevelFilter::Trace,
            "JustForTest".to_owned(),
            // Syslog::try_tcp("localhost:601")?,
            Syslog::try_udp("127.0.0.1:5555", "127.0.0.1:514")?,
        )
        .unwrap();
        let logger = Logger::try_with_str("info")
            .unwrap()
            .format(detailed_format)
            .log_to_file(
                FileSpec::default()
                    .suppress_timestamp()
                    .directory(super::test_utils::dir()),
            )
            .print_message()
            .add_writer("Syslog", boxed_syslog_writer)
            .start()
            .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

        // Explicitly send logs to different loggers
        error!(target : "{Syslog}", "This is a syslog-relevant error message");
        warn!(target : "{Syslog}", "This is a syslog-relevant error message");
        info!(target : "{Syslog}", "This is a syslog-relevant error message");
        debug!(target : "{Syslog}", "This is a syslog-relevant error message");
        trace!(target : "{Syslog}", "This is a syslog-relevant error message");

        error!(target : "{Syslog,_Default}", "This is a syslog- and log-relevant error message");

        // Nicer: use explicit macros
        syslog_error!("This is another syslog- and log-relevant error message");
        warn!("This is a warning message");
        debug!("This is a debug message - you must not see it!");
        trace!("This is a trace message - you must not see it!");

        // Verification:
        logger.validate_logs(&[
            (
                "ERROR",
                "syslog",
                "a syslog- and log-relevant error message",
            ),
            (
                "ERROR",
                "syslog",
                "another syslog- and log-relevant error message",
            ),
            ("WARN", "syslog", "This is a warning message"),
        ]);
        Ok(())
    }
}
