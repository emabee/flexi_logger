#![doc(html_logo_url = "http://www.rust-lang.org/logos/rust-logo-128x128-blk-v2.png",
       html_favicon_url = "http://www.rust-lang.org/favicon.ico",
       html_root_url = "http://doc.rust-lang.org/")]

//! A logger that can write the log to standard error or to a fresh file in a configurable folder
//! and allows custom logline formats.
//! It had started as an extended copy of [env_logger](http://rust-lang.github.io/log/env_logger/).
//!
//! # Usage
//!
//! This crate is on [crates.io](https://crates.io/crates/flexi_logger) and
//! can be used by adding `flexi_logger` to the dependencies in your
//! project's `Cargo.toml`.
//!
//! ```toml
//! [dependencies]
//! flexi_logger = "0.3"
//! log = "*"
//! ```
//!
//! and this to your crate root:
//!
//! ```text
//! #[macro_use]
//! extern crate log;
//! extern crate flexi_logger;
//! ```
//!
//! The latter is needed because flexi_logger plugs into the logging facade given by the
//! [log crate](http://rust-lang.github.io/log/log/).
//! i.e., you use the ```log``` macros to write log lines from your code.
//!
//! In flexi_logger's initialization, you can e.g.
//!
//! *  decide whether you want to write your logs to stderr (like with env_logger),
//!    or to a file,
//! *  configure the folder in which the log files are created,
//! *  provide the log-level-specification, i.e., the decision which log
//!    lines really should be written out, programmatically (if you don't want to
//!    use the environment variable RUST_LOG)
//! *  specify the line format for the log lines
//!
//! See function [init](fn.init.html) and structure [LogConfig](struct.LogConfig.html) for
//! a full description of all configuration options.

extern crate glob;
extern crate log;
extern crate regex;
extern crate time;

use glob::glob;
use log::{Log, LogLevel, LogLevelFilter, LogMetadata};
pub use log::LogRecord;
use regex::Regex;
use std::cell::RefCell;
use std::cmp::max;
use std::env;
use std::fmt;
use std::fs::{create_dir_all,File};
use std::io;
use std::io::{stderr, LineWriter, Write};
use std::ops::{Add,DerefMut};
use std::path::Path;
use std::sync::{Arc, Mutex};

macro_rules! print_err {
    ($($arg:tt)*) => (
        {
            use std::io::prelude::*;
            if let Err(e) = write!(&mut ::std::io::stderr(), "{}\n", format_args!($($arg)*)) {
                panic!("Failed to write to stderr.\
                    \nOriginal error output: {}\
                    \nSecondary error writing to stderr: {}", format!($($arg)*), e);
            }
        }
    )
}



// Encapsulation for LineWriter
struct LwHandle {
    olw: Option<LineWriter<File>>,
    o_filename_base: Option<String>,
    use_rotating: bool,
    written_bytes: usize,
    rotate_idx: usize,
}
impl LwHandle {
    fn new (config: &LogConfig) -> Result<LwHandle,FlexiLoggerError> {
        if !config.log_to_file {
            // we don't need a line-writer, so we return an empty handle
            return Ok(LwHandle{
                olw: None, o_filename_base: None, use_rotating: false, written_bytes: 0, rotate_idx: 0
            });
        }

        // make sure the folder exists or can be created
        let s_directory: String = match config.directory {
            Some(ref dir) => dir.clone(),
            None => ".".to_string()
        };
        let directory = Path::new(&s_directory);

        if let Err(e) = create_dir_all(&directory) {
            return Err(FlexiLoggerError::new(
                format!("Log cannot be written: output directory \"{}\" does not \
                exist and could not be created due to {}", &directory.display(),e)));
        };

        let o_filename_base = match std::fs::metadata(&directory) {
            Ok(metadata) => {
                if metadata.is_dir() {
                    Some(get_filename_base(&s_directory.clone(), & config.discriminant))
                } else {
                    return Err(FlexiLoggerError::new(
                        format!("Log cannot be written: output directory \"{}\" is not \
                        a directory", &directory.display())));
                }
            },
            Err(e) => {
                return Err(FlexiLoggerError::new(
                    format!("Log cannot be written: error accessing output directory \"{}\": {}",
                    &directory.display(), e)));
            }
        };

        let (use_rotating, rotate_idx) = match o_filename_base {
            None => (false, 0),
            Some(ref s_filename_base) => {
                match config.rotate_over_size {
                    None => (false, 0),
                    Some(_) => (true, get_next_rotate_idx(&s_filename_base, & config.suffix))
                }
            }
        };

        let mut lwh = LwHandle{
            olw: None,
            o_filename_base: o_filename_base,
            use_rotating: use_rotating,
            written_bytes: 0,
            rotate_idx: rotate_idx,
        };
        lwh.mount_linewriter(&config.suffix, config.print_message);
        Ok(lwh)
    }

