#![allow(dead_code)]

#[cfg(feature = "use_chrono_for_offset")]
use chrono::{Local, Offset};
use std::path::PathBuf;
use time::{format_description::FormatItem, macros::format_description, OffsetDateTime, UtcOffset};

const CTRL_INDEX: &str = "CTRL_INDEX";

pub fn file(filename: &str) -> PathBuf {
    let mut f = dir();
    f.push(filename);
    f
}

const TS: &[FormatItem<'static>] =
    format_description!("[year]-[month]-[day]_[hour]-[minute]-[second]");

pub fn dir() -> PathBuf {
    let mut d = PathBuf::new();
    d.push("log_files");
    add_prog_name(&mut d);
    d.push(now_local().format(TS).unwrap());
    d
}
pub fn add_prog_name(pb: &mut PathBuf) {
    let path = PathBuf::from(std::env::args().next().unwrap());
    let filename = path.file_stem().unwrap(/*ok*/).to_string_lossy();

    // rsplit_once not available with rustc 1.51.0
    // let (progname, _) = filename.rsplit_once('-').unwrap_or((&filename, ""));
    let filename = filename.to_string();
    let progname = match filename.rfind('-') {
        Some(idx) => &filename[0..idx],
        None => filename.as_str(),
    };

    pb.push(progname);
}

// launch child process from same executable and sets there an additional environment variable
// or finds this environment variable and returns its value
pub fn dispatch(count: u8) -> Option<u8> {
    match std::env::var(CTRL_INDEX) {
        Err(_) => {
            println!("dispatcher");
            let progname = std::env::args().next().unwrap();
            let nocapture = std::env::args().any(|a| a == "--nocapture");
            for value in 0..count {
                let mut command = std::process::Command::new(progname.clone());
                if nocapture {
                    command.arg("--nocapture");
                }
                let status = command
                    .env(CTRL_INDEX, value.to_string())
                    .status()
                    .expect("Command failed to start");
                assert!(status.success());
            }
            None
        }
        Ok(value) => {
            println!("executor {}", value);
            Some(value.parse().unwrap())
        }
    }
}

// Due to https://rustsec.org/advisories/RUSTSEC-2020-0159
// we obtain the offset only once and keep it here
lazy_static::lazy_static! {
    static ref OFFSET: UtcOffset = utc_offset();
}

fn utc_offset() -> UtcOffset {
    #[cfg(feature = "use_chrono_for_offset")]
    return utc_offset_with_chrono();

    #[allow(unreachable_code)]
    utc_offset_with_time()
}

#[cfg(feature = "use_chrono_for_offset")]
fn utc_offset_with_chrono() -> UtcOffset {
    let chrono_offset_seconds = Local::now().offset().fix().local_minus_utc();
    UtcOffset::from_whole_seconds(chrono_offset_seconds).unwrap(/* ok */)
}

fn utc_offset_with_time() -> UtcOffset {
    match OffsetDateTime::now_local() {
        Ok(ts) => ts.offset(),
        Err(_) => {
            eprintln!("flexi_logger-test has to work with UTC rather than with local time",);
            UtcOffset::UTC
        }
    }
}

#[must_use]
pub fn now_local() -> OffsetDateTime {
    OffsetDateTime::now_utc().to_offset(*OFFSET)
}

pub struct Stopwatch(OffsetDateTime);
impl Default for Stopwatch {
    fn default() -> Self {
        Stopwatch(now_local())
    }
}
impl Drop for Stopwatch {
    fn drop(&mut self) {
        log::info!(
            "Task executed in {} ms.",
            (now_local() - self.0).whole_milliseconds()
        );
    }
}
