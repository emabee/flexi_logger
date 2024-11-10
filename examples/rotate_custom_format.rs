use flexi_logger::{
    sort_by_creation_date, Age, Cleanup, Criterion, CustomFormatter, Duplicate, FileSorter,
    FileSpec, FlexiLoggerError, LevelFilter, Logger, Naming,
};
use std::{thread::sleep, time::Duration};

fn format_infix(o_last_infix: Option<String>) -> String {
    let id = match o_last_infix {
        Some(infix) => {
            let id: usize = infix.parse().unwrap();
            id + 1
        }
        None => 0,
    };
    id.to_string()
}

fn main() -> Result<(), FlexiLoggerError> {
    Logger::with(LevelFilter::Info)
        .rotate(
            Criterion::Age(Age::Second),
            Naming::CustomFormat(CustomFormatter::new(format_infix)),
            Cleanup::KeepLogFiles(4),
        )
        .log_to_file(
            FileSpec::default()
                .directory(std::env::current_dir().unwrap().join("log_files"))
                .basename("app-log")
                .suffix("txt")
                .file_sorter(FileSorter::new(sort_by_creation_date)),
        )
        .duplicate_to_stdout(Duplicate::All)
        .start()?;

    log::info!("start");
    for step in 0..30 {
        log::info!("step {}", step);
        sleep(Duration::from_millis(250));
    }
    log::info!("done");

    Ok(())
}
