// Test code that is needed in "external" tests (in folder /tests) and
// and internal tests (in submodules) and benches

#![doc(hidden)]
use chrono::{DateTime, Local};
use std::path::PathBuf;

#[must_use]
pub fn dir() -> PathBuf {
    let folder = folder_for_log_files(PathBuf::new());
    match std::fs::create_dir_all(&folder) {
        Ok(()) => folder,
        Err(e) => {
            eprintln!(
                "Warning: Failed to pre-create directory {}: {e}",
                folder.display()
            );
            let temp_folder = folder_for_log_files(std::env::temp_dir());
            eprintln!("Trying temp_dir instead: {}", temp_folder.display());
            std::fs::create_dir_all(&temp_folder).unwrap(/* if this fails too, we panic */);
            temp_folder
        }
    }
}

#[must_use]
pub fn child_in_dir(s: &str) -> PathBuf {
    let mut d = dir();
    d.push(s);
    d
}

pub const TS: &str = "%Y-%m-%d_%H-%M-%S";

fn folder_for_log_files(mut d: PathBuf) -> PathBuf {
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
#[must_use]
pub fn now_local() -> DateTime<Local> {
    Local::now()
}
