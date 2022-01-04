mod test_utils;

#[cfg(feature = "external_rotation")]
use flexi_logger::{FileSpec, Logger, WriteMode};
#[cfg(feature = "external_rotation")]
use log::*;

#[cfg(feature = "external_rotation")]
#[cfg(feature = "async")]
const COUNT: u8 = 3;
#[cfg(feature = "external_rotation")]
#[cfg(not(feature = "async"))]
const COUNT: u8 = 2;

#[cfg(feature = "external_rotation")]
#[test]
fn test_external_file_remover() {
    if let Some(value) = test_utils::dispatch(COUNT) {
        work(value)
    }
}

#[cfg(feature = "external_rotation")]
fn work(value: u8) {
    let mut logger = Logger::try_with_str("info").unwrap();
    let file_spec = FileSpec::default()
        .directory(self::test_utils::dir())
        .suppress_timestamp()
        .basename("myprog");
    let file_spec_clone = file_spec.clone();
    logger = logger.log_to_file(file_spec);

    // ToDo: test with all write modes, with and without rotation
    match value {
        0 => {
            logger = logger.write_mode(WriteMode::Direct);
        }
        1 => {
            logger = logger.write_mode(WriteMode::BufferAndFlush);
        }
        #[cfg(feature = "async")]
        2 => {
            logger = logger.write_mode(WriteMode::Async);
        }
        COUNT..=u8::MAX => {
            unreachable!("dtrtgfg")
        }
    };

    let logger = logger.watch_external_rotations();

    let handle = logger
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    // write some log lines to initialize the file
    info!("XXX 1 AAA");
    info!("XXX 2 AAA");
    info!("XXX 3 AAA");
    handle.flush();

    // start a thread that deletes the output file
    trace!("Starting file remover thread");
    let worker_handle = std::thread::Builder::new()
        .name("file remover".to_string())
        .spawn(move || {
            for _ in 0..4 {
                std::thread::sleep(std::time::Duration::from_millis(400));
                // remove the log file
                match std::fs::remove_file(file_spec_clone.as_pathbuf(None)) {
                    Ok(()) => {
                        println!(
                            "Removed the log file {:?}",
                            file_spec_clone.as_pathbuf(None),
                        )
                    }
                    Err(e) => {
                        panic!(
                            "Cannot remove log file {:?}, due to {:?}",
                            file_spec_clone.as_pathbuf(None),
                            e
                        )
                    }
                }
            }
        })
        .unwrap();
    trace!("file remover thread started.");

    // write log lines in a slow loop
    for i in 0..200 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        info!("YYY {} AAA", i);
    }

    worker_handle.join().unwrap();

    // TODO: Verify that the error channel was not used
}
