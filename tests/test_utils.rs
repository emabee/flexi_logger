#![allow(dead_code)]

use chrono::{DateTime, Local};
use either::Either;
use flate2::read::GzDecoder;
#[cfg(feature = "compress")]
use std::ffi::OsStr;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    ops::Add,
    path::{Path, PathBuf},
};

const CTRL_INDEX: &str = "CTRL_INDEX";

pub fn file(filename: &str) -> PathBuf {
    let mut f = dir();
    f.push(filename);
    f
}

const TS: &str = "%Y-%m-%d_%H-%M-%S";

pub fn dir() -> PathBuf {
    let mut d = PathBuf::new();
    d.push("log_files");
    add_prog_name(&mut d);
    d.push(now_local().format(TS).to_string());
    d
}
fn add_prog_name(pb: &mut PathBuf) {
    let path = PathBuf::from(std::env::args().next().unwrap());
    let filename = path.file_stem().unwrap(/*ok*/).to_string_lossy();
    let (progname, _) = filename.rsplit_once('-').unwrap_or((&filename, ""));
    pb.push(progname);
}

// launch child processes from same executable and set for each of them an environment variable
// with a specific number, and then return None,
// or, in child processes, find this environment variable and return its value
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
            println!("executor {value}");
            Some(value.parse().unwrap())
        }
    }
}

#[must_use]
pub fn now_local() -> DateTime<Local> {
    Local::now()
}

pub struct Stopwatch(DateTime<Local>);
impl Default for Stopwatch {
    fn default() -> Self {
        Stopwatch(now_local())
    }
}
impl Drop for Stopwatch {
    fn drop(&mut self) {
        log::info!(
            "Task executed in {} ms.",
            (now_local() - self.0).num_milliseconds()
        );
    }
}

pub fn wait_for_start_of_second() {
    loop {
        let ms = Local::now().timestamp_subsec_millis();
        if ms < 50 {
            break;
        } else {
            std::thread::sleep(std::time::Duration::from_millis((1010_u32 - ms).into()));
        }
    }
}

pub fn wait_for_end_of_second() {
    loop {
        let ms = Local::now().timestamp_subsec_millis();
        if ms > 980 {
            break;
        } else {
            std::thread::sleep(std::time::Duration::from_millis((990_u32 - ms).into()));
        }
    }
}

// Count all log lines written in all .log and .log.gz files in the given folder
pub fn count_log_lines(directory: &Path) -> usize {
    // read all files
    let pattern = directory.display().to_string().add("/*");
    let all_files = match glob::glob(&pattern) {
        Err(e) => panic!("Is this ({pattern}) really a directory? Listing failed with {e}",),
        Ok(globresults) => globresults,
    };

    let mut total_line_count = 0_usize;
    for file in all_files.into_iter() {
        let pathbuf = file.unwrap_or_else(|e| panic!("Ups - error occured: {e}"));
        let mut reader: Either<BufReader<GzDecoder<File>>, BufReader<File>> =
            match pathbuf.extension() {
                #[cfg(feature = "compress")]
                Some(os_str) if os_str == AsRef::<OsStr>::as_ref("gz") => {
                    // unpack
                    Either::Left(BufReader::new(GzDecoder::new(
                        File::open(&pathbuf)
                            .unwrap_or_else(|e| panic!("Cannot open file {pathbuf:?} due to {e}")),
                    )))
                }
                _ => {
                    Either::Right(BufReader::new(File::open(&pathbuf).unwrap_or_else(|e| {
                        panic!("Cannot open file {pathbuf:?} due to {e}")
                    })))
                }
            };

        let mut buffer = String::new();
        let mut line_count = 0_usize;
        while reader.read_line(&mut buffer).unwrap() > 0 {
            line_count += 1;
        }
        total_line_count += line_count;
    }

    total_line_count
}
