use jiff::{fmt::strtime::Display, tz::TimeZone, ZonedDateTime};
use std::sync::{Mutex, OnceLock};

const FMT_RFC3339: &str = "%Y-%m-%dT%H:%M:%S%.3f%:z";

/// Deferred timestamp creation.
///
/// Is used to ensure that a log record that is sent to multiple outputs
/// (in maybe different formats) always uses the same timestamp.
#[derive(Debug, Default)]
pub struct DeferredNow(Option<ZonedDateTime>);
impl<'a> DeferredNow {
    /// Constructs a new instance, but does not generate the timestamp.
    #[must_use]
    pub fn new() -> Self {
        Self(None)
    }

    #[cfg(test)]
    #[must_use]
    fn new_from_datetime(dt: ZonedDateTime) -> Self {
        Self(Some(dt))
    }

    /// Retrieve the timestamp for local time zone.
    ///
    /// Requires mutability because the first caller will generate the timestamp.
    pub fn now(&'a mut self) -> &'a ZonedDateTime {
        self.0.get_or_insert_with(ZonedDateTime::now)
    }

    /// Retrieve the UTC timestamp.
    ///
    /// Requires mutability because the first caller will generate the timestamp.
    pub fn now_utc_owned(&mut self) -> ZonedDateTime {
        self.now().with_time_zone(TimeZone::UTC)
    }

    /// Produces a preformatted object suitable for printing.
    pub fn format<'b>(&mut self, fmt: &'b str) -> Display<'b> {
        if use_utc() {
            self.now_utc_owned().strftime(fmt)
        } else {
            self.now().strftime(fmt)
        }
    }

    /// Prints itself in a format compliant with RFC 3339.
    ///
    /// Example: 2021-04-29T13:14:15.678+01:00
    ///
    /// We do not use the Z variant of RFC 3339, because it is often misinterpreted.
    pub fn format_rfc3339(&mut self) -> String {
        self.format(FMT_RFC3339).to_string()
    }

    // format_rfc3164: Mmm dd hh:mm:ss, where
    // mmm = one of "Jan, Feb, Mar, Apr, May, Jun, Jul, Aug, Sep, Oct, Nov, Dec",
    // dd = "xy" where x = " " or "1" or "2" or "3"
    // hh = "00" ... "23"
    // mm, ss= "00" ... "59"
    #[cfg(feature = "syslog_writer")]
    pub(crate) fn format_rfc3164(&mut self) -> String {
        let dt = if use_utc() {
            self.now_utc_owned().datetime()
        } else {
            self.now().datetime()
        };

        format!(
            "{mmm} {dd:>2} {hh:02}:{mm:02}:{ss:02}",
            mmm = match dt.month() {
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
            dd = dt.day(),
            hh = dt.hour(),
            mm = dt.minute(),
            ss = dt.second()
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
        let mut cfg_force_utc = cfg_force_utc().lock().unwrap();
        match *cfg_force_utc {
            Some(false) => {
                panic!("offset is already initialized not to enforce UTC");
            }
            Some(true) => {
                // is already set, nothing to do
            }
            None => *cfg_force_utc = Some(true),
        }
    }
}

fn cfg_force_utc() -> &'static Mutex<Option<bool>> {
    static CFG_FORCE_UTC: OnceLock<Mutex<Option<bool>>> = OnceLock::new();
    CFG_FORCE_UTC.get_or_init(|| Mutex::new(None))
}

fn use_utc() -> bool {
    let mut cfg_force_utc = cfg_force_utc().lock().unwrap();
    if let Some(true) = *cfg_force_utc {
        true
    } else {
        if cfg_force_utc.is_none() {
            *cfg_force_utc = Some(false);
        }
        false
    }
}

#[cfg(test)]
pub(crate) fn set_force_utc(b: bool) {
    let mut cfg_force_utc = cfg_force_utc().lock().unwrap();
    *cfg_force_utc = Some(b);
}

#[cfg(test)]
mod test {
    use super::{DeferredNow, FMT_RFC3339};
    use jiff::{civil, tz, ZonedDateTime};

    #[test]
    fn test_timestamp_taken_only_once() {
        let mut deferred_now = super::DeferredNow::new();
        let once = deferred_now.now().clone();
        std::thread::sleep(std::time::Duration::from_millis(30));
        let again = deferred_now.now().clone();
        assert_eq!(once, again);
        println!("Now: {}", deferred_now.format("%Y-%m-%d %H:%M:%S%.6f %:z"));
        println!("Now: {}", once.strftime("%Y-%m-%d %H:%M:%S%.6f %:z"));
        println!("Now: {}", again.strftime("%Y-%m-%d %H:%M:%S%.6f %:z"));
    }

    fn utc_and_offset_timestamps() -> (ZonedDateTime, ZonedDateTime) {
        let dt = civil::date(2021, 4, 29).at(13, 14, 15, 678_000_000);
        (
            dt.to_zoned_datetime(tz::TimeZone::UTC).unwrap(),
            dt.to_zoned_datetime(tz::offset(1).to_time_zone()).unwrap(),
        )
    }
    fn get_deferred_nows() -> (DeferredNow, DeferredNow) {
        let (ts_utc, ts_plus1) = utc_and_offset_timestamps();
        (
            DeferredNow::new_from_datetime(ts_utc.with_time_zone(tz::TimeZone::system())),
            DeferredNow::new_from_datetime(ts_plus1.with_time_zone(tz::TimeZone::system())),
        )
    }

    #[test]
    fn test_rfc3339() {
        let (ts_utc, ts_plus1) = utc_and_offset_timestamps();

        assert_eq!(
            ts_utc.strftime(FMT_RFC3339).to_string(),
            "2021-04-29T13:14:15.678+00:00",
        );
        assert_eq!(
            ts_plus1.strftime(FMT_RFC3339).to_string(),
            "2021-04-29T13:14:15.678+01:00",
        );
    }

    #[test]
    fn test_formats() {
        #[cfg(feature = "syslog_writer")]
        {
            log::info!("test rfc3164");
            super::set_force_utc(true);
            let (mut dn1, mut dn2) = get_deferred_nows();
            assert_eq!("Apr 29 13:14:15", &dn1.format_rfc3164());
            assert_eq!("Apr 29 12:14:15", &dn2.format_rfc3164());
        }

        log::info!("test rfc3339");
        {
            // with local timestamps, offsets ≠ 0 are printed (except in Greenwich time zone):
            super::set_force_utc(false);
            let (mut dn1, mut dn2) = get_deferred_nows();
            log::info!("2021-04-29T15:14:15.678+02:00, {}", dn1.format_rfc3339());
            log::info!("2021-04-29T14:14:15.678+02:00, {}", dn2.format_rfc3339());

            // with utc, the timestamps are normalized to offset 0
            super::set_force_utc(true);
            let (mut dn1, mut dn2) = get_deferred_nows();
            assert_eq!("2021-04-29T13:14:15.678+00:00", dn1.format_rfc3339());
            assert_eq!("2021-04-29T12:14:15.678+00:00", dn2.format_rfc3339());
        }
    }
}
