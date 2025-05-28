use flexi_logger::{
    Age, Cleanup, Criterion, Duplicate, FileSpec, FlexiLoggerError, LevelFilter, Logger, Naming,
};
use std::{thread::sleep, time::Duration};

fn main() -> Result<(), FlexiLoggerError> {
    Logger::with(LevelFilter::Info)
        .rotate(
            Criterion::Age(Age::Second),
            Naming::TimestampsCustomFormat {
                current_infix: None,
                format: "%Y%m%d_%H%M%S",
            },
            Cleanup::Never,
        )
        .log_to_file(FileSpec::default())
        .duplicate_to_stdout(Duplicate::All)
        .start()?;

    log::info!("start");
    for step in 0..10 {
        log::info!("step {step}");
        sleep(Duration::from_millis(250));
    }
    log::info!("done");

    Ok(())
}
