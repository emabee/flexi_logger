mod test_utils;

#[cfg(feature = "trc")]
mod a {
    use flexi_logger::{
        trc::FormatConfig, writers::FileLogWriter, Age, Cleanup, Criterion, FileSpec,
        LogSpecification, Naming, WriteMode,
    };
    use std::io::Write;
    use tracing::{debug, error, info, trace, warn};

    const WAIT_MILLIS: u64 = 1_500;

    #[test]
    fn test_specfile() {
        let specfile = super::test_utils::file("logspec.toml");

        std::fs::remove_file(&specfile).ok();
        assert!(!specfile.exists());

        let keep_alive_handles = flexi_logger::trc::setup_tracing(
            LogSpecification::info(),
            Some(&specfile),
            FileLogWriter::builder(FileSpec::default().directory(super::test_utils::dir()))
                .rotate(
                    Criterion::Age(Age::Day),
                    Naming::Timestamps,
                    Cleanup::KeepLogFiles(7),
                )
                .write_mode(WriteMode::Async),
            &FormatConfig::default()
                .with_ansi(false)
                .with_level(true)
                .with_target(true)
                .with_time(true),
        )
        .unwrap();

        assert!(specfile.exists());

        write_logs(0);
        super::b::write_logs(0);

        eprintln!(
            "[{}] ===== behave like many editors: rename and recreate; set to warn",
            super::test_utils::now_local()
        );
        {
            let mut old_name = specfile.clone();
            old_name.set_file_name("old_logspec.toml");
            std::fs::rename(&specfile, old_name).unwrap();
            let mut file = std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(&specfile)
                .unwrap();
            file.write_all(
                b"
                global_level = 'warn'\n\
                [modules]\n\
                'test_trc::b' = 'error'\n\
                ",
            )
            .unwrap();
        }

        std::thread::sleep(std::time::Duration::from_millis(WAIT_MILLIS));

        write_logs(1);
        super::b::write_logs(1);

        eprintln!(
            "[{}] ===== truncate and rewrite; set to error",
            super::test_utils::now_local()
        );
        {
            let mut file = std::fs::OpenOptions::new()
                .truncate(true)
                .write(true)
                .open(&specfile)
                .unwrap();
            file.write_all(
                "\
                global_level = 'error'\n\
                [modules]\n\
                'test_trc::b' = 'debug'\n\
                "
                .as_bytes(),
            )
            .unwrap();
        }

        std::thread::sleep(std::time::Duration::from_millis(WAIT_MILLIS));

        write_logs(2);
        super::b::write_logs(2);

        std::thread::sleep(std::time::Duration::from_millis(WAIT_MILLIS));

        keep_alive_handles.0.validate_logs(&[
            ("ERROR", "test_trc::a", "0"),
            ("WARN", "test_trc::a", "0"),
            ("INFO", "test_trc::a", "0"),
            ("ERROR", "test_trc::b", "0"),
            ("WARN", "test_trc::b", "0"),
            ("INFO", "test_trc::b", "0"),
            ("ERROR", "test_trc::a:", "1"),
            ("WARN", "test_trc::a:", "1"),
            ("ERROR", "test_trc::b:", "1"),
            ("ERROR", "test_trc::a:", "2"),
            ("ERROR", "test_trc::b:", "2"),
            ("WARN", "test_trc::b:", "2"),
            ("INFO", "test_trc::b:", "2"),
            ("DEBUG", "test_trc::b:", "2"),
        ]);
    }

    pub(crate) fn write_logs(idx: u8) {
        error!("Error from a::write_logs {idx}");
        warn!("Warning from a::write_logs {idx}");
        info!("Info from a::write_logs {idx}");
        debug!("Debug from a::write_logs {idx}");
        trace!("Trace from a::write_logs {idx}");
    }
}
mod b {
    use tracing::{debug, error, info, trace, warn};

    pub(crate) fn write_logs(idx: u8) {
        error!("Error from b::write_logs {idx}");
        warn!("Warning from b::write_logs {idx}");
        info!("Info from b::write_logs {idx}");
        debug!("Debug from b::write_logs {idx}");
        trace!("Trace from b::write_logs {idx}");
    }
}
