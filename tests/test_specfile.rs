#[cfg(feature = "specfile")]
mod a {
    use flexi_logger::{detailed_format, Logger};
    use log::*;
    use std::io::{BufRead, Write};
    use std::ops::Add;

    /// Rudimentary test of the specfile feature, using the file ./tests/logspec.toml.
    /// For real test, run this manually, change the duration before to a much higher value (see below),
    /// and edit the file while the test is running. You should see the impact immediately -
    /// by default, ERR, WARN, and INFO messages are printed. If you change the level in the file,
    /// less or more lines should be printed.
    #[test]
    fn test_specfile() {
        let specfile = "test_specfile_logspec.toml";

        let logfile = std::path::Path::new(&std::env::args().nth(0).unwrap())
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string()
            .add(".log");

        std::fs::remove_file(specfile).ok();

        Logger::with_str("info")
            .format(detailed_format)
            .log_to_file()
            .suppress_timestamp()
            .start_with_specfile(specfile)
            .unwrap_or_else(|e| panic!("Logger initialization failed because: {}", e));

        error!("This is an error message");
        warn!("This is a warning");
        info!("This is an info message");
        debug!("This is a debug message");
        trace!("This is a trace message");

        // update to warn
        let mut file = std::fs::OpenOptions::new()
            .truncate(true)
            .write(true)
            .open(specfile)
            .unwrap();
        file.write_all(
            b"
        global_level = 'warn'
        [modules]
        ",
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(600));
        error!("This is an error message");
        warn!("This is a warning");
        info!("This is an info message");
        debug!("This is a debug message");
        trace!("This is a trace message");

        // behave like many editors: rename and recreate as err
        std::fs::rename(&specfile, "old_logspec.toml").unwrap();
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .open(specfile)
            .unwrap();
        file.write_all(
            b"
        global_level = 'error'
        [modules]
        ",
        )
        .unwrap();
        std::thread::sleep(std::time::Duration::from_millis(600));
        error!("This is an error message");
        warn!("This is a warning");
        info!("This is an info message");
        debug!("This is a debug message");
        trace!("This is a trace message");

        validate_logs(
            &logfile,
            &[
                ("ERROR", "test_specfile::a", "error"),
                ("WARN", "test_specfile::a", "warning"),
                ("INFO", "test_specfile::a", "info"),
                ("ERROR", "test_specfile::a", "error"),
                ("WARN", "test_specfile::a", "warning"),
                ("ERROR", "test_specfile::a", "error"),
            ],
        );
    }

    fn validate_logs(
        logfile: &str,
        expected: &[(&'static str, &'static str, &'static str)],
    ) -> bool {
        println!("log file = {}", logfile);

        let f = std::fs::File::open(logfile).unwrap();
        let mut reader = std::io::BufReader::new(f);

        let mut line = String::new();
        for tuple in expected {
            line.clear();
            reader.read_line(&mut line).unwrap();
            assert!(
                line.contains(&tuple.0),
                "Did not find tuple.0 = {}",
                tuple.0
            );
            assert!(
                line.contains(&tuple.1),
                "Did not find tuple.1 = {}",
                tuple.1
            );
            assert!(
                line.contains(&tuple.2),
                "Did not find tuple.2 = {}",
                tuple.2
            );
        }
        false
    }

}
