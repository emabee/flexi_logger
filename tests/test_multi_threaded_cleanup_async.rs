mod test_utils;

#[cfg(feature = "compress")]
mod d {
    use flate2::bufread::GzDecoder;
    use flexi_logger::{
        Cleanup, Criterion, DeferredNow, Duplicate, FileSpec, LogSpecification, Logger, Naming,
        Record, WriteMode,
    };
    use glob::glob;
    use lazy_static::lazy_static;
    use log::*;
    use std::collections::BTreeMap;
    use std::fs::File;
    use std::io::{BufRead, BufReader, Write};
    use std::ops::Add;
    use std::path::{Path, PathBuf};
    use std::thread::{self, JoinHandle};
    use time::{
        format_description::{self, FormatItem},
        OffsetDateTime,
    };

    const NO_OF_THREADS: usize = 5;
    const NO_OF_LOGLINES_PER_THREAD: usize = 20_000;
    const ROTATE_OVER_SIZE: u64 = 600_000;
    const NO_OF_LOG_FILES: usize = 2;
    const NO_OF_GZ_FILES: usize = 5;

    #[test]
    fn multi_threaded() {
        // we use a special log line format that starts with a special string so that it is easier to
        // verify that all log lines are written correctly

        let start = super::test_utils::now_local_or_utc();
        let directory = super::test_utils::dir();
        let logger = Logger::try_with_str("debug")
            .unwrap()
            .log_to_file(FileSpec::default().directory(&directory));

        #[cfg(not(feature = "async"))]
        let logger = logger.write_mode(WriteMode::BufferAndFlush);

        #[cfg(feature = "async")]
        let logger = logger.write_mode(WriteMode::Async);

        let mut logger = logger
            .format(test_format)
            .duplicate_to_stderr(Duplicate::Info)
            .rotate(
                Criterion::Size(ROTATE_OVER_SIZE),
                Naming::Timestamps,
                Cleanup::KeepLogAndCompressedFiles(NO_OF_LOG_FILES, NO_OF_GZ_FILES),
            )
            .start()
            .unwrap_or_else(|e| panic!("Logger initialization failed with {}", e));
        info!(
            "create a huge number of log lines with a considerable number of threads, \
             verify the log"
        );

        let worker_handles = start_worker_threads(NO_OF_THREADS);
        let new_spec = LogSpecification::parse("trace").unwrap();
        thread::sleep(std::time::Duration::from_millis(1000));
        logger.set_new_spec(new_spec);

        wait_for_workers_to_close(worker_handles);

        let delta1 = (super::test_utils::now_local_or_utc() - start).whole_milliseconds();

        std::mem::drop(logger);
        let delta2 = (super::test_utils::now_local_or_utc() - start).whole_milliseconds();
        println!(
            "Task executed with {} threads in {} ms, writing logs extended to {} ms.",
            NO_OF_THREADS, delta1, delta2
        );
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
            if idx % 1_000 == 0 {
                std::thread::yield_now();
            }
            debug!("({})  writing out line number {}", thread_number, idx);
        }
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

    const TS_S: &str = "[year]-[month]-[day] [hour]:[minute]:[second].[subsecond digits:6] \
                    [offset_hour sign:mandatory]:[offset_minute]";
    lazy_static! {
        static ref TS: Vec<FormatItem<'static>> = format_description::parse(TS_S).unwrap(/*ok*/);
    }

    pub fn test_format(
        w: &mut dyn std::io::Write,
        now: &mut DeferredNow,
        record: &Record,
    ) -> std::io::Result<()> {
        write!(
            w,
            "XXXXX [{}] T[{:?}] {} [{}:{}] {}",
            now.now().format(&TS).unwrap(),
            thread::current().name().unwrap_or("<unnamed>"),
            record.level(),
            record.file().unwrap_or("<unnamed>"),
            record.line().unwrap_or(0),
            &record.args()
        )
    }

    fn verify_logs(directory: &str) {
        let basename = String::from(directory).add("/").add(
            &std::path::Path::new(&std::env::args().next().unwrap())
            .file_stem().unwrap(/*cannot fail*/)
            .to_string_lossy().to_string(),
        );

        let mut counters = Counters {
            total: (None, BTreeMap::new()),
            threads: [
                (None, BTreeMap::new()),
                (None, BTreeMap::new()),
                (None, BTreeMap::new()),
                (None, BTreeMap::new()),
                (None, BTreeMap::new()),
            ],
        };

        let fn_pattern = String::with_capacity(180)
            .add(&basename)
            // .add("_r[0-9][0-9]*.");
            .add("_r*.");

        let log_pattern = fn_pattern.clone().add("log");
        let no_of_log_files = glob(&log_pattern)
            .unwrap()
            .map(Result::unwrap)
            .inspect(|p| inspect_file(p, &mut counters))
            .count();

        let gz_pattern = fn_pattern.add("gz");
        let no_of_gz_files = glob(&gz_pattern)
            .unwrap()
            .map(Result::unwrap)
            .inspect(|p| inspect_file(p, &mut counters))
            .count();

        assert_eq!(no_of_log_files, NO_OF_LOG_FILES + 1);
        assert_eq!(no_of_gz_files, NO_OF_GZ_FILES);

        // info!("Found correct number of log and compressed files");
        write_csv(directory, "total.csv", &counters.total.1);
        write_csv(directory, "thread_0.csv", &counters.threads[0].1);
        write_csv(directory, "thread_1.csv", &counters.threads[1].1);
        write_csv(directory, "thread_2.csv", &counters.threads[2].1);
        write_csv(directory, "thread_3.csv", &counters.threads[3].1);
        write_csv(directory, "thread_4.csv", &counters.threads[4].1);
    }

    fn inspect_file(p: &Path, counters: &mut Counters) {
        let buf_reader: Box<dyn BufRead> = if p.extension().unwrap() == "gz" {
            Box::new(BufReader::new(GzDecoder::new(BufReader::new(
                File::open(p).unwrap(),
            ))))
        } else {
            Box::new(BufReader::new(File::open(p).unwrap()))
        };

        for line in buf_reader.lines() {
            let line = line.unwrap();
            //9 fraction digits, should be 6
            if let Ok(ts) = OffsetDateTime::parse(&line[7..40], &TS) {
                let n = match &line[45..46].parse::<usize>() {
                    Ok(n) => *n,
                    Err(_) => continue,
                };

                if let Some(bts) = counters.total.0 {
                    *counters
                        .total
                        .1
                        .entry((ts - bts).whole_microseconds())
                        .or_insert(1) += 1;
                }
                counters.total.0 = Some(ts);

                if let Some(bts) = counters.threads[n].0 {
                    *counters.threads[n]
                        .1
                        .entry((ts - bts).whole_microseconds())
                        .or_insert(1) += 1;
                }
                counters.threads[n].0 = Some(ts);
            }
        }
    }

    fn write_csv(directory: &str, name: &str, data: &BTreeMap<i128, usize>) {
        let mut path = PathBuf::from(directory);
        path.push(name);
        let mut file = std::io::BufWriter::new(
            std::fs::OpenOptions::new()
                .write(true)
                .create(true)
                .truncate(true)
                .open(path)
                .unwrap(),
        );
        for (interval, count) in data {
            writeln!(&mut file, "{:?};{};", interval, count).unwrap();
        }
    }

    struct Counters {
        total: (Option<OffsetDateTime>, BTreeMap<i128, usize>),
        threads: [(Option<OffsetDateTime>, BTreeMap<i128, usize>); 5],
    }
}
