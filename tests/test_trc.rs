mod test_utils;

#[cfg(feature = "trc")]
mod a {
    use flexi_logger::{
        writers::FileLogWriter, Age, Cleanup, Criterion, FileSpec, LogSpecification, Naming,
        WriteMode,
    };
    use std::io::Write;
    use tracing::{debug, error, info, trace, warn};

    const WAIT_MILLIS: u64 = 2000;

    /// Test of the specfile feature
    #[test]
    fn test_specfile() {
        let specfile = super::test_utils::file("logspec.toml");

        std::fs::remove_file(&specfile).ok();
        assert!(!specfile.exists());

        let _keep_alive_handles = flexi_logger::trc::setup_tracing(
            LogSpecification::info(),
            Some(&specfile),
            FileLogWriter::builder(FileSpec::default().directory(super::test_utils::dir()))
                .rotate(
                    Criterion::Age(Age::Day),
                    Naming::Timestamps,
                    Cleanup::KeepLogFiles(7),
                )
                .write_mode(WriteMode::Async),
        )
        .unwrap();

        assert!(specfile.exists());

        error!("This is an error-0");
        warn!("This is a warning-0");
        info!("This is an info-0");
        debug!("This is a debug-0");
        trace!("This is a trace-0");

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
                global_level = 'warn'
                [modules]
                ",
            )
            .unwrap();
        }

        std::thread::sleep(std::time::Duration::from_millis(WAIT_MILLIS));

        error!("This is an error-1");
        warn!("This is a warning-1");
        info!("This is an info-1");
        debug!("This is a debug-1");
        trace!("This is a trace-1");

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
                "
                .as_bytes(),
            )
            .unwrap();
        }

        std::thread::sleep(std::time::Duration::from_millis(WAIT_MILLIS));

        error!("This is an error-2");
        warn!("This is a warning-2");
        info!("This is an info-2");
        debug!("This is a debug-2");
        trace!("This is a trace-2");
    }
}