    fn mount_linewriter(&mut self, suffix: &Option<String>, print_message: bool) {
        if let None = self.olw {
            if let Some(ref s_filename_base) = self.o_filename_base {
                let filename = get_filename(s_filename_base, self.use_rotating, self.rotate_idx, suffix);
                let path = Path::new(&filename);
                if print_message {
                    println!("Log is written to {}", path.display());
                }
                self.olw = Some(LineWriter::new(File::create(path.clone()).unwrap()));
            }
        }
    }
}

fn get_filename_base(s_directory: &String, discriminant: &Option<String>) -> String {
    let arg0 = env::args().next().unwrap();
    let progname = Path::new(&arg0).file_stem().unwrap().to_string_lossy();
    let mut filename = String::with_capacity(180).add(&s_directory).add("/").add(&progname);
    if let Some(ref s_d) = *discriminant {
        filename = filename.add(&format!("_{}", s_d));
    }
    filename
}

fn get_filename(s_filename_base: &String,
                do_rotating: bool,
                rotate_idx: usize,
                o_suffix: &Option<String>) -> String {
    let mut filename = String::with_capacity(180).add(&s_filename_base);
    if do_rotating {
        filename = filename.add(&format!("_r{:0>5}", rotate_idx));
    } else {
        filename = filename.add(&time::strftime("_%Y-%m-%d_%H-%M-%S",&time::now()).unwrap());
    }
    if let &Some(ref suffix) = o_suffix {
        filename = filename.add(".").add(suffix);
    }
    filename
}

fn get_filename_pattern(s_filename_base: &String, o_suffix: &Option<String>) -> String {
    let mut filename = String::with_capacity(180).add(&s_filename_base);
    filename = filename.add("_r*");
    if let &Some(ref suffix) = o_suffix {
        filename = filename.add(".").add(suffix);
    }
    filename
}

// FIXME error handling
fn get_next_rotate_idx(s_filename_base: & String, o_suffix: & Option<String>) -> usize {
    let fn_pattern = get_filename_pattern(s_filename_base, o_suffix);
    let paths = glob(&fn_pattern);
    let mut rotate_idx = 0;
    match paths {
        Err(e) => {
            panic!("Is this ({}) really a directory? Listing failed with {}", fn_pattern, e); // FIXME
        },
        Ok(it) => {
            for globresult in it {
                match globresult {
                    Err(e) => println!("Ups - error occured: {}", e),
                    Ok(pathbuf) => {
                        let filename = pathbuf.file_stem().unwrap().to_string_lossy();
                        let mut it = filename.rsplit("_r");
                        let idx: usize = it.next().unwrap().parse().unwrap_or(0);
                        rotate_idx = max(rotate_idx, idx);
                    }
                }
            }
        }
    }
    rotate_idx+1
}


pub struct FlexiLogger{
    directives: Vec<LogDirective>,
    o_filter: Option<Regex>,
    amo_line_writer: Arc<Mutex<RefCell<LwHandle>>>,
    config: LogConfig
}
impl FlexiLogger {
    pub fn new(loglevelspec: Option<String>, config: LogConfig)
            -> Result<FlexiLogger, FlexiLoggerError>  {

        let (mut directives, filter) = match loglevelspec {
            Some(ref llspec) => {let spec: &str = llspec; parse_logging_spec(&spec)},
            None => {
                match env::var("RUST_LOG") {
                    Ok(spec) => parse_logging_spec(&spec),
                    Err(..) => (vec![LogDirective { name: None, level: LogLevelFilter::Error }], None),
                }
            }
        };

        // Sort the provided directives by length of their name, this allows a
        // little more efficient lookup at runtime.
        directives.sort_by(|a, b| {
            let alen = a.name.as_ref().map(|a| a.len()).unwrap_or(0);
            let blen = b.name.as_ref().map(|b| b.len()).unwrap_or(0);
            alen.cmp(&blen)
        });

        let lwh = LwHandle::new(&config);
        match lwh {
            Ok(lwh) =>  Ok(FlexiLogger {
                            directives: directives,
                            o_filter: filter,
                            amo_line_writer: Arc::new(Mutex::new(RefCell::new(lwh))),
                            config: config }),
            Err(e) => Err(e)
        }
    }

