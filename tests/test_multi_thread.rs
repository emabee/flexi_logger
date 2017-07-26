extern crate chrono;
extern crate glob;
extern crate flexi_logger;
#[macro_use]
extern crate log;

use chrono::Local;
use flexi_logger::{Logger, LogRecord};
use glob::glob;

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::ops::Add;
use std::thread::{self, JoinHandle};

const NO_OF_THREADS: usize = 5;
const NO_OF_LOGLINES_PER_THREAD: usize = 100_000;
const ROTATE_OVER_SIZE: usize = 4_000_000;

// cargo test multi_threaded -- --nocapture
#[test]
fn multi_threaded() {
    // we use a special log line format that starts with a special string so that it is easier to
    // verify that all log lines are written correctly

    let start = Local::now();
    let directory = define_directory();
    Logger::with_str("debug")
        .log_to_file()
        .format(test_format)
        .duplicate_info()
        .directory(directory.clone())
        .rotate_over_size(ROTATE_OVER_SIZE)
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
    info!("create a huge number of log lines with a significant number of threads, verify the \
        log");

    let worker_handles = start_worker_threads(NO_OF_THREADS);
    wait_for_workers_to_close(worker_handles);

    let delta = Local::now().signed_duration_since(start).num_milliseconds();
    info!("Task executed with {} threads in {}ms.", NO_OF_THREADS, delta);
    verify_logs(directory);
}

/// Starts given number of worker threads and lets each execute "do_work"
fn start_worker_threads(no_of_workers: usize) -> Vec<JoinHandle<u8>> {
    let mut worker_handles: Vec<JoinHandle<u8>> = Vec::with_capacity(no_of_workers);
    trace!("Starting {} worker threads", no_of_workers);
    for thread_number in 0..no_of_workers {
        trace!("Starting thread {}", thread_number);
        worker_handles.push(thread::Builder::new()
            .name(thread_number.to_string())
            .spawn(move || {
                do_work(thread_number);
                0 as u8
            })
            .unwrap());
    }
    trace!("All {} worker threads started.", worker_handles.len());
    worker_handles
}

fn do_work(thread_number: usize) {
    trace!("({})     Thread started working", thread_number);

    for idx in 0..NO_OF_LOGLINES_PER_THREAD {
        debug!("({})  writing out line number {}", thread_number, idx);
    }
}

fn wait_for_workers_to_close(worker_handles: Vec<JoinHandle<u8>>) {
    for worker_handle in worker_handles {
        worker_handle.join()
                     .unwrap_or_else(|e| panic!("Joining worker thread failed: {:?}", e));
    }
    trace!("All worker threads joined.");
}

fn define_directory() -> String {
    "./log_files/mt_logs/".to_string().add(&Local::now().format("%Y-%m-%d_%H-%M-%S").to_string())
}

fn test_format(record: &LogRecord) -> String {
    format!("XXXXX [{}] T[{:?}] {} [{}:{}] {}",
            Local::now().format("%Y-%m-%d %H:%M:%S%.6f %:z"),
            thread::current().name().unwrap_or("<unnamed>"),
            record.level(),
            record.location().file(),
            record.location().line(),
            &record.args())
}

fn verify_logs(directory: String) {
    // read all files
    let pattern = String::from(directory).add("/*");
    let globresults = match glob(&pattern) {
        Err(e) => panic!("Is this ({}) really a directory? Listing failed with {}", pattern, e),
        Ok(globresults) => globresults,
    };
    let mut no_of_log_files = 0;
    let mut line_count = 0_usize;
    for globresult in globresults {
        let pathbuf = globresult.unwrap_or_else(|e| panic!("Ups - error occured: {}", e));
        let f = File::open(&pathbuf)
            .unwrap_or_else(|e| panic!("Cannot open file {:?} due to {}", pathbuf, e));
        no_of_log_files += 1;
        let mut reader = BufReader::new(f);
        let mut buffer = String::new();
        while reader.read_line(&mut buffer).unwrap() > 0 {
            if buffer.starts_with("XXXXX") {
                line_count += 1;
            } else {
                assert!(false, format!("irregular line in log file {:?}: \"{}\"", pathbuf, buffer));
            }
            buffer.clear();
        }
    }
    assert_eq!(line_count, NO_OF_THREADS * NO_OF_LOGLINES_PER_THREAD + 2);
    info!("Wrote {} log lines from {} threads into {} files",
          NO_OF_THREADS * NO_OF_LOGLINES_PER_THREAD + 1,
          NO_OF_THREADS,
          no_of_log_files);
}
