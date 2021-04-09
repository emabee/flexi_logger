use flexi_logger::DeferredNow;
use flexi_logger::{
    colored_detailed_format, detailed_format, AdaptiveFormat, Duplicate, FileSpec, Logger,
};
use log::*;
use std::sync::atomic::AtomicU32;

#[test]
fn test_recursion() {
    let logger = Logger::with_str("info")
        .format(detailed_format)
        .log_to_file(FileSpec::default())
        .duplicate_to_stderr(Duplicate::All)
        .duplicate_to_stdout(Duplicate::All)
        .print_message()
        .format_for_stderr(colored_detailed_format);
    #[cfg(feature = "colors")]
    let logger =
        logger.adaptive_format_for_stdout(AdaptiveFormat::Custom(my_format, my_colored_format));
    logger
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed because: {}", e));

    let dummy = Dummy();

    for _ in 0..10 {
        error!("This is an error message for {}", dummy);
        warn!("This is a warning for {}", dummy);
        info!("This is an info message for {}", dummy);
        debug!("This is a debug message for {}", dummy);
        trace!("This is a trace message for {}", dummy);
    }
}

struct Dummy();
impl std::fmt::Display for Dummy {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        static COUNT: AtomicU32 = AtomicU32::new(0);
        info!(
            "Here comes the inner message ({}):-| ",
            COUNT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        );
        f.write_str("Dummy!!")?;
        Ok(())
    }
}

#[cfg(feature = "colors")]
pub fn my_colored_format(
    w: &mut dyn std::io::Write,
    _now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    let level = record.level();
    let style = yansi::Style::new(yansi::Color::Fixed(165));
    write!(
        w,
        "{} [{}] {}",
        style.paint(level),
        record.module_path().unwrap_or("<unnamed>"),
        style.paint(record.args())
    )
}
pub fn my_format(
    w: &mut dyn std::io::Write,
    _now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    let level = record.level();
    write!(
        w,
        "{} [{}] {}",
        level,
        record.module_path().unwrap_or("<unnamed>"),
        record.args()
    )
}
