#![allow(dead_code)]

use chrono::Local;
use std::path::PathBuf;

const CTRL_INDEX: &str = "CTRL_INDEX";

pub fn file(filename: &str) -> PathBuf {
    let mut f = dir();
    f.push(filename);
    f
}
pub fn dir() -> PathBuf {
    let mut d = PathBuf::new();
    d.push("log_files");
    add_prog_name(&mut d);
    d.push(format!("{}", Local::now().format("%Y-%m-%d_%H-%M-%S")));
    d
}
pub fn add_prog_name(pb: &mut PathBuf) {
    let path = PathBuf::from(std::env::args().next().unwrap());
    let filename = path.file_stem().unwrap(/*ok*/).to_string_lossy();

    // rsplit not available with rustc 1.46
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
                let mut command = std::process::Command::new(progname.to_string());
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
