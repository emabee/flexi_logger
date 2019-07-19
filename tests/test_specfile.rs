#[cfg(feature = "specfile")]
mod a {
    use flexi_logger::{detailed_format, Logger};
    use log::*;
    use std::io::{BufRead, Write};
    use std::ops::Add;

    const WAIT: u64 = 1100;

    /// Rudimentary test of the specfile feature, using the file ./tests/logspec.toml.
    /// For real test, run this manually, change the duration before to a much higher value (see below),
    /// and edit the file while the test is running. You should see the impact immediately -
    /// by default, ERR, WARN, and INFO messages are printed. If you change the level in the file,
    /// less or more lines should be printed.
    #[test]
    fn test_specfile() {
        let specfile = "tmp1/test_specfile_logspec.toml";

        std::fs::remove_file(specfile).ok();
        assert!(!std::path::Path::new(specfile).exists());

        Logger::with_str("info")
            .format(detailed_format)
            .log_to_file()
            .suppress_timestamp()
            .start_with_specfile(specfile)
            .unwrap_or_else(|e| panic!("Logger initialization failed because: {}", e));

        // eprintln!("Current specfile: \n{}\n",std::fs::read_to_string(specfile).unwrap());

        std::thread::sleep(std::time::Duration::from_millis(500));

        error!("This is an error message");
        warn!("This is a warning");
        info!("This is an info message");
        debug!("This is a debug message");
        trace!("This is a trace message");

        // eprintln!(
        //     "[{}]===== truncate and rewrite, update to warn",
        //     chrono::Local::now()
        // );
        // {
        //     let mut file = std::fs::OpenOptions::new()
        //         .truncate(true)
        //         .write(true)
        //         .open(specfile)
        //         .unwrap();
        //     file.write_all(
        //         b"
        //         global_level = 'warn'
        //         [modules]
        //         ",
        //     )
        //     .unwrap();
        // }

        eprintln!(
            "[{}]===== behave like many editors: rename and recreate, as warn",
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

        // eprintln!("Current specfile: \n{}\n",std::fs::read_to_string(specfile).unwrap());

        std::thread::sleep(std::time::Duration::from_millis(WAIT));

        error!("This is an error message");
        warn!("This is a warning");
        info!("This is an info message");
        debug!("This is a debug message");
        trace!("This is a trace message");

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

        // eprintln!(
        //     "[{}] ===== behave like many editors: rename and recreate, as error",
        //     chrono::Local::now()
        // );
        // {
        //     std::fs::rename(&specfile, "old_logspec.toml").unwrap();
        //     let mut file = std::fs::OpenOptions::new()
        //         .create(true)
        //         .write(true)
        //         .open(specfile)
        //         .unwrap();
        //     file.write_all(
        //         b"
        //         global_level = 'error'
        //         [modules]
        //         ",
        //     )
        //     .unwrap();
        // }

        // eprintln!("Current specfile: \n{}\n",std::fs::read_to_string(specfile).unwrap());

        std::thread::sleep(std::time::Duration::from_millis(WAIT));

        error!("This is an error message");
        warn!("This is a warning");
        info!("This is an info message");
        debug!("This is a debug message");
        trace!("This is a trace message");

        let logfile = std::path::Path::new(&std::env::args().nth(0).unwrap())
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string()
            .add(".log");

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
