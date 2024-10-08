mod test_utils;

#[cfg(feature = "compress")]
use flexi_logger::{Age, Cleanup, Criterion, Duplicate, FileSpec, Logger, Naming};
#[cfg(feature = "compress")]
use log::*;

#[cfg(feature = "compress")]
const COUNT: u8 = 4;

#[cfg(feature = "compress")]
#[test]
fn test_rotate_immediate_compression() {
    if let Some(value) = test_utils::dispatch(COUNT) {
        work(value)
    }
}

#[cfg(feature = "compress")]
fn work(value: u8) {
    match value {
        0 => test_variant(
            Naming::Timestamps,
            Criterion::Age(Age::Second),
            Cleanup::KeepCompressedFiles(100),
        ),
        1 => test_variant(
            Naming::TimestampsDirect,
            Criterion::Age(Age::Second),
            Cleanup::KeepCompressedFiles(100),
        ),
        2 => test_variant(
            Naming::Numbers,
            Criterion::Age(Age::Second),
            Cleanup::KeepCompressedFiles(100),
        ),
        3 => test_variant(
            Naming::NumbersDirect,
            Criterion::Age(Age::Second),
            Cleanup::KeepCompressedFiles(100),
        ),
        COUNT..=u8::MAX => unreachable!("asAS"),
    }
}

#[cfg(feature = "compress")]
fn test_variant(naming: Naming, criterion: Criterion, cleanup: Cleanup) {
    use std::time::{Duration, Instant};

    let directory = test_utils::dir();

    test_utils::wait_for_start_of_second();

    let mut written_lines = 1;
    {
        let logger = Logger::try_with_str("trace")
            .unwrap()
            .log_to_file(
                FileSpec::default()
                    .directory(&directory)
                    .suppress_basename(),
            )
            .format_for_files(flexi_logger::detailed_format)
            .format_for_stderr(flexi_logger::detailed_format)
            .duplicate_to_stderr(Duplicate::Info)
            .rotate(criterion, naming, cleanup)
            .start()
            .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

        info!(
            "test correct rotation by {}",
            match criterion {
                Criterion::Age(_) => "age",
                Criterion::AgeOrSize(_, _) => "age or size",
                Criterion::Size(_) => "size",
            }
        );

        let start = Instant::now();
        let max_runtime = Duration::from_millis(1_200);
        let sleep_time = Duration::from_millis(7);
        while Instant::now() - start < max_runtime {
            written_lines += 1;
            if written_lines % 17 == 4 {
                logger.trigger_rotation().unwrap();
            }
            trace!("line_count = {written_lines}");
            std::thread::sleep(sleep_time);
        }
    }

    let read_lines = test_utils::count_log_lines(&directory);
    assert_eq!(
        read_lines, written_lines,
        "wrong line count: {read_lines} instead of {written_lines}"
    );
}
