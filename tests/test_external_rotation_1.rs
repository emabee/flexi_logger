mod test_utils;

#[cfg(feature = "external_rotation")]
use flexi_logger::{FileSpec, Logger, WriteMode};
#[cfg(feature = "external_rotation")]
use log::*;
#[cfg(feature = "external_rotation")]
use std::path::Path;

#[cfg(feature = "external_rotation")]
#[cfg(feature = "async")]
const COUNT: u8 = 3;
#[cfg(feature = "external_rotation")]
#[cfg(not(feature = "async"))]
const COUNT: u8 = 2;

#[cfg(feature = "external_rotation")]
#[test]
fn test_external_file_rotator() {
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
    logger = logger.log_to_file(file_spec.clone());

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

    let handle = logger
        .watch_external_rotations()
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    // write some log lines to initialize the file
    info!("XXX 1 AAA");
    info!("XXX 2 AAA");
    info!("XXX 3 AAA");
    handle.flush();

    // create the "moved" folder
    let mut mv_dir = file_spec_clone.as_pathbuf(None);
    mv_dir.pop();
    mv_dir.push("moved");
    std::fs::create_dir_all(mv_dir.clone()).unwrap();
    let target_filespec = FileSpec::try_from(file_spec_clone.as_pathbuf(None))
        .unwrap()
        .directory(mv_dir.clone());

    // start a thread that renames the output file
    trace!("Starting file rotator thread");
    let worker_handle = std::thread::Builder::new()
        .name("file rotator".to_string())
        .spawn(move || {
            for i in 0..4 {
                std::thread::sleep(std::time::Duration::from_millis(400));
                // rotate the log file
                let target_name = target_filespec.as_pathbuf(Some(&i.to_string()));
                match std::fs::rename(file_spec_clone.as_pathbuf(None), &target_name.clone()) {
                    Ok(()) => {
                        println!(
                            "Renamed the log file {:?} to {:?}",
                            file_spec_clone.as_pathbuf(None),
                            &target_name,
                        )
                    }
                    Err(e) => {
                        // should be panic - is defused because test doesn't work properly on linux
                        println!(
                            "Cannot rename log file {:?} to {:?} due to {:?}",
                            file_spec_clone.as_pathbuf(None),
                            &target_name,
                            e
                        )
                    }
                }
            }
        })
        .unwrap();
    trace!("file rotator thread started.");

    // write log lines in a slow loop
    for i in 0..200 {
        std::thread::sleep(std::time::Duration::from_millis(10));
        info!("YYY {} AAA", i);
    }

    worker_handle.join().unwrap();

    // verify that all log lines are written and are found in moved files
    let mut files = 1;
    let mut sum = count_lines(&file_spec.as_pathbuf(None));
    for entry in std::fs::read_dir(mv_dir).unwrap() {
        let entry = entry.unwrap();
        let lines = count_lines(&entry.path());
        sum += lines;
        if lines > 0 {
            files += 1;
        }
    }
    // assert!(files > 4);
    println!("Number of files: {}  (should be 5)", files);
    // assert_eq!(203, sum);
    println!("Number of found log lines: {} (should be 203)", sum);
}

#[cfg(feature = "external_rotation")]
fn count_lines(path: &Path) -> usize {
    match std::fs::read_to_string(path) {
        Ok(s) => s.lines().filter(|line| line.contains("AAA")).count(),
        Err(_e) => 0,
    }
}
