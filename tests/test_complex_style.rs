extern crate flexi_logger;
extern crate log;

use flexi_logger::{detailed_format,init,LogConfig};

#[test]
fn test_complex_style() {
    init( LogConfig { log_to_file: true,
                      directory: Some("log_files".to_string()),
                      format: detailed_format,
                      .. LogConfig::new() },
          Some("myprog=debug,mylib=warn".to_string()) )
    .unwrap_or_else(|e|{panic!("Logger initialization failed with {}",e)});
}
