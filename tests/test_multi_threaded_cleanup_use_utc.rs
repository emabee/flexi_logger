mod test_utils;

#[cfg(feature = "compress")]
mod d {
    use cond_sync::{CondSync, Other};
    use flexi_logger::{
        Cleanup, Criterion, DeferredNow, Duplicate, FileSpec, LogSpecification, Logger, Naming,
        WriteMode, TS_DASHES_BLANK_COLONS_DOT_BLANK,
    };
    use glob::glob;
    use log::*;
    use std::{ops::Add, thread::JoinHandle};

    const NO_OF_THREADS: usize = 5;
    const NO_OF_LOGLINES_PER_THREAD: usize = 20_000;
    const ROTATE_OVER_SIZE: u64 = 600_000;
    const NO_OF_LOG_FILES: usize = 2;
    const NO_OF_GZ_FILES: usize = 5;

    // we use a special log line format that starts with a special string
    // so that it is easier to verify that all log lines are written correctly
    #[test]
    fn multi_threaded() {
        let directory = super::test_utils::dir();
        {
            let _stopwatch = super::test_utils::Stopwatch::default();
            let logger = Logger::try_with_str("debug")
                .unwrap()
                .log_to_file(FileSpec::default().directory(&directory))
                .write_mode(WriteMode::BufferAndFlushWith(
                    10 * 1024,
                    std::time::Duration::from_millis(600),
                ))
                .format(test_format)
                .duplicate_to_stderr(Duplicate::Info)
                .rotate(
                    Criterion::Size(ROTATE_OVER_SIZE),
                    Naming::Timestamps,
                    Cleanup::KeepLogAndCompressedFiles(NO_OF_LOG_FILES, NO_OF_GZ_FILES),
                )
                .cleanup_in_background_thread(false)
                .use_utc()
                .start()
                .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));

            info!(
                "create a huge number of log lines with a considerable number of threads, \
                verify the log"
            );

            let cond_sync = CondSync::new(0_usize);
            let worker_handles = start_worker_threads(NO_OF_THREADS, &cond_sync);
            cond_sync
                .wait_until(|value| *value == NO_OF_THREADS)
                .unwrap();

            logger.set_new_spec(LogSpecification::parse("trace").unwrap());

            join_all_workers(worker_handles);
        } // drop stopwatch and logger

        verify_logs(&directory.display().to_string());
    }

    // Starts given number of worker threads and lets each execute `do_work`
    fn start_worker_threads(
        no_of_workers: usize,
        cond_sync: &CondSync<usize>,
    ) -> Vec<JoinHandle<u8>> {
        let mut worker_handles: Vec<JoinHandle<u8>> = Vec::with_capacity(no_of_workers);
        trace!("Starting {no_of_workers} worker threads");
        for thread_number in 0..no_of_workers {
            trace!("Starting thread {thread_number}");
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
        trace!("({thread_number})     Thread started working");
        trace!("ERROR_IF_PRINTED");

        cond_sync
            .modify_and_notify(|value| *value += 1, Other::One)
            .unwrap();

        for idx in 0..NO_OF_LOGLINES_PER_THREAD {
            debug!("({thread_number})  writing out line number {idx}");
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
        // Since the cleanup deleted log files, we can only check that the correct number of
        // log files and compressed files exist

        let basename = String::from(directory).add("/").add(
            &std::path::Path::new(&std::env::args().next().unwrap())
            .file_stem().unwrap(/*cannot fail*/)
            .to_string_lossy(),
        );

        let fn_pattern = String::with_capacity(180)
            .add(&basename)
            .add("_r[0-9][0-9]*.");

        let no_of_log_files = glob(&fn_pattern.clone().add("log"))
            .unwrap()
            .map(Result::unwrap)
            .count();

        let no_of_gz_files = glob(&fn_pattern.add("gz"))
            .unwrap()
            .map(Result::unwrap)
            .count();

        assert_eq!(no_of_log_files, NO_OF_LOG_FILES);
        assert_eq!(no_of_gz_files, NO_OF_GZ_FILES);

        info!("Found correct number of log and compressed files");
    }
}
