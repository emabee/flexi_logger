#[cfg(feature = "specfile")]
mod a {
    use flexi_logger::{detailed_format, Logger};
    use log::*;
    use std::io::{BufRead, Write};
    use std::ops::Add;

    const WAIT: u64 = 2000;

    /// Rudimentary test of the specfile feature, using the file ./tests/logspec.toml.
    /// For real test, run this manually, change the duration before to a much higher value (see below),
    /// and edit the file while the test is running. You should see the impact immediately -
    /// by default, ERR, WARN, and INFO messages are printed. If you change the level in the file,
    /// less or more lines should be printed.
    #[test]
    fn test_specfile() {
        let specfile = "test_spec/test_specfile_logspec.toml";

        std::fs::remove_file(specfile).ok();
        assert!(!std::path::Path::new(specfile).exists());

        Logger::with_str("info")
            .format(detailed_format)
            .log_to_file()
            .suppress_timestamp()
            .start_with_specfile(specfile)
            .unwrap_or_else(|e| panic!("Logger initialization failed because: {}", e));

        error!("This is an error-0");
        warn!("This is a warning-0");
        info!("This is an info-0");
        debug!("This is a debug-0");
        trace!("This is a trace-0");

        eprintln!(
            "[{}] ===== behave like many editors: rename and recreate, as warn",
            chrono::Local::now()
        );
        {
            std::fs::rename(&specfile, "old_logspec.toml").unwrap();
            let mut file = std::fs::OpenOptions::new()
                .create(true)
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
        }

        std::thread::sleep(std::time::Duration::from_millis(WAIT));

        error!("This is an error-1");
        warn!("This is a warning-1");
        info!("This is an info-1");
        debug!("This is a debug-1");
        trace!("This is a trace-1");

        eprintln!(
            "[{}] ===== truncate and rewrite, update to error",
            chrono::Local::now()
        );
        {
            let mut file = std::fs::OpenOptions::new()
                .truncate(true)
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
        }

        std::thread::sleep(std::time::Duration::from_millis(WAIT));

        error!("This is an error-2");
        warn!("This is a warning-2");
        info!("This is an info-2");
        debug!("This is a debug-2");
        trace!("This is a trace-2");

        let logfile = std::path::Path::new(&std::env::args().nth(0).unwrap())
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string()
            .add(".log");

        validate_logs(
            &logfile,
            &[
                ("ERROR", "test_specfile::a", "error-0"),
                ("WARN", "test_specfile::a", "warning-0"),
                ("INFO", "test_specfile::a", "info-0"),
                ("ERROR", "test_specfile::a", "error-1"),
                ("WARN", "test_specfile::a", "warning-1"),
                ("ERROR", "test_specfile::a", "error-2"),
            ],
        );
    }

    fn validate_logs(logfile: &str, expected: &[(&'static str, &'static str, &'static str)]) {
        println!("validating log file = {}", logfile);

        let f = std::fs::File::open(logfile).unwrap();
        let mut reader = std::io::BufReader::new(f);

        let mut buf = String::new();
        for tuple in expected {
            buf.clear();
            reader.read_line(&mut buf).unwrap();
            assert!(buf.contains(&tuple.0), "Did not find tuple.0 = {}", tuple.0);
            assert!(buf.contains(&tuple.1), "Did not find tuple.1 = {}", tuple.1);
            assert!(buf.contains(&tuple.2), "Did not find tuple.2 = {}", tuple.2);
        }
        buf.clear();
        reader.read_line(&mut buf).unwrap();
        assert!(
            buf.is_empty(),
            "Found more log lines than expected: {} ",
            buf
        );
    }
}
