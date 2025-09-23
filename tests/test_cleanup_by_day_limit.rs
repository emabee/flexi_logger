mod test_utils;

#[test]
fn test_cleanup_by_day_limit() {
    use filetime::{set_file_mtime, FileTime};
    use flexi_logger::{Cleanup, Criterion, Duplicate, FileSpec, Logger, Naming};
    use log::*;
    use std::{
        fs, thread,
        time::{Duration, Instant, SystemTime},
    };

    let directory = test_utils::dir();

    Logger::try_with_str("info")
        .unwrap()
        .log_to_file(FileSpec::default().directory(&directory))
        .duplicate_to_stderr(Duplicate::Info)
        .rotate(
            Criterion::Size(100),
            Naming::Numbers,
            Cleanup::KeepForDays(1),
        )
        .start()
        .unwrap();

    // create four "full" log files (r00000, r00001, r00002, rCURRENT)
    for i in 0..12 {
        info!("log line {i}");
    }

    let mut log_files: Vec<_> = fs::read_dir(&directory)
        .unwrap()
        .filter_map(|r_dir_entry| {
            let p = r_dir_entry.unwrap().path();
            if p.extension().map(|ext| ext == "log").unwrap_or(false) {
                Some(p)
            } else {
                None
            }
        })
        .collect();
    log_files.sort();

    // artificially age the first file...
    let first_file = log_files.first().unwrap();
    let two_days_ago = SystemTime::now() - Duration::from_secs(2 * 24 * 3600);
    set_file_mtime(first_file, FileTime::from_system_time(two_days_ago)).unwrap();

    // ...and ensure it gets deleted automatically with the next rotation
    info!("add line to trigger the cleanup");
    let start = Instant::now();
    while first_file.exists() && start.elapsed().as_secs() < 2 {
        thread::sleep(Duration::from_millis(50));
    }
    assert!(
        !first_file.exists(),
        "old file should be deleted by day_limit cleanup"
    );
}
