mod test_utils;

use flexi_logger::{
    Cleanup, Criterion, DeferredNow, Duplicate, FileSpec, LogSpecification, Logger, Naming, Record,
    WriteMode, TS_DASHES_BLANK_COLONS_DOT_BLANK,
};
use glob::glob;
use log::*;
use std::{
    fs::File,
    io::{BufRead, BufReader},
    ops::Add,
    thread::JoinHandle,
};
use termcolor::WriteColor;

const NO_OF_THREADS: usize = 5;
const NO_OF_LOGLINES_PER_THREAD: usize = 20_000;
const ROTATE_OVER_SIZE: u64 = 800_000;

#[test]
fn multi_threaded() {
    // we use a special log line format that starts with a special string so that it is easier to
    // verify that all log lines are written correctly

    let directory = test_utils::dir();
    {
        let _stopwatch = test_utils::Stopwatch::default();
        let logger = Logger::try_with_str("debug")
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
            .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
        info!(
            "create a huge number of log lines with a considerable number of threads, \
             verify the log"
        );

        // clippy ignores the Drop implementation of the inner log handle :-(
        #[allow(clippy::redundant_clone)]
        let logger2 = logger.clone();
        let worker_handles = start_worker_threads(NO_OF_THREADS);
        let new_spec = LogSpecification::parse("trace").unwrap();
        std::thread::Builder::new()
            .spawn(move || {
                std::thread::sleep(std::time::Duration::from_millis(500));
                logger2.set_new_spec(new_spec);
                0
            })
            .unwrap();
        wait_for_workers_to_close(worker_handles);
    }
    verify_logs(&directory.display().to_string());
}

// Starts given number of worker threads and lets each execute `do_work`
fn start_worker_threads(no_of_workers: usize) -> Vec<JoinHandle<u8>> {
    let mut worker_handles: Vec<JoinHandle<u8>> = Vec::with_capacity(no_of_workers);
    trace!("Starting {} worker threads", no_of_workers);
    for thread_number in 0..no_of_workers {
        trace!("Starting thread {}", thread_number);
        worker_handles.push(
            std::thread::Builder::new()
                .name(thread_number.to_string())
                .spawn(move || {
                    do_work(thread_number);
                    0
                })
                .unwrap(),
        );
    }
    trace!("All {} worker threads started.", worker_handles.len());
    worker_handles
}

fn do_work(thread_number: usize) {
    trace!("({})     Thread started working", thread_number);
    trace!("ERROR_IF_PRINTED");
    for idx in 0..NO_OF_LOGLINES_PER_THREAD {
        debug!("({})  writing out line number {}", thread_number, idx);
    }
    std::thread::sleep(std::time::Duration::from_millis(500));
    trace!("MUST_BE_PRINTED");
}

fn wait_for_workers_to_close(worker_handles: Vec<JoinHandle<u8>>) {
    for worker_handle in worker_handles {
        worker_handle
            .join()
            .unwrap_or_else(|e| panic!("Joining worker thread failed: {:?}", e));
    }
    trace!("All worker threads joined.");
}

pub fn test_format(
    w: &mut dyn WriteColor,
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
        Err(e) => panic!(
            "Is this ({}) really a directory? Listing failed with {}",
            pattern, e
        ),
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
                panic!("irregular line in log file {:?}: \"{}\"", pathbuf, buffer);
            }
            buffer.clear();
        }
    }
    assert_eq!(
        line_count,
        NO_OF_THREADS * NO_OF_LOGLINES_PER_THREAD + NO_OF_THREADS + 2
    );
    println!(
        "Found {} log lines from {} threads in {} files",
        line_count, NO_OF_THREADS, no_of_log_files
    );
}
