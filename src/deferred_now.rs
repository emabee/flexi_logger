use chrono::{
    format::{DelayedFormat, StrftimeItems},
    DateTime, Local, Utc,
};
#[cfg(feature = "syslog_writer")]
use chrono::{Datelike, Timelike};
use std::sync::{Arc, Mutex};

/// Deferred timestamp creation.
///
/// Is used to ensure that a log record that is sent to multiple outputs
/// (in maybe different formats) always uses the same timestamp.
#[derive(Debug, Default)]
pub struct DeferredNow(Option<DateTime<Local>>);
impl<'a> DeferredNow {
    /// Constructs a new instance, but does not generate the timestamp.
    #[must_use]
    pub fn new() -> Self {
        Self(None)
    }

    /// Retrieve the timestamp for local time zone.
    ///
    /// Requires mutability because the first caller will generate the timestamp.
    pub fn now(&'a mut self) -> &'a DateTime<Local> {
        self.0.get_or_insert_with(Local::now)
    }

    /// Retrieve the UTC timestamp.
    ///
    /// Requires mutability because the first caller will generate the timestamp.
    pub fn now_utc_owned(&'a mut self) -> DateTime<Utc> {
        (*self.now()).into()
    }

    /// Produces a preformatted object suitable for printing.
    ///
    /// # Panics
    ///
    /// Panics if `fmt` has an inappropriate value.
    pub fn format<'b>(&'a mut self, fmt: &'b str) -> DelayedFormat<StrftimeItems<'b>> {
        if use_utc() {
            self.now_utc_owned().format(fmt)
        } else {
            self.now().format(fmt)
        }
    }

    /// Produces a preformatted object suitable for printing.
    ///
    /// The format described in RFC 3339 is used. Example: 1985-04-12T23:20:50.523Z
    pub fn format_rfc3339(&mut self) -> DelayedFormat<StrftimeItems<'_>> {
        self.format("%Y-%m-%dT%H:%M:%S%.3f%Z")
    }

    // format_rfc3164: Mmm dd hh:mm:ss, where
    // mmm = one of "Jan, Feb, Mar, Apr, May, Jun, Jul, Aug, Sep, Oct, Nov, Dec",
    // dd = "xy" where x = " " or "1" or "2" or "3"
    // hh = "00" ... "23"
    // mm, ss= "00" ... "59"
    #[cfg(feature = "syslog_writer")]
    pub(crate) fn format_rfc3164(&mut self) -> String {
        let (date, time) = if use_utc() {
            let now = self.now_utc_owned();
            (now.date_naive(), now.time())
        } else {
            let now = self.now();
            (now.date_naive(), now.time())
        };

        format!(
            "{mmm} {dd:>2} {hh:02}:{mm:02}:{ss:02}",
            mmm = match date.month() {
                1 => "Jan",
                2 => "Feb",
                3 => "Mar",
                4 => "Apr",
                5 => "May",
                6 => "Jun",
                7 => "Jul",
                8 => "Aug",
                9 => "Sep",
                10 => "Oct",
                11 => "Nov",
                12 => "Dec",
                _ => unreachable!(),
            },
            dd = date.day(),
            hh = time.hour(),
            mm = time.minute(),
            ss = time.second()
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

    // // Get the current timestamp, usually in local time.
    // #[doc(hidden)]
    // #[must_use]
    // pub fn now_local() -> DateTime<Local> {
    //     Local::now()
    // }
}

lazy_static::lazy_static! {
    static ref FORCE_UTC: Arc<Mutex<Option<bool>>> =
    Arc::new(Mutex::new(None));
}
fn use_utc() -> bool {
    let mut force_utc_guard = FORCE_UTC.lock().unwrap();
    if let Some(true) = *force_utc_guard {
        true
    } else {
        if force_utc_guard.is_none() {
            *force_utc_guard = Some(false);
        }
        false
    }
}

#[cfg(test)]
mod test {
    #[test]
    fn test_deferred_now() {
        let mut deferred_now = super::DeferredNow::new();
        let once = deferred_now.now().to_string();
        println!("This should be the current timestamp: {once}");
        std::thread::sleep(std::time::Duration::from_millis(300));
        let again = deferred_now.now().to_string();
        println!("This must be the same timestamp:      {again}");
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

    #[test]
    #[cfg(feature = "syslog_writer")]
    fn test_format_rfc3339() {
        // The format described in RFC 3339; example: 1985-04-12T23:20:50.52Z
        let s = super::DeferredNow::new().format_rfc3339().to_string();
        let bytes = s.clone().into_bytes();
        assert_eq!(bytes[4], b'-', "s = {s}");
        assert_eq!(bytes[7], b'-', "s = {s}");
        assert_eq!(bytes[10], b'T', "s = {s}");
        assert_eq!(bytes[13], b':', "s = {s}");
        assert_eq!(bytes[16], b':', "s = {s}");
        assert_eq!(bytes[19], b'.', "s = {s}");
        assert_eq!(bytes[23], b'+', "s = {s}");
        assert_eq!(bytes[26], b':', "s = {s}");
    }
}
