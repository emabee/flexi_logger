use flexi_logger::{
    filter::{LogLineFilter, LogLineWriter},
    DeferredNow, FlexiLoggerError,
};

fn main() -> Result<(), FlexiLoggerError> {
    flexi_logger::Logger::try_with_str("info")?
        .filter(Box::new(BarsOnly))
        .start()?;
    log::info!("barista");
    log::info!("foo"); // will be swallowed by the filter
    log::info!("bar");
    log::info!("gaga"); // will be swallowed by the filter
    Ok(())
}

pub struct BarsOnly;
impl LogLineFilter for BarsOnly {
    fn write(
        &self,
        now: &mut DeferredNow,
        record: &log::Record,
        log_line_writer: &dyn LogLineWriter,
    ) -> std::io::Result<()> {
        if record.args().to_string().contains("bar") {
            log_line_writer.write(now, record)?;
        }
        Ok(())
    }
}
