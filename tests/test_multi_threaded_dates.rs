mod test_utils;

use cond_sync::{CondSync, Other};
use flexi_logger::{
    Age, Cleanup, Criterion, DeferredNow, Duplicate, FileSpec, LogSpecification, Logger, Naming,
    TS_DASHES_BLANK_COLONS_DOT_BLANK,
};
use glob::glob;
use log::*;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    ops::Add,
    thread::JoinHandle,
};

const NO_OF_THREADS: usize = 5;
const NO_OF_LOGLINES_PER_THREAD: usize = 20_000;

// we use a special log line format that starts with a special string so that it is easier to
// verify that all log lines are written correctly
#[test]
fn test_multi_threaded_dates() {
    let directory = test_utils::dir();
    {
        let _stopwatch = test_utils::Stopwatch::default();
        let logger = Logger::try_with_str("debug")
            .unwrap()
            .log_to_file(FileSpec::default().directory(&directory))
            .format(test_format)
            .create_symlink("link_to_mt_log")
            .duplicate_to_stderr(Duplicate::Info)
            .rotate(
                Criterion::Age(Age::Second),
                Naming::Timestamps,
                Cleanup::Never,
            )
            .start()
            .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

        info!("create many log lines with a considerable number of threads, verify the log");

        let cond_sync = CondSync::new(0_usize);
        let worker_handles = start_worker_threads(NO_OF_THREADS, &cond_sync);
        cond_sync
            .wait_until(|value| *value == NO_OF_THREADS)
            .unwrap();

        logger.set_new_spec(LogSpecification::parse("trace").unwrap());

        join_all_workers(worker_handles);
    } // drop stopwatch

    verify_logs(&directory.display().to_string());
}

// Starts given number of worker threads and lets each execute `do_work`
fn start_worker_threads(no_of_workers: usize, cond_sync: &CondSync<usize>) -> Vec<JoinHandle<u8>> {
    let mut worker_handles: Vec<JoinHandle<u8>> = Vec::with_capacity(no_of_workers);
    trace!("Starting {} worker threads", no_of_workers);
    for thread_number in 0..no_of_workers {
        trace!("Starting thread {}", thread_number);
        let cond_sync_t = cond_sync.clone();
        worker_handles.push(
            std::thread::Builder::new()
                .name(thread_number.to_string())
                .spawn(move || {
                    do_work(thread_number, cond_sync_t);
                    0
                })
                .unwrap(),
        );
    }
    trace!("All {} worker threads started.", worker_handles.len());
    worker_handles
}

fn do_work(thread_number: usize, cond_sync: CondSync<usize>) {
    trace!("({})     Thread started working", thread_number);
    trace!("ERROR_IF_PRINTED");

    cond_sync
        .modify_and_notify(|value| *value += 1, Other::One)
        .unwrap();

    for idx in 0..NO_OF_LOGLINES_PER_THREAD {
        debug!("({})  writing out line number {}", thread_number, idx);
    }
    trace!("MUST_BE_PRINTED");
}

fn join_all_workers(worker_handles: Vec<JoinHandle<u8>>) {
    for worker_handle in worker_handles {
        worker_handle
            .join()
            .unwrap_or_else(|e| panic!("Joining worker thread failed: {e:?}"));
    }
    trace!("All worker threads joined.");
}

pub fn test_format(
    w: &mut dyn std::io::Write,
    now: &mut DeferredNow,
    record: &Record,
) -> std::io::Result<()> {
    write!(
        w,
        "XXXXX [{}] T[{:?}] {} [{}:{}] {}",
        now.format(TS_DASHES_BLANK_COLONS_DOT_BLANK),
        std::thread::current().name().unwrap_or("<unnamed>"),
        record.level(),
        record.file().unwrap_or("<unnamed>"),
        record.line().unwrap_or(0),
        &record.args()
    )
}

fn verify_logs(directory: &str) {
    // read all files
    let pattern = String::from(directory).add("/*");
    let globresults = match glob(&pattern) {
        Err(e) => panic!("Is this ({pattern}) really a directory? Listing failed with {e}",),
        Ok(globresults) => globresults,
    };
    let mut no_of_log_files = 0;
    let mut line_count = 0_usize;
    for globresult in globresults {
        let pathbuf = globresult.unwrap_or_else(|e| panic!("Ups - error occured: {e}"));
        let f = File::open(&pathbuf)
            .unwrap_or_else(|e| panic!("Cannot open file {pathbuf:?} due to {e}"));
        no_of_log_files += 1;
        let mut reader = BufReader::new(f);
        let mut buffer = String::new();
        while reader.read_line(&mut buffer).unwrap() > 0 {
            if buffer.starts_with("XXXXX") {
                line_count += 1;
            } else {
                panic!("irregular line in log file {pathbuf:?}: \"{buffer}\"");
            }
            buffer.clear();
        }
    }
    assert_eq!(
        line_count,
        NO_OF_THREADS * (NO_OF_LOGLINES_PER_THREAD + 1) + 3
    );
    println!(
        "Found {line_count} log lines from {NO_OF_THREADS} threads in {no_of_log_files} files",
    );
}
