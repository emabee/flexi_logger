extern crate flexi_logger;
#[macro_use]
extern crate log;

#[allow(deprecated)]
use flexi_logger::LogOptions;

#[test]
#[allow(deprecated)]
fn files_dir_rot() {
    assert_eq!(
        (),
        LogOptions::new()
            .log_to_file(true)
            .directory(Some("log_files".to_string()))
            .rotate_over_size(Some(2000))
            .init(Some("info".to_string()))
            .unwrap()
    );

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");
}
