mod test_utils;

use flexi_logger::{Logger, WriteMode};
use log::*;
use std::thread::{self, JoinHandle};

const NO_OF_THREADS: usize = 5;
const NO_OF_LOGLINES_PER_THREAD: usize = 5_000;

#[test]
fn multi_threaded() {
    test_utils::wait_for_start_of_second();
    let _logger = Logger::try_with_str("debug")
        .unwrap()
        .log_to_stderr()
        .write_mode(WriteMode::BufferAndFlushWith(
            1024,
            std::time::Duration::from_millis(600),
        ))
        .start()
        .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));
    info!("create a huge number of log lines with a considerable number of threads");
    for i in 0..50 {
        std::thread::sleep(std::time::Duration::from_millis(100));
        info!("********** check delay of this log line ({}) **********", i);
    }
    let _stopwatch = test_utils::Stopwatch::default();

    let worker_handles = start_worker_threads(NO_OF_THREADS);

    wait_for_workers_to_close(worker_handles);
}

// Starts given number of worker threads and lets each execute `do_work`
fn start_worker_threads(no_of_workers: usize) -> Vec<JoinHandle<u8>> {
    let mut worker_handles: Vec<JoinHandle<u8>> = Vec::with_capacity(no_of_workers);
    trace!("Starting {} worker threads", no_of_workers);
    for thread_number in 0..no_of_workers {
        trace!("Starting thread {}", thread_number);
        worker_handles.push(
            thread::Builder::new()
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