    pub fn fl_enabled(&self, level: LogLevel, target: &str) -> bool {
        // Search for the longest match, the vector is assumed to be pre-sorted.
        for directive in self.directives.iter().rev() {
            match directive.name {
                Some(ref name) if !target.starts_with(&**name) => {},
                Some(..) | None => {
                    return level <= directive.level
                }
            }
        }
        false
    }
}

impl Log for FlexiLogger {
    fn enabled(&self, metadata: &LogMetadata) -> bool {
        self.fl_enabled(metadata.level(), metadata.target())
    }

    fn log(&self, record: &LogRecord) {
        if !Log::enabled(self, record.metadata()) {
            return;
        }

        if let Some(filter) = self.o_filter.as_ref() {
            if filter.is_match(&*record.args().to_string()) {
                return;
            }
        }

        let mut msg = (self.config.format)(record);
        if self.config.log_to_file {
            if self.config.duplicate_error && record.level() == LogLevel::Error
            || self.config.duplicate_info  && record.level() == LogLevel::Info {
                println!("{}",&record.args());
            }
            msg.push('\n');
            let msgb = msg.as_bytes();

            let amo_lw = self.amo_line_writer.clone();  // Arc<Mutex<RefCell<LwHandle>>>
            let mut mg_rc_lwh = amo_lw.lock().unwrap(); // MutexGuard<RefCell<LwHandle>>
            let rc_lwh = mg_rc_lwh.deref_mut();         // &mut RefCell<LwHandle>
            let mut rm_lwh = rc_lwh.borrow_mut();       // RefMut<LwHandle>
            let lwh: &mut LwHandle = rm_lwh.deref_mut();

            if lwh.use_rotating && (lwh.written_bytes > self.config.rotate_over_size.unwrap()) {
                lwh.olw = None;  // Hope that closes the previous lw
                lwh.written_bytes = 0;
                lwh.rotate_idx += 1;
                lwh.mount_linewriter(&self.config.suffix,  self.config.print_message);
            }
            if let Some(ref mut lw) = lwh.olw {
                &lw.write(msgb).unwrap_or_else( |e|{panic!("File logger: write failed with {}",e);} );//FIXME
                if lwh.use_rotating {
                    lwh.written_bytes += msgb.len();
                }
            };
        } else {
            let _ = writeln!(&mut io::stderr(), "{}", msg );
        }
    }
}


/// Describes errors in the initialization of flexi_logger.
#[derive(Debug)]
pub struct FlexiLoggerError {
    message: String
}
impl FlexiLoggerError {
    pub fn new(s: String) -> FlexiLoggerError {
        FlexiLoggerError {message: s}
    }
}
impl fmt::Display for  FlexiLoggerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}

/// Allows influencing the behavior of flexi_logger.
pub struct LogConfig {
    /// Allows providing a custom logline format; by default ```flexi_logger::default_format``` is used.
    /// You can either choose between two predefined variants, ```default_format``` and ```detailed_format```,
    /// or you create and use your own format function with the signature ```fn(&LogRecord) -> String```.

    pub format: fn(&LogRecord) -> String,

    /// * If `false` (default), the log is written to stderr.
    /// * If `true`, a new file is created and the log is written to it.
    /// The default pattern for the filename is '\<program_name\>\_\<date\>\_\<time\>.\<suffix\>',
    ///  e.g. `myprog_2015-07-08_10-44-11.log`.
    ///
    /// <p>Note that all following members of LogConfig are only relevant if this one is set to `true`.
    pub log_to_file: bool,

    /// If `true` (default), the name of the logfile is documented in a message to stdout.
    pub print_message: bool,

    /// If `true` (default), all logged error messages are duplicated to stdout.
    pub duplicate_error: bool,

    /// If `true` (default), all logged warning and info messages are also duplicated to stdout.
    pub duplicate_info: bool,

    /// Allows specifying a directory in which the log files are created. Default is ```None```.
    pub directory: Option<String>,

