#[cfg(feature = "specfile")]
extern crate flexi_logger;
#[cfg(feature = "specfile")]
#[macro_use]
extern crate log;

#[cfg(feature = "specfile")]
use flexi_logger::{detailed_format, Logger};
#[cfg(feature = "specfile")]
use std::{fs, thread, time};

#[cfg(feature = "specfile")]
#[cfg_attr(feature = "specfile", test)]
fn test_specfile() {
    let specfile = "./tests/logspec.toml";
    fs::remove_file(specfile).ok();
    Logger::with_str("mod4 = warn, modddddd9 = warn, debug, moddddd8 = warn, moddd6 = warn")
        .format(detailed_format)
        .start_with_specfile(specfile)
        .unwrap_or_else(|e| panic!("Logger initialization failed because: {}", e));

    // let wait = time::Duration::from_millis(1);
    // give yourself a real chance to update the specfile in between:
    let wait = time::Duration::from_millis(1000);
    for _ in 0..100 {
        thread::sleep(wait);
        error!("This is an error message");
        warn!("This is a warning");
        info!("This is an info message");
        debug!("This is a debug message");
        trace!("This is a trace message");
    }
}
