extern crate flexi_logger;
#[macro_use]
extern crate log;

use flexi_logger::{opt_format, Logger};

#[test]
fn files_dir_dscr_rot() {
    let link_name = "link_to_log".to_string();
    let handle = Logger::with_str("info")
        .format(opt_format)
        .log_to_file()
        .directory("log_files")
        .discriminant("foo".to_string())
        .rotate_over_size(2000)
        .create_symlink(link_name.clone())
        .start_reconfigurable()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");
    handle.validate_logs(&[("ERROR", "error"), ("WARN", "warning"), ("INFO", "info")]);
    self::platform::check_link(&link_name);
}

mod platform {
    #[cfg(target_os = "linux")]
    pub fn check_link(link_name: &str) {
        use std::fs;
        assert!(fs::metadata(link_name).is_ok());
    }

    #[cfg(not(target_os = "linux"))]
    pub fn check_link(_: &str) {}
}
