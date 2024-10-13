mod test_utils;

use flexi_logger::{Cleanup, Criterion, FileSpec, LogfileSelector, Logger, Naming};
use log::*;

const COUNT: u8 = 2;

#[test]
fn test_restart_with_no_suffix() {
    if let Some(value) = test_utils::dispatch(COUNT) {
        std::thread::sleep(std::time::Duration::from_millis(1000));
        work(value)
    }
}

fn work(value: u8) {
    let directory = test_utils::dir();
    let file_spec = FileSpec::default()
        .directory(directory.clone())
        .o_suffix(match value {
            0 => {
                println!("With suffix log");
                Some("log".to_string())
            }
            1 => {
                println!("Without suffix");
                None
            }
            COUNT..=u8::MAX => {
                unreachable!("got unexpected value {}", value)
            }
        });

    let logger = Logger::try_with_str("debug")
        .unwrap()
        .log_to_file(file_spec)
        .rotate(Criterion::Size(100), Naming::Timestamps, Cleanup::Never)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

    for i in 0..100 {
        error!("This is error message {i}");
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    // verify all log lines are found
    assert_eq!(100, test_utils::count_log_lines(&directory));

    // verify that no unexpected files are found
    match value {
        0 => assert_eq!(
            0,
            logger
                .existing_log_files(&LogfileSelector::default())
                .unwrap()
                .into_iter()
                .filter(
                    |pb| pb.extension().map(|oss| oss.to_string_lossy().to_string())
                        != Some(String::from("log"))
                )
                .count()
        ),
        1 => assert_eq!(
            0,
            logger
                .existing_log_files(&LogfileSelector::default())
                .unwrap()
                .into_iter()
                .filter(|pb| match pb.extension() {
                    Some(oss) => !oss.to_string_lossy().to_string().starts_with("restart"),
                    None => false,
                })
                .count()
        ),
        COUNT..=u8::MAX => {
            unreachable!("got unexpected value {}", value)
        }
    }
}
