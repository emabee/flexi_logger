use crate::util::{eprint_err, ERRCODE};
use time::{formatting::Formattable, OffsetDateTime, UtcOffset};

/// Deferred timestamp creation.
///
/// Is used to ensure that a log record that is sent to multiple outputs
/// (in maybe different formats) always uses the same timestamp.
#[derive(Debug)]
pub struct DeferredNow(Option<OffsetDateTime>);
impl Default for DeferredNow {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> DeferredNow {
    /// Constructs a new instance, but does not generate the timestamp.
    #[must_use]
    pub fn new() -> Self {
        Self(None)
    }

    /// Retrieve the timestamp.
    ///
    /// Requires mutability because the first caller will generate the timestamp.
    pub fn now(&'a mut self) -> &'a OffsetDateTime {
        self.0.get_or_insert_with(now_local_or_utc)
    }

    /// Convert into a formatted String.
    ///
    /// # Panics
    ///
    /// if fmt has an inappropriate value
    pub fn format(&'a mut self, fmt: &(impl Formattable + ?Sized)) -> String {
        self.now().format(fmt).unwrap(/* ok */)
    }

    #[cfg(feature = "syslog_writer")]
    pub(crate) fn format_rfc3339(&mut self) -> String {
        self.format(&time::format_description::well_known::Rfc3339)
    }
}

/// Function used to determine the current timestamp.
///
/// Due to the issue of the `time` crate
/// (see their [CHANGELOG](https://github.com/time-rs/time/blob/main/CHANGELOG.md#035-2021-11-12))
/// that determining the offset is not safely working on linux,
/// and is not even tried there if the program is multi-threaded, this method retrieves the
/// offset only once and caches it then.
/// The method is called now during the initialization of `flexi_logger`, so when you do this while
/// the program is single-threaded, you'll likely get the right time offset in yor trace output
/// even on linux.
#[must_use]
pub fn now_local_or_utc() -> OffsetDateTime {
    lazy_static::lazy_static! {
        static ref OFFSET: UtcOffset = match OffsetDateTime::now_local() {
            Err(e) => {
                eprint_err(
                    ERRCODE::Time,
                    "flexi_logger has to work with UTC rather than with local time",
                    &e,
                );
                UtcOffset::UTC
            }
            Ok(ts) => ts.offset(),
        };
    }
    OffsetDateTime::now_utc().to_offset(*OFFSET)
}

#[cfg(test)]
mod test {
    #[test]
    fn test_deferred_now() {
        let mut deferred_now = super::DeferredNow::new();
        let now = deferred_now.now().to_string();
        println!("This should be the current timestamp: {}", now);
        std::thread::sleep(std::time::Duration::from_millis(300));
        let again = deferred_now.now().to_string();
        println!("This must be the same timestamp:      {}", again);
        assert_eq!(now, again);
    }
}
