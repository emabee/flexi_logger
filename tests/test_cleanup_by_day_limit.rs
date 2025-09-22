mod test_utils;

#[test]
fn test_cleanup_by_day_limit() {
    use filetime::{set_file_mtime, FileTime};
    use flexi_logger::{Cleanup, Criterion, Duplicate, FileSpec, Logger, Naming};
    use log::*;
    use std::{fs, thread, time::Duration};

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

    for i in 0..5 {
        info!("log line {i}");
    }

    let mut log_files: Vec<_> = fs::read_dir(&directory)
        .unwrap()
        .filter_map(|e| {
            let p = e.unwrap().path();
            if p.extension().map(|ext| ext == "log").unwrap_or(false) {
                Some(p)
            } else {
                None
            }
        })
        .collect();
    let two_days_ago = std::time::SystemTime::now() - Duration::from_secs(2 * 24 * 3600);
    let ft = FileTime::from_system_time(two_days_ago);
    for file in &log_files {
        if file.exists() {
            let _ = set_file_mtime(file, ft);
        }
    }
    log_files.sort();
    let old_file = log_files.first().expect("should have log files");
    let two_days_ago = std::time::SystemTime::now() - Duration::from_secs(2 * 24 * 3600);
    let ft = FileTime::from_system_time(two_days_ago);
    if old_file.exists() {
        set_file_mtime(old_file, ft).unwrap();
    }

    for i in 0..3 {
        info!("trigger cleanup {i}");
        thread::sleep(Duration::from_millis(120));
    }

    let start = std::time::Instant::now();
    while old_file.exists() && start.elapsed().as_secs() < 2 {
        thread::sleep(Duration::from_millis(50));
    }
    assert!(
        !old_file.exists(),
        "old file should be deleted by day_limit cleanup"
    );
}
