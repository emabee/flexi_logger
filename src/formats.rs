use crate::DeferredNow;
use log::Record;
#[cfg(feature = "colors")]
use nu_ansi_term::{Color, Style};
use std::thread;

/// Time stamp format that is used by the provided format functions.
pub const TS_DASHES_BLANK_COLONS_DOT_BLANK: &str = "%Y-%m-%d %H:%M:%S%.6f %:z";

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

/// A colored version of the logline-formatter `default_format`
/// that produces log lines like <br>
/// <code><span style="color:red">ERROR</span> &#91;`my_prog::some_submodule`&#93; <span
/// style="color:red">File not found</span></code>
///
/// See method `[style](crate::style)` if you want to influence coloring.
///
/// # Errors
///
/// See `std::write`
#[cfg_attr(docsrs, doc(cfg(feature = "colors")))]
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
        style(level).paint(level.to_string()),
        record.module_path().unwrap_or("<unnamed>"),
        style(level).paint(record.args().to_string())
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
        now.format(TS_DASHES_BLANK_COLONS_DOT_BLANK),
        record.level(),
        record.file().unwrap_or("<unnamed>"),
        record.line().unwrap_or(0),
        &record.args()
    )
}

/// A colored version of the logline-formatter `opt_format`.
///
/// See method [style](crate::style) if you want to influence coloring.
///
/// # Errors
///
/// See `std::write`
#[cfg_attr(docsrs, doc(cfg(feature = "colors")))]
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
        style(level).paint(now.format(TS_DASHES_BLANK_COLONS_DOT_BLANK).to_string()),
        style(level).paint(level.to_string()),
        record.file().unwrap_or("<unnamed>"),
        record.line().unwrap_or(0),
        style(level).paint(&record.args().to_string())
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
        now.format(TS_DASHES_BLANK_COLONS_DOT_BLANK),
        record.level(),
        record.module_path().unwrap_or("<unnamed>"),
        record.file().unwrap_or("<unnamed>"),
        record.line().unwrap_or(0),
        &record.args()
    )
}

/// A colored version of the logline-formatter `detailed_format`.
///
/// See method [style](crate::style) if you want to influence coloring.
///
/// # Errors
///
/// See `std::write`
#[cfg_attr(docsrs, doc(cfg(feature = "colors")))]
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
        style(level).paint(now.format(TS_DASHES_BLANK_COLONS_DOT_BLANK).to_string()),
        style(level).paint(record.level().to_string()),
        record.module_path().unwrap_or("<unnamed>"),
        record.file().unwrap_or("<unnamed>"),
        record.line().unwrap_or(0),
        style(level).paint(&record.args().to_string())
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
        "[{}] T[{}] {} [{}:{}] {}",
        now.format(TS_DASHES_BLANK_COLONS_DOT_BLANK),
        thread::current().name().unwrap_or("<unnamed>"),
        record.level(),
        record.file().unwrap_or("<unnamed>"),
        record.line().unwrap_or(0),
        &record.args()
    )
}

