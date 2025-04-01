mod test_utils;

#[cfg(feature = "buffer_writer")]
#[test]
fn test_buffer_writer() {
    use flexi_logger::{opt_format, Logger, Snapshot};
    use log::*;

    const LIMIT: usize = 1000;
    let logger_handle = Logger::try_with_str("info")
        .unwrap()
        .log_to_buffer(LIMIT, Some(opt_format))
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

    // let background thread write many log lines, one every ms
    const NO_OF_LOGLINES: usize = 100;
    const LOG_INTERVAL: u64 = 1;
    let join_handle = std::thread::spawn(|| {
        for i in 0..NO_OF_LOGLINES {
            std::thread::sleep(std::time::Duration::from_millis(LOG_INTERVAL));
            info!("this is line >>{i}<<");
        }
    });

    // let main thread retrieve 20 snapshots
    const NO_OF_SNAPSHOTS: usize = 20;
    const SNAP_INTERVAL: u64 = 5;
    const LAST_SNAPSHOT: usize = NO_OF_SNAPSHOTS - 1;

    let mut snapshots = vec![Snapshot::new(); NO_OF_SNAPSHOTS];
    for snapshot in &mut snapshots {
        std::thread::sleep(std::time::Duration::from_millis(SNAP_INTERVAL));
        logger_handle.update_snapshot(snapshot).unwrap();
    }
    // let background thread end and join back
    join_handle.join().unwrap();

    // -> verify that each snapshot is smaller than the limit
    for snapshot in &snapshots {
        assert!(snapshot.text.len() < LIMIT);
    }

    // assert that the last one does not contain the first log line
    assert!(!snapshots[LAST_SNAPSHOT].text.contains(">>1<<"));

    // verify that no more update is done since no more logs are written
    logger_handle
        .update_snapshot(&mut snapshots[LAST_SNAPSHOT])
        .unwrap();
    assert!(!logger_handle
        .update_snapshot(&mut snapshots[LAST_SNAPSHOT])
        .unwrap());
}
