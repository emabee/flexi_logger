#!/usr/bin/env rust-script
//! Cleans up all files and folders that were produced by test runs.
//!
//! ```cargo
//! [dependencies]
//! glob = "*"
//! ```
extern crate glob;

fn main() {
    for pattern in &[
        "./*.alerts",
        "./*.log",
        "./*.seclog",
        "./*logspec.toml",
        "./log_files/**/.DS_Store",
        "./log_files/**/test_restart_with_no_suffix-*",
        "./log_files/**/*.alerts",
        "./log_files/**/*.csv",
        "./log_files/**/*.err",
        "./log_files/**/*.gz",
        "./log_files/**/*.log",
        "./log_files/**/*.seclog",
        "./log_files/**/*.toml",
        "./server/**/*.toml",
    ] {
        for globresult in glob::glob(pattern).unwrap() {
            match globresult {
                Err(e) => eprintln!("Evaluating pattern {:?} produced error {}", pattern, e),
                Ok(pathbuf) => {
                    std::fs::remove_file(&pathbuf).unwrap();
                }
            }
        }
    }

    for dir_pattern in ["./log_files/**", "./server/**"] {
        let dirs: Vec<std::path::PathBuf> = glob::glob(dir_pattern)
            .unwrap()
            .filter_map(|r| match r {
                Err(e) => {
                    eprintln!("Searching for folders produced error {}", e);
                    None
                }
                Ok(_) => Some(r.unwrap()),
            })
            .collect();
        for pathbuf in dirs.iter().rev() {
            std::fs::remove_dir(&pathbuf).expect(&format!("folder not empty? {:?}", pathbuf));
        }
    }

    std::fs::remove_dir("./log_files/").ok();
    std::fs::remove_dir("./server/").ok();
    std::fs::remove_file("./link_to_log").ok();
    std::fs::remove_file("./link_to_mt_log").ok();
}