/// A colored version of the logline-formatter `with_thread`.
///
/// See method [style](crate::style) if you want to influence coloring.
///
/// # Errors
///
/// See `std::write`
#[cfg_attr(docsrs, doc(cfg(feature = "colors")))]
#[cfg(feature = "colors")]
pub fn colored_with_thread(
    w: &mut dyn std::io::Write,
    now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error> {
    let level = record.level();
    write!(
        w,
        "[{}] T[{}] {} [{}:{}] {}",
        style(level).paint(now.format(TS_DASHES_BLANK_COLONS_DOT_BLANK).to_string()),
        style(level).paint(thread::current().name().unwrap_or("<unnamed>")),
        style(level).paint(level.to_string()),
        record.file().unwrap_or("<unnamed>"),
        record.line().unwrap_or(0),
        style(level).paint(&record.args().to_string())
    )
}

/// Helper function that is used in the provided coloring format functions to apply
/// colors based on the log level and the effective color palette.
///
/// See [`Logger::set_palette`](crate::Logger::set_palette) if you want to
/// modify the color palette.
#[allow(clippy::missing_panics_doc)]
#[cfg_attr(docsrs, doc(cfg(feature = "colors")))]
#[cfg(feature = "colors")]
#[must_use]
pub fn style(level: log::Level) -> Style {
    let palette = &*(PALETTE.read().unwrap());
    match level {
        log::Level::Error => palette.error,
        log::Level::Warn => palette.warn,
        log::Level::Info => palette.info,
        log::Level::Debug => palette.debug,
        log::Level::Trace => palette.trace,
    }
}

#[cfg(feature = "colors")]
lazy_static::lazy_static! {
    static ref PALETTE: std::sync::RwLock<Palette> = std::sync::RwLock::new(Palette::default());
}

// Overwrites the default PALETTE value either from the environment, if set,
// or from the parameter, if filled.
// Returns an error if parsing failed.
#[cfg(feature = "colors")]
pub(crate) fn set_palette(input: &Option<String>) -> Result<(), std::num::ParseIntError> {
    match std::env::var_os("FLEXI_LOGGER_PALETTE") {
        Some(ref env_osstring) => {
            *(PALETTE.write().unwrap()) = Palette::from(env_osstring.to_string_lossy().as_ref())?;
        }
        None => match input {
            Some(ref input_string) => {
                *(PALETTE.write().unwrap()) = Palette::from(input_string)?;
            }
            None => {}
        },
    }
    Ok(())
}

#[cfg(feature = "colors")]
#[derive(Debug)]
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
            error: Style::default().fg(Color::Fixed(196)),
            warn: Style::default().fg(Color::Fixed(208)),
            info: Style::default(),
            debug: Style::default().fg(Color::Fixed(27)),
            trace: Style::default().fg(Color::Fixed(8)),
        }
    }

    fn from(palette_string: &str) -> Result<Palette, std::num::ParseIntError> {
        let mut items = palette_string.split(';');
        Ok(Palette {
            error: parse_style(items.next().unwrap_or("196").trim())?,
            warn: parse_style(items.next().unwrap_or("208").trim())?,
            info: parse_style(items.next().unwrap_or("-").trim())?,
            debug: parse_style(items.next().unwrap_or("27").trim())?,
            trace: parse_style(items.next().unwrap_or("8").trim())?,
        })
    }
}

#[cfg(feature = "colors")]
fn parse_style(input: &str) -> Result<Style, std::num::ParseIntError> {
    Ok(if input == "-" {
        Style::new()
    } else {
        match input.strip_prefix('b') {
            None => Style::new().fg(Color::Fixed(input.parse()?)),
            Some(s) => Style::new().bold().fg(Color::Fixed(s.parse()?)),
        }
    })
}

/// Can be used in
/// [`Logger::adaptive_format_for_stderr`](crate::Logger::adaptive_format_for_stderr) and
/// [`Logger::adaptive_format_for_stdout`](crate::Logger::adaptive_format_for_stdout)
/// to use coloring only if the output goes to a tty.
///
/// This is helpful if the output is sometimes piped into other programs, which usually
/// do not expect color control byte sequences.
#[cfg_attr(docsrs, doc(cfg(feature = "is-terminal")))]
#[cfg(feature = "is-terminal")]
#[derive(Clone, Copy)]
pub enum AdaptiveFormat {
    /// Chooses between [`default_format`](crate::default_format)
    /// and [`colored_default_format`](crate::colored_default_format).
    #[cfg_attr(docsrs, doc(cfg(feature = "colors")))]
    #[cfg(feature = "colors")]
    Default,
    /// Chooses between [`detailed_format`](crate::detailed_format)
    /// and [`colored_detailed_format`](crate::colored_detailed_format).
    #[cfg_attr(docsrs, doc(cfg(feature = "colors")))]
    #[cfg(feature = "colors")]
    Detailed,
    /// Chooses between [`opt_format`](crate::opt_format)
    /// and [`colored_opt_format`](crate::colored_opt_format).
    #[cfg_attr(docsrs, doc(cfg(feature = "colors")))]
    #[cfg(feature = "colors")]
    Opt,
    /// Chooses between [`with_thread`](crate::with_thread)
    /// and [`colored_with_thread`](crate::colored_with_thread).
    #[cfg_attr(docsrs, doc(cfg(feature = "colors")))]
    #[cfg(feature = "colors")]
    WithThread,
    /// Chooses between the first format function (which is supposed to be uncolored)
    /// and the second (which is supposed to be colored).
    ///
    /// Allows providing own format functions, with freely choosable coloring technique,
    /// _and_ making use of the tty detection.
    Custom(FormatFunction, FormatFunction),
}

