mod test_utils;

#[cfg(feature = "compress")]
mod d {
    use flexi_logger::{
        Cleanup, Criterion, DeferredNow, Duplicate, FileSpec, LogSpecification, LogfileSelector,
        Logger, Naming, WriteMode, TS_DASHES_BLANK_COLONS_DOT_BLANK,
    };
    use glob::glob;
    use log::*;
    use std::{
        ops::Add,
        thread::{self, JoinHandle},
    };

    const NO_OF_THREADS: usize = 5;
    const NO_OF_LOGLINES_PER_THREAD: usize = 20_000;
    const ROTATE_OVER_SIZE: u64 = 600_000;
    const NO_OF_LOG_FILES: usize = 2;
    const NO_OF_GZ_FILES: usize = 5;

    // we use a special log line format that starts with a special string
    // so that it is easier to verify that all log lines are written correctly
    #[test]
    fn multi_threaded() {
        super::test_utils::wait_for_start_of_second();
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

            let worker_handles = start_worker_threads(NO_OF_THREADS);
            let new_spec = LogSpecification::parse("trace").unwrap();
            thread::sleep(std::time::Duration::from_millis(500));
            logger.set_new_spec(new_spec);

            join_all_workers(worker_handles);

            let log_files = logger
                .existing_log_files(
                    &LogfileSelector::default()
                        .with_compressed_files()
                        .with_r_current(),
                )
                .unwrap();
            assert_eq!(log_files.len(), NO_OF_LOG_FILES + NO_OF_GZ_FILES + 1);
            for f in log_files {
                debug!("Existing log file: {f:?}");
            }
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
        std::thread::sleep(std::time::Duration::from_millis(500));
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
            thread::current().name().unwrap_or("<unnamed>"),
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

        let log_pattern = fn_pattern.clone().add("log");
        println!("log_pattern = {log_pattern}");
        let no_of_log_files = glob(&log_pattern)
            .unwrap()
            .map(Result::unwrap)
            .inspect(|p| println!("found: {p:?}"))
            .count();

        let gz_pattern = fn_pattern.add("gz");
        let no_of_gz_files = glob(&gz_pattern)
            .unwrap()
            .map(Result::unwrap)
            .inspect(|p| println!("found: {p:?}"))
            .count();

        assert_eq!(no_of_log_files, NO_OF_LOG_FILES);
        assert_eq!(no_of_gz_files, NO_OF_GZ_FILES);

        info!("Found correct number of log and compressed files");
    }
}
