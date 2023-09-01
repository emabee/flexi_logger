mod test_utils;

use flexi_logger::{FileSpec, Logger, WriteMode};
use log::*;
use std::path::Path;

#[cfg(feature = "async")]
const COUNT: u8 = 3;
#[cfg(not(feature = "async"))]
const COUNT: u8 = 2;

#[test]
fn test_external_rename() {
    if let Some(value) = test_utils::dispatch(COUNT) {
        work(value)
    }
}

fn work(value: u8) {
    let mut logger = Logger::try_with_str("info").unwrap();
    let file_spec = FileSpec::default()
        .directory(self::test_utils::dir())
        .suppress_timestamp()
        .basename("myprog");
    let file_path = file_spec.as_pathbuf(None);
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

    // create the "moved" folder
    let mut mv_dir = file_path.clone();
    mv_dir.pop();
    mv_dir.push("moved");
    std::fs::create_dir_all(mv_dir.clone()).unwrap();
    let target_filespec = FileSpec::try_from(&file_path)
        .unwrap()
        .directory(mv_dir.clone());
    {
        let logger = logger
            .start()
            .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

        // write some log lines to initialize the file
        info!("XXX 1 AAA");
        info!("XXX 2 AAA");
        info!("XXX 3 AAA");

        // write log lines in a slow loop, and rename the log file intermittently
        for i in 0..100 {
            if i % 25 == 20 {
                let target_path = target_filespec.as_pathbuf(Some(&i.to_string()));
                match std::fs::rename(file_path.clone(), target_path.clone()) {
                    Ok(()) => {
                        println!("Renamed the log file {:?} to {:?}", file_path, &target_path);
                        logger.reopen_output().unwrap();
                    }
                    Err(e) => {
                        panic!(
                            "Cannot rename log file {file_path:?} to {target_path:?} due to {e:?}",
                        )
                    }
                }
            }

            std::thread::sleep(std::time::Duration::from_millis(10));
            info!("YYY {} AAA", i);
        }
    }

    // verify that all log lines are written and are found in moved files
    let mut files = 1;
    let mut sum = count_lines(&file_path);
    for entry in std::fs::read_dir(mv_dir).unwrap() {
        let entry = entry.unwrap();
        let lines = count_lines(&entry.path());
        sum += lines;
        if lines > 0 {
            files += 1;
        }
    }
    assert_eq!(files, 5, "wrong number of files");
    assert_eq!(sum, 103, "wrong number of log lines");
}

fn count_lines(path: &Path) -> usize {
    match std::fs::read_to_string(path) {
        Ok(s) => s.lines().filter(|line| line.contains("AAA")).count(),
        Err(_e) => 0,
    }
}
