//! This module contains two traits which allow adding a stateful filter
//! using [`Logger::filter`](crate::Logger::filter).
//!
//! # Example
//!
//! ```rust
//! use flexi_logger::{
//!     filter::{LogLineFilter, LogLineWriter},
//!     DeferredNow, FlexiLoggerError,
//! };
//!
//! pub struct BarsOnly;
//! impl LogLineFilter for BarsOnly {
//!     fn write(
//!         &self,
//!         now: &mut DeferredNow,
//!         record: &log::Record,
//!         log_line_writer: &dyn LogLineWriter,
//!     ) -> std::io::Result<()> {
//!         if record.args().to_string().contains("bar") {
//!             log_line_writer.write(now, record)?;
//!         }
//!         Ok(())
//!     }
//! }
//!
//! fn main() -> Result<(), FlexiLoggerError> {
//!     flexi_logger::Logger::try_with_str("info")?
//!         .filter(Box::new(BarsOnly))
//!         .start()?;
//!     log::info!("barista");
//!     log::info!("foo"); // will be swallowed by the filter
//!     log::info!("bar");
//!     log::info!("gaga"); // will be swallowed by the filter
//!     Ok(())
//! }
//! ```
use crate::DeferredNow;
use log::Record;

/// Trait of the filter object.
#[allow(clippy::module_name_repetitions)]
pub trait LogLineFilter {
    /// Each log line that `flexi_logger` would write to the configured output channel is
    /// sent to this method.
    ///
    /// Note that the log line only appears in the configured output channel if the
    /// filter implementation forwards it to the provided `LogLineWriter`.
    ///
    /// # Errors
    ///
    /// If writing to the configured output channel fails.
    fn write(
        &self,
        now: &mut DeferredNow,
        record: &Record,
        log_line_writer: &dyn LogLineWriter,
    ) -> std::io::Result<()>;
}

/// Write out a single log line
pub trait LogLineWriter {
    /// Write out a log line to the configured output channel.
    ///
    /// # Errors
    ///
    /// If writing to the configured output channel fails.
    fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()>;
}
