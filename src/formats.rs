use crate::DeferredNow;
use log::Record;
use std::thread;

/// A logline-formatter that produces log lines like <br>
/// ```INFO [my_prog::some_submodule] Task successfully read from conf.json```
///
/// # Errors
///
/// See `std::write`
pub fn default_format(
    w: &mut dyn std::io::Write,
    _now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    write!(
        w,
        "{} [{}] {}",
        record.level(),
        record.module_path().unwrap_or("<unnamed>"),
        record.args()
    )
}

#[allow(clippy::doc_markdown)]
/// A colored version of the logline-formatter `default_format`
/// that produces log lines like <br>
/// <code><span style="color:red">ERROR</span>
/// &#91;my_prog::some_submodule&#93;
/// <span style="color:red">File not found</span></code>
///
/// Only available with feature `colors`.
///
/// # Errors
///
/// See `std::write`
#[cfg(feature = "colors")]
pub fn colored_default_format(
    w: &mut dyn std::io::Write,
    _now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    let level = record.level();
    write!(
        w,
        "{} [{}] {}",
        style(level, level),
        record.module_path().unwrap_or("<unnamed>"),
        style(level, record.args())
    )
}

/// A logline-formatter that produces log lines with timestamp and file location, like
/// <br>
/// ```[2016-01-13 15:25:01.640870 +01:00] INFO [src/foo/bar:26] Task successfully read from conf.json```
/// <br>
///
/// # Errors
///
/// See `std::write`
pub fn opt_format(
    w: &mut dyn std::io::Write,
    now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    write!(
        w,
        "[{}] {} [{}:{}] {}",
        now.now().format("%Y-%m-%d %H:%M:%S%.6f %:z"),
        record.level(),
        record.file().unwrap_or("<unnamed>"),
        record.line().unwrap_or(0),
        &record.args()
    )
}

/// A colored version of the logline-formatter `opt_format`.
///
/// Only available with feature `colors`.
///
/// # Errors
///
/// See `std::write`
#[cfg(feature = "colors")]
pub fn colored_opt_format(
    w: &mut dyn std::io::Write,
    now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    let level = record.level();
    write!(
        w,
        "[{}] {} [{}:{}] {}",
        style(level, now.now().format("%Y-%m-%d %H:%M:%S%.6f %:z")),
        style(level, level),
        record.file().unwrap_or("<unnamed>"),
        record.line().unwrap_or(0),
        style(level, &record.args())
    )
}

/// A logline-formatter that produces log lines like
/// <br>
/// ```[2016-01-13 15:25:01.640870 +01:00] INFO [foo::bar] src/foo/bar.rs:26: Task successfully read from conf.json```
/// <br>
/// i.e. with timestamp, module path and file location.
///
/// # Errors
///
/// See `std::write`
pub fn detailed_format(
    w: &mut dyn std::io::Write,
    now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    write!(
        w,
        "[{}] {} [{}] {}:{}: {}",
        now.now().format("%Y-%m-%d %H:%M:%S%.6f %:z"),
        record.level(),
        record.module_path().unwrap_or("<unnamed>"),
        record.file().unwrap_or("<unnamed>"),
        record.line().unwrap_or(0),
        &record.args()
    )
}

/// A colored version of the logline-formatter `detailed_format`.
///
/// Only available with feature `colors`.
///
/// # Errors
///
/// See `std::write`
#[cfg(feature = "colors")]
pub fn colored_detailed_format(
    w: &mut dyn std::io::Write,
    now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    let level = record.level();
    write!(
        w,
        "[{}] {} [{}] {}:{}: {}",
        style(level, now.now().format("%Y-%m-%d %H:%M:%S%.6f %:z")),
        style(level, record.level()),
        record.module_path().unwrap_or("<unnamed>"),
        record.file().unwrap_or("<unnamed>"),
        record.line().unwrap_or(0),
        style(level, &record.args())
    )
}

/// A logline-formatter that produces log lines like
/// <br>
/// ```[2016-01-13 15:25:01.640870 +01:00] T[taskreader] INFO [src/foo/bar:26] Task successfully read from conf.json```
/// <br>
/// i.e. with timestamp, thread name and file location.
///
/// # Errors
///
/// See `std::write`
pub fn with_thread(
    w: &mut dyn std::io::Write,
    now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    write!(
        w,
        "[{}] T[{:?}] {} [{}:{}] {}",
        now.now().format("%Y-%m-%d %H:%M:%S%.6f %:z"),
        thread::current().name().unwrap_or("<unnamed>"),
        record.level(),
        record.file().unwrap_or("<unnamed>"),
        record.line().unwrap_or(0),
        &record.args()
    )
}

/// A colored version of the logline-formatter `with_thread`.
///
/// Only available with feature `colors`.
///
/// # Errors
///
/// See `std::write`
#[cfg(feature = "colors")]
pub fn colored_with_thread(
    w: &mut dyn std::io::Write,
    now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    let level = record.level();
    write!(
        w,
        "[{}] T[{:?}] {} [{}:{}] {}",
        style(level, now.now().format("%Y-%m-%d %H:%M:%S%.6f %:z")),
        style(level, thread::current().name().unwrap_or("<unnamed>")),
        style(level, level),
        record.file().unwrap_or("<unnamed>"),
        record.line().unwrap_or(0),
        style(level, &record.args())
    )
}

/// Helper function that is used in the provided colored format functions.
///
/// Only available with feature `colors`.
#[cfg(feature = "colors")]
pub fn style<T>(level: log::Level, item: T) -> yansi::Paint<T> {
    match level {
        log::Level::Error => yansi::Paint::fixed(196, item).bold(),
        log::Level::Warn => yansi::Paint::fixed(208, item).bold(),
        log::Level::Info => yansi::Paint::new(item),
        log::Level::Debug => yansi::Paint::fixed(7, item),
        log::Level::Trace => yansi::Paint::fixed(8, item),
    }
}
