use crate::DeferredNow;
use log::Record;
use std::thread;
#[cfg(feature = "colors")]
use yansi::{Color, Paint, Style};

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
/// <code><span style="color:red">ERROR</span> &#91;my_prog::some_submodule&#93; <span
/// style="color:red">File not found</span></code>
///
/// See method [style](fn.style.html) if you want to influence coloring.
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
/// See method [style](fn.style.html) if you want to influence coloring.
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
/// See method [style](fn.style.html) if you want to influence coloring.
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
/// See method [style](fn.style.html) if you want to influence coloring.
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
/// The palette that is used by `style` can be overridden by setting the environment variable
/// `FLEXI_LOGGER_PALETTE` to a semicolon-separated list of numbers (0..=255) and/or dashes (´-´).
/// The first five values denote the fixed color that is used for coloring error, warning, info,
/// debug, and trace messages.
///
/// `FLEXI_LOGGER_PALETTE = "196;208;-;7;8"`
/// reflects the default palette; color 196 is used for error messages, and so on.
///
/// The '-' means that no coloring is done, i.e., with "-;-;-;-;-" all coloring is switched off.
///
/// For your convenience, if you want to specify your own palette,
/// you can produce a colored list of all 255 colors with `cargo run --example colors`
/// to see the available colors.
///
/// Only available with feature `colors`.
#[cfg(feature = "colors")]
pub fn style<T>(level: log::Level, item: T) -> Paint<T> {
    match level {
        log::Level::Error => Paint::new(item).with_style(PALETTE.error),
        log::Level::Warn => Paint::new(item).with_style(PALETTE.warn),
        log::Level::Info => Paint::new(item).with_style(PALETTE.info),
        log::Level::Debug => Paint::new(item).with_style(PALETTE.debug),
        log::Level::Trace => Paint::new(item).with_style(PALETTE.trace),
    }
}

#[cfg(feature = "colors")]
lazy_static::lazy_static! {
    static ref PALETTE: Palette = {
        match std::env::var("FLEXI_LOGGER_PALETTE") {
            Ok(palette) => Palette::parse(&palette).unwrap_or_else(|_| Palette::default()),
            Err(..) => Palette::default(),
        }

    };
}

#[cfg(feature = "colors")]
struct Palette {
    pub error: Style,
    pub warn: Style,
    pub info: Style,
    pub debug: Style,
    pub trace: Style,
}
#[cfg(feature = "colors")]
impl Palette {
    fn default() -> Palette {
        Palette {
            error: Style::new(Color::Fixed(196)).bold(),
            warn: Style::new(Color::Fixed(208)).bold(),
            info: Style::new(Color::Unset),
            debug: Style::new(Color::Fixed(7)),
            trace: Style::new(Color::Fixed(8)),
        }
    }

    fn parse(palette: &str) -> Result<Palette, std::num::ParseIntError> {
        let mut items = palette.split(';');
        Ok(Palette {
            error: parse_style(items.next().unwrap_or("196").trim())?,
            warn: parse_style(items.next().unwrap_or("208").trim())?,
            info: parse_style(items.next().unwrap_or("-").trim())?,
            debug: parse_style(items.next().unwrap_or("7").trim())?,
            trace: parse_style(items.next().unwrap_or("8").trim())?,
        })
    }
}

#[cfg(feature = "colors")]
fn parse_style(input: &str) -> Result<Style, std::num::ParseIntError> {
    Ok(if input == "-" {
        Style::new(Color::Unset)
    } else {
        Style::new(Color::Fixed(input.parse()?))
    })
}
