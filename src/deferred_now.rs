#[cfg(not(feature = "use_chrono_for_offset"))]
use crate::util::{eprint_err, ERRCODE};
#[cfg(feature = "use_chrono_for_offset")]
use chrono::{Local, Offset};

use std::sync::{Arc, Mutex};
#[cfg(feature = "syslog_writer")]
use time::Month;
use time::{formatting::Formattable, OffsetDateTime, UtcOffset};

/// Deferred timestamp creation.
///
/// Is used to ensure that a log record that is sent to multiple outputs
/// (in maybe different formats) always uses the same timestamp.
#[derive(Debug, Default)]
pub struct DeferredNow(Option<OffsetDateTime>);

impl DeferredNow {
    /// Constructs a new instance, but does not generate the timestamp.
    #[must_use]
    pub fn new() -> Self {
        Self(None)
    }

    /// Retrieve the timestamp.
    ///
    /// Requires mutability because the first caller will generate the timestamp.
    pub fn now(&mut self) -> &OffsetDateTime {
        self.0.get_or_insert_with(Self::now_local)
    }

    /// Convert into a formatted String.
    ///
    /// # Panics
    ///
    /// Panics if `fmt` has an inappropriate value.
    pub fn format(&mut self, fmt: &(impl Formattable + ?Sized)) -> String {
        self.now().format(fmt).unwrap(/* ok */)
    }

    #[cfg(feature = "syslog_writer")]
    pub(crate) fn format_rfc3339(&mut self) -> String {
        self.format(&time::format_description::well_known::Rfc3339)
    }

    // format_rfc3164: Mmm dd hh:mm:ss, where
    // mmm = one of "Jan, Feb, Mar, Apr, May, Jun, Jul, Aug, Sep, Oct, Nov, Dec",
    // dd = "xy" where x = " " or "1" or "2" or "3"
    // hh = "00" ... "23"
    // mm, ss= "00" ... "59"
    #[cfg(feature = "syslog_writer")]
    pub(crate) fn format_rfc3164(&mut self) -> String {
        let now = self.now();
        format!(
            "{mmm} {dd:>2} {hh:02}:{mm:02}:{ss:02}",
            mmm = match now.month() {
                Month::January => "Jan",
                Month::February => "Feb",
                Month::March => "Mar",
                Month::April => "Apr",
                Month::May => "May",
                Month::June => "Jun",
                Month::July => "Jul",
                Month::August => "Aug",
                Month::September => "Sep",
                Month::October => "Oct",
                Month::November => "Nov",
                Month::December => "Dec",
            },
            dd = now.day(),
            hh = now.hour(),
            mm = now.minute(),
            ss = now.second()
        )
    }

    /// Enforce the use of UTC rather than local time.
    ///
    /// By default, `flexi_logger` uses or tries to use local time.
    /// By calling early in your program either `Logger::use_utc()` or directly this method,
    /// you can override this to always use UTC.
    ///
    /// # Panics
    ///
    /// Panics if called too late, i.e., if [`DeferredNow::now`] was already called before on
    /// any instance of `DeferredNow`.
    pub fn force_utc() {
        let mut guard = FORCE_UTC.lock().unwrap();
        match *guard {
            Some(false) => {
                panic!("offset is already initialized not to enforce UTC");
            }
            Some(true) => {
                // is already set, nothing to do
            }
            None => *guard = Some(true),
        }
    }

    // Get the current timestamp, usually in local time.
    //
    // This method retrieves the timezone offset only once and caches it then.
    // This is to mitigate the issue of the `time` crate
    // (see their [CHANGELOG](https://github.com/time-rs/time/blob/main/CHANGELOG.md#035-2021-11-12))
    // that determining the offset is not safely working on linux,
    // and is not even tried there if the program is multi-threaded, or on other Unix-like systems.
    //
    // The method is called a first time during the initialization of `flexi_logger`,
    // and when the initialization is done while the program is single-threaded,
    // this should produce the right time offset in the trace output on linux.
    // On Windows and Mac there are no such limitations.
    //
    // If `Logger::use_utc()` is used, then this method will always return a UTC timestamp.
    #[doc(hidden)]
    #[must_use]
    pub fn now_local() -> OffsetDateTime {
        OffsetDateTime::now_utc().to_offset(*OFFSET)
    }
}

// Due to https://rustsec.org/advisories/RUSTSEC-2020-0159
// we obtain the offset only once and keep it here
lazy_static::lazy_static! {
    static ref OFFSET: UtcOffset = {
        let mut force_utc_guard = FORCE_UTC.lock().unwrap();
        if let Some(true) = *force_utc_guard { UtcOffset::UTC } else {
            if force_utc_guard.is_none() {
                *force_utc_guard = Some(false);
            }

            #[cfg(feature = "use_chrono_for_offset")]
            {
                let chrono_offset_seconds = Local::now().offset().fix().local_minus_utc();
                UtcOffset::from_whole_seconds(chrono_offset_seconds).unwrap(/* ok */)
            }
            #[cfg(not(feature = "use_chrono_for_offset"))]
            {
                match OffsetDateTime::now_local() {
                    Ok(ts) => {ts.offset()},
                    Err(e) => {
                        eprint_err(
                            ERRCODE::Time,
                            "flexi_logger has to work with UTC rather than with local time",
                            &e,
                        );
                        UtcOffset::UTC
                    }
                }
            }
        }
    };
}

// now_local() takes the offset from the lazy_static OFFSET, and this should be cheap.
// At the same time we want to influence the value in OFFSET based on whether Logger::use_utc()
// is used.
// Logger::use_utc() thus modifies the (expensive) lazy_static FORCE_UTC, and then the (cheap)
// lazy_static OFFSET is filled in the first invocation of now_local().
lazy_static::lazy_static! {
    static ref FORCE_UTC: Arc<Mutex<Option<bool>>> =
    Arc::new(Mutex::new(None));
}

#[cfg(test)]
mod test {
    #[test]
    fn test_deferred_now() {
        let mut deferred_now = super::DeferredNow::new();
        let once = deferred_now.now().to_string();
        println!("This should be the current timestamp: {}", once);
        std::thread::sleep(std::time::Duration::from_millis(300));
        let again = deferred_now.now().to_string();
        println!("This must be the same timestamp:      {}", again);
        assert_eq!(once, again);
    }

    #[cfg(feature = "syslog_writer")]
    #[test]
    fn test_format_rfc3164() {
        // println!(
        //     "{mmm} {dd:>2} {hh:02}:{mm:02}:{ss:02}",
        //     mmm = "Jan",
        //     dd = 1,
        //     hh = 2,
        //     mm = 3,
        //     ss = 4
        // );

        let mut deferred_now = super::DeferredNow::new();
        println!("rfc3164: {}", deferred_now.format_rfc3164());
    }
}
