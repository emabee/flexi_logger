extern crate flexi_logger;

#[macro_use]
extern crate log;

use flexi_logger::{opt_format,init,LogConfig};

#[test]
fn files_dir_dscr_rot() {
    let link_name = String::from("link_to_log");
    assert_eq!(
        (),
        init( LogConfig {
                 format: opt_format,
                 log_to_file: true,
                 directory: Some("log_files".to_string()),
                 discriminant: Some("foo".to_string()),
                 rotate_over_size: Some(2000),
                 create_symlink: Some(link_name.clone()),
                 .. LogConfig::new()
               }, Some("info".to_string()) ).unwrap()
    );

    error!("This is an error message");
    warn!("This is a warning");
    info!("This is an info message");
    debug!("This is a debug message - you must not see it!");
    trace!("This is a trace message - you must not see it!");
    self::platform::check_link(&link_name);
}

mod platform {
    #[cfg(target_os = "linux")]
    pub fn check_link(link_name: &String) {
        use std::fs;
        assert!(fs::metadata(link_name).is_ok());
    }

    #[cfg(not(target_os = "linux"))]
    pub fn check_link(_: &String) {}
}