    /// Allows specifying the filesystem suffix of the log files (without the dot).  Default is ```log```.
    pub suffix: Option<String>,

    /// Allows specifying a maximum size for log files in bytes; when
    /// the specified file size is reached or exceeded, the file will be closed and a new one will be opened.
    /// The filename pattern changes - instead of the timestamp the serial number is included into the filename.
    pub rotate_over_size: Option<usize>,

    /// Allows specifying an additional part of the log file name that is inserted after the program name.
    pub discriminant: Option<String>,
}
impl LogConfig {
    /// initializes with
    ///
    /// *  log_to_file = false,
    /// *  print_message = true,
    /// *  duplicate_error = true,
    /// *  duplicate_info = false,
    /// *  format = flexi_logger::default_format,
    /// *  no directory (log files are created where the program was started),
    /// *  no rotate_over: log file grows indefinitely
    /// *  no discriminant: log file name consists only of progname, date or rotate_idx,  and suffix.
    /// *  suffix =  "log".

    pub fn new() -> LogConfig {
        LogConfig {
            log_to_file: false,
            print_message: true,
            duplicate_error: true,
            duplicate_info: false,
            format: default_format,
            directory: None,
            rotate_over_size: None,
            discriminant: None,
            suffix: Some("log".to_string()),
        }
    }
}

/// A logline-formatter that produces lines like <br>
/// ```INFO [my_prog::some_submodel] Task successfully read from conf.json```
pub fn default_format(record: &LogRecord) -> String {
    format!( "{} [{}] {}", record.level(), record.location().module_path(), record.args() )
}


/// A logline-formatter that produces lines like <br>
/// ```[2015-07-08 12:12:32:639785] INFO [my_prog::some_submodel] src/some_submodel.rs:26: Task successfully read from conf.json```
#[allow(unused)]
pub fn detailed_format(record: &LogRecord) -> String {
    let timespec = time::get_time(); // high-precision now
    let tm = time::at(timespec);     // formattable. but low-precision now
    let mut time: String = time::strftime("%Y-%m-%d %H:%M:%S:", &tm).unwrap();
    // ugly code to format milli and micro seconds
    let tmp = 1000000000 + timespec.nsec;
    let mut s = tmp.to_string();
    s.remove(9);s.remove(8);s.remove(7);s.remove(0);
    time = time.add(&s);
    format!( "[{}] {} [{}] {}:{}: {}",
                &time,
                record.level(),
                record.location().module_path(),
                record.location().file(),
                record.location().line(),
                &record.args())
}

struct LogDirective {
    name: Option<String>,
    level: LogLevelFilter,
}

/// Initializes the flexi_logger to your needs, and the global logger with flexi_logger.
///
/// Note: this should be called early in the execution of a Rust program. The
/// global logger may only be initialized once, subsequent initialization attempts
/// will return an error.
///
/// ## Configuration
///
/// See [LogConfig](struct.LogConfig.html) for most of the initialization options.
///
/// ## Log Level Specification
///
/// Specifying the log levels that you really want to see in a specific program run
/// can be done in the syntax defined by
/// [env_logger -> enabling logging](http://rust-lang.github.io/log/env_logger/#enabling-logging)
/// (from where this functionality was ruthlessly copied).
/// You can hand over the desired log-level-specification as an
/// initialization parameter to flexi_logger, or, if you don't do so,
/// with the environment variable RUST_LOG (as with env_logger).
/// Since using environment variables is on Windows not as comfortable as on linux,
/// you might consider using e.g. a docopt option for specifying the
/// log-Level-specification on the command line of your program.
///
///
/// ## Examples
///
/// ### Use defaults only
///
/// If you initialize flexi_logger with default settings, then it behaves like env_logger:
///
/// ```
/// use flexi_logger::{init,LogConfig};
///
/// init(LogConfig::new(), None).unwrap();
/// ```
///
/// ### Write to files, use a detailed log-line format
///
/// Here we configure flexi_logger to write log entries with fine-grained
/// time and location info into a log file in folder "log_files",
/// and we provide the loglevel-specification programmatically
/// as a ```Some<String>```, which might come in this form from what e.g. [docopt](https://crates.io/crates/docopt)
/// could provide for a respective command-line option:
///
/// ```
/// use flexi_logger::{detailed_format,init,LogConfig};
///
/// init( LogConfig { log_to_file: true,
///                   directory: Some("log_files".to_string()),
///                   format: detailed_format,
///                   .. LogConfig::new() },
///       Some("myprog=debug,mylib=warn".to_string()) )
/// .unwrap_or_else(|e|{panic!("Logger initialization failed with {}",e)});
/// ```
///
/// # Failures
///
/// Init returns a FlexiLoggerError, if it is supposed to write to an output file
/// but the file cannot be opened, e.g. because of operating system issues.
///
pub fn init(config: LogConfig, loglevelspec: Option<String>) -> Result<(),FlexiLoggerError> {
    let result = FlexiLogger::new(loglevelspec,config);
    match result {
        Ok(fl) => {
            let max = fl.directives.iter().map(|d| d.level).max().unwrap_or(LogLevelFilter::Off);
            log::set_logger( |max_level| {max_level.set(max);Box::new(fl)} )
                 .map_err(|e|{FlexiLoggerError::new(format!("Logger initialization failed due to {}", e))})
        },
        Err(e) => Err(e),
    }
}

