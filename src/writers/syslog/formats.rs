use crate::DeferredNow;
use log::Record;

/// Default way of writing the message to the syslog.
///
/// Just uses the `Display` implementation of `record.args()`.
///
/// # Errors
///
/// `std:io::Error` from writing to the given output stram.
pub fn syslog_default_format(
    w: &mut dyn std::io::Write,
    _now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    write!(w, "{}", record.args())
}

/// Similar to `syslog_default_format`, but inserts the thread name in the beginning of the message,
/// encapsulated in square brackets.
///
/// # Errors
///
/// `std:io::Error` from writing to the given output stram.
pub fn syslog_format_with_thread(
    w: &mut dyn std::io::Write,
    _now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    write!(
        w,
        "[{}] {}",
        std::thread::current().name().unwrap_or("<unnamed>"),
        record.args()
    )
}