#[cfg(feature = "is-terminal")]
impl AdaptiveFormat {
    #[must_use]
    pub(crate) fn format_function(self, is_tty: bool) -> FormatFunction {
        if is_tty {
            match self {
                #[cfg(feature = "colors")]
                Self::Default => colored_default_format,
                #[cfg(feature = "colors")]
                Self::Detailed => colored_detailed_format,
                #[cfg(feature = "colors")]
                Self::Opt => colored_opt_format,
                #[cfg(feature = "colors")]
                Self::WithThread => colored_with_thread,
                Self::Custom(_, colored) => colored,
            }
        } else {
            match self {
                #[cfg(feature = "colors")]
                Self::Default => default_format,
                #[cfg(feature = "colors")]
                Self::Detailed => detailed_format,
                #[cfg(feature = "colors")]
                Self::Opt => opt_format,
                #[cfg(feature = "colors")]
                Self::WithThread => with_thread,
                Self::Custom(uncolored, _) => uncolored,
            }
        }
    }
}

/// Function type for format functions.
///
/// If you want to write the log lines in your own format,
/// implement a function with this signature and provide it to one of the methods
/// [`Logger::format()`](crate::Logger::format),
/// [`Logger::format_for_files()`](crate::Logger::format_for_files),
/// [`Logger::format_for_stdout()`](crate::Logger::format_for_stdout),
/// or [`Logger::format_for_stderr()`](crate::Logger::format_for_stderr).
///
/// Check out the code of the provided [format functions](index.html#functions)
/// if you want to start with a template.
///
/// ## Parameters
///
/// - `write`: the output stream
///
/// - `now`: the timestamp that you should use if you want a timestamp to appear in the log line
///
/// - `record`: the log line's content and metadata, as provided by the log crate's macros.
///
pub type FormatFunction = fn(
    write: &mut dyn std::io::Write,
    now: &mut DeferredNow,
    record: &Record,
) -> Result<(), std::io::Error>;

#[cfg(test)]
mod test {
    use crate::DeferredNow;

    #[test]
    fn test_opt_format() {
        let mut buf = Vec::<u8>::new();
        let w = &mut buf;
        let mut now = DeferredNow::new();

        let record = log::Record::builder()
            .file(Some("a"))
            .line(Some(1))
            .args(format_args!("test message"))
            .build();

        super::opt_format(w, &mut now, &record).unwrap();
        // [2016-01-13 15:25:01.640870 +01:00]
        assert_eq!(buf[0], b'[');
        assert_eq!(buf[5], b'-');
        assert_eq!(buf[8], b'-');
        assert_eq!(buf[11], b' ');
        assert_eq!(buf[14], b':');
        assert_eq!(buf[17], b':');
        assert_eq!(buf[20], b'.');
        assert_eq!(buf[27], b' ');
        assert_eq!(buf[28], b'+');
        assert_eq!(buf[31], b':');
        assert_eq!(buf[34], b']');

        let s = String::from_utf8(buf[35..].to_vec()).unwrap();
        assert_eq!(s.as_str(), " INFO [a:1] test message");
        println!("s: {s}");
    }
}