/// Parse a logging specification string (e.g: "crate1,crate2::mod3,crate3::x=error/foo")
/// and return a vector with log directives.
fn parse_logging_spec(spec: &str) -> (Vec<LogDirective>, Option<Regex>) {
    let mut dirs = Vec::new();

    let mut parts = spec.split('/');
    let mods = parts.next();
    let filter = parts.next();
    if parts.next().is_some() {
        print_err!("warning: invalid logging spec '{}', ignoring it (too many '/'s)", spec);
        return (dirs, None);
    }
    mods.map(|m| { for s in m.split(',') {
        if s.len() == 0 { continue }
        let mut parts = s.split('=');
        let (log_level, name) = match (parts.next(), parts.next().map(|s| s.trim()), parts.next()) {
            (Some(part0), None, None) => {
                // if the single argument is a log-level string or number, treat that as a global fallback
                match part0.parse() {
                    Ok(num) => (num, None),
                    Err(_) => (LogLevelFilter::max(), Some(part0)),
                }
            }
            (Some(part0), Some(""), None) => (LogLevelFilter::max(), Some(part0)),
            (Some(part0), Some(part1), None) => {
                match part1.parse() {
                    Ok(num) => (num, Some(part0)),
                    _ => {
                        print_err!("warning: invalid logging spec '{}', ignoring it", part1);
                        continue
                    }
                }
            },
            _ => {
                print_err!("warning: invalid logging spec '{}', ignoring it", s);
                continue
            }
        };
        dirs.push(LogDirective {
            name: name.map(|s| s.to_string()),
            level: log_level,
        });
    }});

    let filter = filter.map_or(None, |filter| {
        match Regex::new(filter) {
            Ok(re) => Some(re),
            Err(e) => {
                print_err!("warning: invalid regex filter - {}", e);
                None
            }
        }
    });

    return (dirs, filter);
}



#[cfg(test)]
mod tests {
    use log::{LogLevel,LogLevelFilter};
    use super::{FlexiLogger, LogConfig,parse_logging_spec};

    fn make_logger(loglevelspec: &'static str) -> FlexiLogger {
        FlexiLogger::new(Some(loglevelspec.to_string()), LogConfig::new()).unwrap()
    }

    #[test]
    fn match_full_path() {
        let logger = make_logger("crate2=info,crate1::mod1=warn");
        assert!(logger.fl_enabled(LogLevel::Warn, "crate1::mod1"));
        assert!(!logger.fl_enabled(LogLevel::Info, "crate1::mod1"));
        assert!(logger.fl_enabled(LogLevel::Info, "crate2"));
        assert!(!logger.fl_enabled(LogLevel::Debug, "crate2"));
    }

    #[test]
    fn no_match() {
        let logger = make_logger("crate2=info,crate1::mod1=warn");
        assert!(!logger.fl_enabled(LogLevel::Warn, "crate3"));
    }

    #[test]
    fn match_beginning() {
        let logger = make_logger("crate2=info,crate1::mod1=warn");
        assert!(logger.fl_enabled(LogLevel::Info, "crate2::mod1"));
    }

    #[test]
    fn match_beginning_longest_match() {
        let logger = make_logger("crate2=info,crate2::mod=debug,crate1::mod1=warn");
        assert!(logger.fl_enabled(LogLevel::Debug, "crate2::mod1"));
        assert!(!logger.fl_enabled(LogLevel::Debug, "crate2"));
    }

