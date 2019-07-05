#[cfg(feature = "specfile")]
mod a {
    use flexi_logger::{detailed_format, Logger};
    use log::*;

    /// Rudimentary test of the specfile feature, using the file ./tests/logspec.toml.
    /// For real test, run this manually, change the duration before to a much higher value (see below),
    /// and edit the file while the test is running. You should see the impact immediately -
    /// by default, ERR, WARN, and INFO messages are printed. If you change the level in the file,
    /// less or more lines should be printed.
    #[test]
    fn test_specfile() {
        let specfile = "./tests/logspec.toml";
        std::fs::remove_file(specfile).ok();
        Logger::with_str("info")
            .format(detailed_format)
            .start_with_specfile(specfile)
            .unwrap_or_else(|e| panic!("Logger initialization failed because: {}", e));

        let wait = std::time::Duration::from_millis(1);
        // if you want to give yourself a real chance to update the specfile in between:
        // let wait = std::time::Duration::from_millis(500);
        for _ in 0..100 {
            std::thread::sleep(wait);
            error!("This is an error message");
            warn!("This is a warning");
            info!("This is an info message");
            debug!("This is a debug message");
            trace!("This is a trace message");
        }
    }
}

