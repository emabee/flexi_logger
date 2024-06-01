mod test_utils;

use flexi_logger::{Cleanup, Criterion, FileSpec, Logger, Naming};
use log::*;

const COUNT: u8 = 2;

#[test]
fn test_write_modes() {
    if let Some(value) = test_utils::dispatch(COUNT) {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        work(value)
    }
}

fn work(value: u8) {
    let directory = test_utils::dir();
    let file_spec = FileSpec::default()
        .directory(directory)
        .o_suffix(match value {
            0 => None,
            1 => Some("log".to_string()),
            COUNT..=u8::MAX => {
                unreachable!("got unexpected value {}", value)
            }
        });

    let _ = Logger::try_with_str("debug")
        .unwrap()
        .log_to_file(file_spec)
        .rotate(Criterion::Size(100), Naming::Timestamps, Cleanup::Never)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

    for _ in 0..100 {
        error!("This is an error message");
    }
}
