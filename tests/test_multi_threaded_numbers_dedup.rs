mod test_utils;

use flexi_logger::{
    filter::{LogLineFilter, LogLineWriter},
    Cleanup, Criterion, DeferredNow, Duplicate, FileSpec, Logger, Naming, WriteMode,
    TS_DASHES_BLANK_COLONS_DOT_BLANK,
};

use glob::glob;
use log::*;
use std::cmp::Ordering;
use std::fs::File;
use std::io::{BufRead, BufReader};
use std::num::NonZeroUsize;
use std::ops::Add;
use std::sync::Mutex;
use std::thread::JoinHandle;

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
        let _logger;
        let _stopwatch = test_utils::Stopwatch::default();
        _logger = Logger::try_with_str("debug")
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
            .filter(Box::new(DedupWriter::with_leeway(
                std::num::NonZeroUsize::new(22).unwrap(),
            )))
            .start()
            .unwrap_or_else(|e| panic!("Logger initialization failed with {e}"));
        info!("create a huge number of log lines, but deduplicate them");

        wait_for_workers_to_close(start_worker_threads(NO_OF_THREADS));
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
    for _idx in 0..NO_OF_LOGLINES_PER_THREAD {
        debug!("bliblablub");
    }
    std::thread::sleep(std::time::Duration::from_millis(500));
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

/// A helper to skip duplicated consecutive log lines.
pub struct DedupWriter {
    deduper: Mutex<Deduper>,
}
impl DedupWriter {
    /// Constructs a new [`Deduper`] that will skip duplicated entries after
    /// some record has been received for the consecutive times specified by
    /// `leeway`.
    pub fn with_leeway(leeway: NonZeroUsize) -> Self {
        Self {
            deduper: Mutex::new(Deduper::with_leeway(leeway)),
        }
    }
}
impl LogLineFilter for DedupWriter {
    fn write(
        &self,
        now: &mut DeferredNow,
        record: &Record,
        log_line_writer: &dyn LogLineWriter,
    ) -> std::io::Result<()> {
        let mut deduper = self.deduper.lock().unwrap();
        let dedup_action = deduper.dedup(record);
        match dedup_action {
            DedupAction::Allow => {
                // Just log
                log_line_writer.write(now, record)
            }
            DedupAction::AllowLastOfLeeway(_) => {
                // Log duplicate
                log_line_writer.write(now, record)?;
                // Log warning
                log_line_writer.write(
                    now,
                    &log::Record::builder()
                        .level(log::Level::Warn)
                        .file_static(Some(file!()))
                        .line(Some(line!()))
                        .module_path_static(Some("flexi_logger"))
                        .target("flexi_logger")
                        .args(format_args!(
                            "last record has been repeated consecutive times, \
                             following duplicates will be skipped...",
                        ))
                        .build(),
                )
            }
            DedupAction::AllowAfterSkipped(skipped) => {
                // Log summary of skipped
                log_line_writer.write(
                    now,
                    &log::Record::builder()
                        .level(log::Level::Info)
                        .file_static(Some(file!()))
                        .line(Some(line!()))
                        .module_path_static(Some("flexi_logger"))
                        .target("flexi_logger")
                        .args(format_args!("last record was skipped {skipped} times"))
                        .build(),
                )?;
                // Log new record
                log_line_writer.write(now, record)
            }
            DedupAction::Skip => Ok(()),
        }
    }
}

// A helper to track duplicated consecutive logs and skip them until a
// different event is received.
struct Deduper {
    leeway: NonZeroUsize,
    last_record: LastRecord,
    duplicates: usize,
}

/// Action to be performed for some record.
#[derive(Debug, PartialEq, Eq)]
enum DedupAction {
    /// The record should be allowed and logged normally.
    Allow,
    /// The record is the last consecutive duplicate to be allowed.
    ///
    /// Any following duplicates will be skipped until a different event is
    /// received (or the duplicates count overflows).
    AllowLastOfLeeway(usize),
    /// The record should be allowed, the last `N` records were skipped as
    /// consecutive duplicates.
    AllowAfterSkipped(usize),
    /// The record should be skipped because no more consecutive duplicates
    /// are allowed.
    Skip,
}

impl Deduper {
    // Constructs a new [`Deduper`] that will skip duplicated entries after
    // some record has been received for the consecutive times specified by
    // `leeway`.
    pub fn with_leeway(leeway: NonZeroUsize) -> Self {
        Self {
            leeway,
            last_record: LastRecord {
                file: None,
                line: None,
                msg: String::new(),
            },
            duplicates: 0,
        }
    }

    /// Returns wether a record should be skipped or allowed.
    ///
    /// See [`DedupAction`].
    fn dedup(&mut self, record: &Record) -> DedupAction {
        let new_line = record.line();
        let new_file = record.file();
        let new_msg = record.args().to_string();
        if new_line == self.last_record.line
            && new_file == self.last_record.file.as_deref()
            && new_msg == self.last_record.msg
        {
            // Update dups count
            if let Some(updated_dups) = self.duplicates.checked_add(1) {
                self.duplicates = updated_dups;
            } else {
                let skipped = self.duplicates - self.leeway();
                self.duplicates = 0;
                return DedupAction::AllowAfterSkipped(skipped);
            }

            match self.duplicates.cmp(&self.leeway()) {
                Ordering::Less => DedupAction::Allow,
                Ordering::Equal => DedupAction::AllowLastOfLeeway(self.leeway()),
                Ordering::Greater => DedupAction::Skip,
            }
        } else {
            // Update last record
            self.last_record.file = new_file.map(ToOwned::to_owned);
            self.last_record.line = new_line;
            self.last_record.msg = new_msg;

            let dups = self.duplicates;
            self.duplicates = 0;

            match dups {
                n if n > self.leeway() => DedupAction::AllowAfterSkipped(n - self.leeway()),
                _ => DedupAction::Allow,
            }
        }
    }

    fn leeway(&self) -> usize {
        self.leeway.get()
    }
}

struct LastRecord {
    file: Option<String>,
    line: Option<u32>,
    msg: String,
}

fn verify_logs(directory: &str) {
    // read all files
    let pattern = String::from(directory).add("/*");
    let globresults = match glob(&pattern) {
        Err(e) => panic!("Is this ({pattern}) really a directory? Listing failed with {e}"),
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
    assert_eq!(line_count, 27);
    println!(
        "Found {line_count} log lines from {NO_OF_THREADS} threads in {no_of_log_files} files"
    );
}
