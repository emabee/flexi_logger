use log::*;

#[test]
fn test_default_files_dir_rot() {
    flexi_logger::Logger::with_str("info")
        .log_to_file()
        .directory("log_files")
        .rotate_over_size(2000)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");
}
