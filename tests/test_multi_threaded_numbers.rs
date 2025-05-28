mod test_utils;

use cond_sync::{CondSync, Other};
use flexi_logger::{
    Cleanup, Criterion, DeferredNow, Duplicate, FileSpec, LogSpecification, LogfileSelector,
    Logger, Naming, WriteMode, TS_DASHES_BLANK_COLONS_DOT_BLANK,
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
const ROTATE_OVER_SIZE: u64 = 800_000;

// we use a special log line format that starts with a special string so that it is easier to
// verify that all log lines are written correctly
#[test]
fn multi_threaded() {
    test_utils::wait_for_start_of_second();

    let directory = test_utils::dir();
    {
        let logger;
        let _stopwatch = test_utils::Stopwatch::default();
        logger = Logger::try_with_str("debug")
            .unwrap()
            .log_to_file(
                FileSpec::default()
                    .basename("test_mtn")
                    .directory(&directory),
            )
            .write_mode(WriteMode::BufferAndFlush)
            .format(test_format)
            .duplicate_to_stderr(Duplicate::Info)
            .rotate(
                Criterion::Size(ROTATE_OVER_SIZE),
                Naming::Numbers,
                Cleanup::Never,
            )
            .start()
            .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));
        info!(
            "create a huge number of log lines with a considerable number of threads, \
             verify the log"
        );

        let logger2 = logger.clone();

        let cond_sync = CondSync::new(0_usize);
        let worker_handles = start_worker_threads(NO_OF_THREADS, &cond_sync);
        cond_sync
            .wait_until(|value| *value == NO_OF_THREADS)
            .unwrap();

        std::thread::Builder::new()
            .spawn(move || {
                logger2.set_new_spec(LogSpecification::parse("trace").unwrap());
                0
            })
            .unwrap();
        wait_for_workers_to_close(worker_handles);

        let log_files = logger
            .existing_log_files(
                &LogfileSelector::default()
                    .with_compressed_files()
                    .with_r_current(),
            )
            .unwrap();
        assert_eq!(log_files.len(), 17);
        logger.parse_new_spec("info").unwrap();
        for f in log_files {
            trace!("Existing log file: {f:?}");
        }
    }
    verify_logs(&directory.display().to_string());
}

// Starts given number of worker threads and lets each execute `do_work`
fn start_worker_threads(no_of_workers: usize, cond_sync: &CondSync<usize>) -> Vec<JoinHandle<u8>> {
    let mut worker_handles: Vec<JoinHandle<u8>> = Vec::with_capacity(no_of_workers);
    trace!("(should not appear) Starting {no_of_workers} worker threads");
    for thread_number in 0..no_of_workers {
        trace!("(should not appear) Starting thread {thread_number}");
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
    trace!(
        "(should not appear) All {} worker threads started.",
        worker_handles.len()
    );
    worker_handles
}

fn do_work(thread_number: usize, cond_sync: CondSync<usize>) {
    trace!("(should not appear) ({thread_number})     Thread started working");
    cond_sync
        .modify_and_notify(|value| *value += 1, Other::One)
        .unwrap();
    for idx in 0..NO_OF_LOGLINES_PER_THREAD {
        debug!("({thread_number})  writing out line number {idx}");
    }
    trace!("MUST_BE_PRINTED");
}

fn wait_for_workers_to_close(worker_handles: Vec<JoinHandle<u8>>) {
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
            if buffer.starts_with("XXXXX") && !buffer.contains("should not appear") {
                line_count += 1;
            } else {
                panic!("irregular line in log file {pathbuf:?}: \"{buffer}\"");
            }
            buffer.clear();
        }
    }
    assert_eq!(
        line_count,
        NO_OF_THREADS * NO_OF_LOGLINES_PER_THREAD + NO_OF_THREADS + 3
    );
    println!(
        "Found {line_count} log lines from {NO_OF_THREADS} threads in {no_of_log_files} files",
    );
}
