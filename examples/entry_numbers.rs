use flexi_logger::{DeferredNow, FlexiLoggerError, Logger};
use log::*;
use std::sync::atomic::{AtomicU32, Ordering};

// Produces
//      1 INFO [entry_numbers] first
//      2 WARN [entry_numbers] second
//      3 ERROR [entry_numbers] third
fn main() -> Result<(), FlexiLoggerError> {
    Logger::try_with_str("info")?.format(my_format).start()?;

    info!("first");
    warn!("second");
    error!("third");
    Ok(())
}

pub fn my_format(
    w: &mut dyn std::io::Write,
    _now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    static LINE: AtomicU32 = AtomicU32::new(1);
    write!(
        w,
        "{:>6} {} [{}] {}",
        LINE.fetch_add(1, Ordering::Relaxed),
        record.level(),
        record.module_path().unwrap_or("<unnamed>"),
        record.args()
    )
}