    #[test]
    fn match_default() {
        let logger = make_logger("info,crate1::mod1=warn");
        assert!(logger.fl_enabled(LogLevel::Warn, "crate1::mod1"));
        assert!(logger.fl_enabled(LogLevel::Info, "crate2::mod2"));
    }

    #[test]
    fn zero_level() {
        let logger = make_logger("info,crate1::mod1=off");
        assert!(!logger.fl_enabled(LogLevel::Error, "crate1::mod1"));
        assert!(logger.fl_enabled(LogLevel::Info, "crate2::mod2"));
    }

    #[test]
    fn parse_logging_spec_valid() {
        let (dirs, filter) = parse_logging_spec("crate1::mod1=error,crate1::mod2,crate2=debug");
        assert_eq!(dirs.len(), 3);
        assert_eq!(dirs[0].name, Some("crate1::mod1".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::Error);

        assert_eq!(dirs[1].name, Some("crate1::mod2".to_string()));
        assert_eq!(dirs[1].level, LogLevelFilter::max());

        assert_eq!(dirs[2].name, Some("crate2".to_string()));
        assert_eq!(dirs[2].level, LogLevelFilter::Debug);
        assert!(filter.is_none());
    }

    #[test]
    fn parse_logging_spec_invalid_crate() {
        // test parse_logging_spec with multiple = in specification
        let (dirs, filter) = parse_logging_spec("crate1::mod1=warn=info,crate2=debug");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::Debug);
        assert!(filter.is_none());
    }

    #[test]
    fn parse_logging_spec_invalid_log_level() {
        // test parse_logging_spec with 'noNumber' as log level
        let (dirs, filter) = parse_logging_spec("crate1::mod1=noNumber,crate2=debug");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::Debug);
        assert!(filter.is_none());
    }

    #[test]
    fn parse_logging_spec_string_log_level() {
        // test parse_logging_spec with 'warn' as log level
        let (dirs, filter) = parse_logging_spec("crate1::mod1=wrong,crate2=warn");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::Warn);
        assert!(filter.is_none());
    }

    #[test]
    fn parse_logging_spec_empty_log_level() {
        // test parse_logging_spec with '' as log level
        let (dirs, filter) = parse_logging_spec("crate1::mod1=wrong,crate2=");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::max());
        assert!(filter.is_none());
    }

    #[test]
    fn parse_logging_spec_global() {
        // test parse_logging_spec with no crate
        let (dirs, filter) = parse_logging_spec("warn,crate2=debug");
        assert_eq!(dirs.len(), 2);
        assert_eq!(dirs[0].name, None);
        assert_eq!(dirs[0].level, LogLevelFilter::Warn);
        assert_eq!(dirs[1].name, Some("crate2".to_string()));
        assert_eq!(dirs[1].level, LogLevelFilter::Debug);
        assert!(filter.is_none());
    }

    #[test]
    fn parse_logging_spec_valid_filter() {
        let (dirs, filter) = parse_logging_spec("crate1::mod1=error,crate1::mod2,crate2=debug/abc");
        assert_eq!(dirs.len(), 3);
        assert_eq!(dirs[0].name, Some("crate1::mod1".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::Error);

        assert_eq!(dirs[1].name, Some("crate1::mod2".to_string()));
        assert_eq!(dirs[1].level, LogLevelFilter::max());

        assert_eq!(dirs[2].name, Some("crate2".to_string()));
        assert_eq!(dirs[2].level, LogLevelFilter::Debug);
        assert!(filter.is_some() && filter.unwrap().to_string() == "abc");
    }

    #[test]
    fn parse_logging_spec_invalid_crate_filter() {
        let (dirs, filter) = parse_logging_spec("crate1::mod1=error=warn,crate2=debug/a.c");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate2".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::Debug);
        assert!(filter.is_some() && filter.unwrap().to_string() == "a.c");
    }

    #[test]
    fn parse_logging_spec_empty_with_filter() {
        let (dirs, filter) = parse_logging_spec("crate1/a*c");
        assert_eq!(dirs.len(), 1);
        assert_eq!(dirs[0].name, Some("crate1".to_string()));
        assert_eq!(dirs[0].level, LogLevelFilter::max());
        assert!(filter.is_some() && filter.unwrap().to_string() == "a*c");
    }
}
