#![allow(clippy::module_name_repetitions)]
mod builder;
mod config;
mod state;
pub use self::builder::{FileLogWriterBuilder, FlWriteMode};

use self::config::{Config, RotationConfig};
use crate::primary_writer::buffer_with;
use crate::writers::LogWriter;
use crate::{DeferredNow, FileSpec, FlexiLoggerError, FormatFunction};
#[cfg(feature = "async")]
use crossbeam::{
    channel::{self, SendError, Sender},
    queue::ArrayQueue,
};
use log::Record;
use state::State;
use std::io::Write;
use std::path::PathBuf;
#[cfg(feature = "async")]
use std::sync::Arc;
use std::sync::Mutex;
#[cfg(feature = "async")]
use std::thread::JoinHandle;

const WINDOWS_LINE_ENDING: &[u8] = b"\r\n";
const UNIX_LINE_ENDING: &[u8] = b"\n";
#[cfg(feature = "async")]
const ASYNC_FLW_FLUSH: &[u8] = b"F";
#[cfg(feature = "async")]
const ASYNC_FLW_SHUTDOWN: &[u8] = b"S";

const ERR_1: &str = "FileLogWriter: formatting failed ";
const ERR_2: &str = "FileLogWriter: writing failed ";
#[cfg(feature = "async")]
const ERR_3: &str = "FileLogWriter: flushing failed ";

fn write_err(msg: &str, err: &std::io::Error) {
    eprintln!("[flexi_logger] {} with {}", msg, err);
}

/// A configurable [`LogWriter`] implementation that writes to a file or a sequence of files.
///
/// See [writers](crate::writers) for usage guidance.
pub struct FileLogWriter {
    format: FormatFunction,
    // the state needs to be mutable; since `Log.log()` requires an unmutable self,
    // which translates into a non-mutating `LogWriter::write()`,
    // we need internal mutability and thread-safety.
    state_handle: StateHandle,
    max_log_level: log::LevelFilter,
}
enum StateHandle {
    Sync(Mutex<State>),
    #[cfg(feature = "async")]
    Async(AsyncHandle),
}
impl std::fmt::Debug for StateHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            Self::Sync(ref m_state) => f.write_fmt(format_args!("{:?}", &*m_state.lock().unwrap())),
            #[cfg(feature = "async")]
            Self::Async(ref async_handle) => f.write_fmt(format_args!("{:?}", async_handle)),
        }
    }
}

#[cfg(feature = "async")]
#[derive(Debug)]
struct AsyncHandle {
    am_state: Arc<Mutex<State>>,
    sender: Sender<Vec<u8>>,
    mo_thread_handle: Mutex<Option<JoinHandle<()>>>,
    a_pool: Arc<ArrayQueue<Vec<u8>>>,
    msg_capa: usize,
}
#[cfg(feature = "async")]
impl AsyncHandle {
    fn pop_buffer(&self) -> Vec<u8> {
        self.a_pool
            .pop()
            .unwrap_or_else(|| Vec::with_capacity(self.msg_capa))
    }
    fn send(&self, buffer: Vec<u8>) -> Result<(), SendError<Vec<u8>>> {
        self.sender.send(buffer)
    }
}

impl FileLogWriter {
    pub(crate) fn new(
        format: FormatFunction,
        state: State,
        max_log_level: log::LevelFilter,
    ) -> FileLogWriter {
        #[cfg(feature = "async")]
        let state_handle = if let FlWriteMode::BufferAsync(_bufsize, pool_capa, msg_capa) =
            state.config().write_mode
        {
            let am_state = Arc::new(Mutex::new(state));
            let (sender, receiver) = channel::unbounded::<Vec<u8>>();
            let a_pool = Arc::new(ArrayQueue::new(pool_capa));

            let t_state = Arc::clone(&am_state);
            let t_pool = Arc::clone(&a_pool);

            let mo_thread_handle = Mutex::new(Some(
                std::thread::Builder::new()
                    .name("flexi_logger-async_file_log_writer".to_string())
                    .spawn(move || loop {
                        match receiver.recv() {
                            Err(_) => break,
                            Ok(mut message) => {
                                let mut state = t_state.lock().unwrap();
                                match message.as_ref() {
                                    ASYNC_FLW_FLUSH => {
                                        state.flush().unwrap_or_else(|e| write_err(ERR_3, &e));
                                    }
                                    ASYNC_FLW_SHUTDOWN => {
                                        state.shutdown();
                                        break;
                                    }
                                    _ => {
                                        message
                                            .write_all(state.config().line_ending)
                                            .unwrap_or_else(|e| write_err(ERR_2, &e));
                                        state
                                            .write_buffer(&message)
                                            .unwrap_or_else(|e| write_err(ERR_2, &e));
                                    }
                                }
                                if message.capacity() <= msg_capa {
                                    message.clear();
                                    t_pool.push(message).ok();
                                }
                            }
                        }
                    })
                    .unwrap(),
            )); // yes, let's panic if the thread can't be spawned
            StateHandle::Async(AsyncHandle {
                am_state,
                sender,
                mo_thread_handle,
                a_pool,
                msg_capa,
            })
        } else {
            StateHandle::Sync(Mutex::new(state))
        };
        #[cfg(not(feature = "async"))]
        let state_handle = StateHandle::Sync(Mutex::new(state));

        FileLogWriter {
            format,
            state_handle,
            max_log_level,
        }
    }

    /// Instantiates a builder for `FileLogWriter`.
    #[must_use]
    pub fn builder(file_spec: FileSpec) -> FileLogWriterBuilder {
        FileLogWriterBuilder::new(file_spec)
    }

    /// Returns a reference to its configured output format function.
    #[inline]
    pub fn format(&self) -> FormatFunction {
        self.format
    }

    #[doc(hidden)]
    pub fn current_filename(&self) -> PathBuf {
        match &self.state_handle {
            StateHandle::Sync(state) => state.lock().unwrap(),
            #[cfg(feature = "async")]
            StateHandle::Async(handle) => handle.am_state.lock().unwrap(),
        }
        .current_filename()
    }

    /// Replaces parts of the configuration of the file log writer.
    ///
    /// Note that the write mode and the format function cannot be reset and
    /// that the provided `FileLogWriterBuilder` must have the same values for these as the
    /// current `FileLogWriter`.
    ///
    /// # Errors
    ///
    /// `FlexiLoggerError::Reset` if no file log writer is configured,
    ///  or if a reset was tried with a different write mode.
    /// `FlexiLoggerError::Io` if the specified path doesn't work.
    /// `FlexiLoggerError::Poison` if some mutex is poisoned.
    pub fn reset(&self, flwb: &FileLogWriterBuilder) -> Result<(), FlexiLoggerError> {
        let mut state = match &self.state_handle {
            StateHandle::Sync(state) => state.lock(),
            #[cfg(feature = "async")]
            StateHandle::Async(handle) => handle.am_state.lock(),
        }
        .map_err(|_| FlexiLoggerError::Poison)?;
        flwb.assert_write_mode((*state).config().write_mode)?;
        *state = flwb.try_build_state()?;
        Ok(())
    }
}

impl LogWriter for FileLogWriter {
    #[inline]
    fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        match &self.state_handle {
            StateHandle::Sync(state) => {
                buffer_with(|tl_buf| match tl_buf.try_borrow_mut() {
                    Ok(mut buffer) => {
                        (self.format)(&mut *buffer, now, record)
                            .unwrap_or_else(|e| write_err(ERR_1, &e));
                        let mut state_guard = state.lock().unwrap();
                        let state = &mut *state_guard;
                        buffer
                            .write_all(state.config().line_ending)
                            .unwrap_or_else(|e| write_err(ERR_2, &e));
                        state
                            .write_buffer(&*buffer)
                            .unwrap_or_else(|e| write_err(ERR_2, &e));
                        buffer.clear();
                    }
                    Err(_e) => {
                        // We arrive here in the rare cases of recursive logging
                        // (e.g. log calls in Debug or Display implementations)
                        // we print the inner calls, in chronological order, before finally the
                        // outer most message is printed
                        let mut tmp_buf = Vec::<u8>::with_capacity(200);
                        (self.format)(&mut tmp_buf, now, record)
                            .unwrap_or_else(|e| write_err(ERR_1, &e));
                        let mut state_guard = state.lock().unwrap();
                        let state = &mut *state_guard;
                        tmp_buf
                            .write_all(state.config().line_ending)
                            .unwrap_or_else(|e| write_err(ERR_2, &e));
                        state
                            .write_buffer(&tmp_buf)
                            .unwrap_or_else(|e| write_err(ERR_2, &e));
                    }
                });
            }
            #[cfg(feature = "async")]
            StateHandle::Async(handle) => {
                let mut buffer = handle.pop_buffer();
                (self.format)(&mut buffer, now, record).unwrap_or_else(|e| write_err(ERR_1, &e));
                handle.send(buffer).unwrap();
            }
        }

        Ok(())
    }

    #[inline]
    fn flush(&self) -> std::io::Result<()> {
        match &self.state_handle {
            StateHandle::Sync(state) => {
                if let Ok(ref mut state) = state.lock() {
                    state.flush()?;
                }
            }
            #[cfg(feature = "async")]
            StateHandle::Async(handle) => {
                let mut buffer = handle.pop_buffer();
                buffer.extend(ASYNC_FLW_FLUSH);
                handle.send(buffer).ok();
            }
        }
        Ok(())
    }

    #[inline]
    fn max_log_level(&self) -> log::LevelFilter {
        self.max_log_level
    }

    #[doc(hidden)]
    fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) {
        match &self.state_handle {
            StateHandle::Sync(state) => {
                if let Ok(ref mut state) = state.lock() {
                    state.validate_logs(expected);
                }
            }
            #[cfg(feature = "async")]
            StateHandle::Async(handle) => {
                if let Ok(ref mut state) = handle.am_state.lock() {
                    state.validate_logs(expected);
                }
            }
        }
    }

    fn shutdown(&self) {
        match &self.state_handle {
            StateHandle::Sync(state) => {
                // do nothing in case of poison errors
                if let Ok(ref mut state) = state.lock() {
                    state.shutdown();
                }
            }
            #[cfg(feature = "async")]
            StateHandle::Async(handle) => {
                let mut buffer = handle.pop_buffer();
                buffer.extend(ASYNC_FLW_SHUTDOWN);
                handle.send(buffer).unwrap();
                if let Ok(ref mut o_th) = handle.mo_thread_handle.lock() {
                    o_th.take().and_then(|th| th.join().ok());
                }
            }
        }
    }
}

impl std::fmt::Debug for FileLogWriter {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        f.write_fmt(format_args!("{:?}", self.state_handle))
    }
}

impl Drop for FileLogWriter {
    fn drop(&mut self) {
        self.shutdown();
    }
}

#[cfg(test)]
mod test {
    use crate::writers::FlWriteMode;
    use crate::writers::LogWriter;
    use crate::{Cleanup, Criterion, DeferredNow, FileSpec, Naming};
    use chrono::Local;

    use std::ops::Add;
    use std::path::{Path, PathBuf};
    const DIRECTORY: &str = r"log_files/rotate";
    const ONE: &str = "ONE";
    const TWO: &str = "TWO";
    const THREE: &str = "THREE";
    const FOUR: &str = "FOUR";
    const FIVE: &str = "FIVE";
    const SIX: &str = "SIX";
    const SEVEN: &str = "SEVEN";
    const EIGHT: &str = "EIGHT";
    const NINE: &str = "NINE";

    // cargo test --lib -- --nocapture

    #[test]
    fn test_rotate_no_append_numbers() {
        // we use timestamp as discriminant to allow repeated runs
        let ts = Local::now()
            .format("false-numbers-%Y-%m-%d_%H-%M-%S")
            .to_string();
        let naming = Naming::Numbers;

        // ensure we start with -/-/-
        assert!(not_exists("00000", &ts));
        assert!(not_exists("00001", &ts));
        assert!(not_exists("CURRENT", &ts));

        // ensure this produces -/-/ONE
        write_loglines(false, naming, &ts, &[ONE]);
        assert!(not_exists("00000", &ts));
        assert!(not_exists("00001", &ts));
        assert!(contains("CURRENT", &ts, ONE));

        // ensure this produces ONE/-/TWO
        write_loglines(false, naming, &ts, &[TWO]);
        assert!(contains("00000", &ts, ONE));
        assert!(not_exists("00001", &ts));
        assert!(contains("CURRENT", &ts, TWO));

        // ensure this also produces ONE/-/TWO
        remove("CURRENT", &ts);
        assert!(not_exists("CURRENT", &ts));
        write_loglines(false, naming, &ts, &[TWO]);
        assert!(contains("00000", &ts, ONE));
        assert!(not_exists("00001", &ts));
        assert!(contains("CURRENT", &ts, TWO));

        // ensure this produces ONE/TWO/THREE
        write_loglines(false, naming, &ts, &[THREE]);
        assert!(contains("00000", &ts, ONE));
        assert!(contains("00001", &ts, TWO));
        assert!(contains("CURRENT", &ts, THREE));
    }

    #[allow(clippy::cognitive_complexity)]
    #[test]
    fn test_rotate_with_append_numbers() {
        // we use timestamp as discriminant to allow repeated runs
        let ts = Local::now()
            .format("true-numbers-%Y-%m-%d_%H-%M-%S")
            .to_string();
        let naming = Naming::Numbers;

        // ensure we start with -/-/-
        assert!(not_exists("00000", &ts));
        assert!(not_exists("00001", &ts));
        assert!(not_exists("CURRENT", &ts));

        // ensure this produces 12/-/3
        write_loglines(true, naming, &ts, &[ONE, TWO, THREE]);
        assert!(contains("00000", &ts, ONE));
        assert!(contains("00000", &ts, TWO));
        assert!(not_exists("00001", &ts));
        assert!(contains("CURRENT", &ts, THREE));

        // ensure this produces 12/34/56
        write_loglines(true, naming, &ts, &[FOUR, FIVE, SIX]);
        assert!(contains("00000", &ts, ONE));
        assert!(contains("00000", &ts, TWO));
        assert!(contains("00001", &ts, THREE));
        assert!(contains("00001", &ts, FOUR));
        assert!(contains("CURRENT", &ts, FIVE));
        assert!(contains("CURRENT", &ts, SIX));

        // ensure this also produces 12/34/56
        remove("CURRENT", &ts);
        remove("00001", &ts);
        assert!(not_exists("CURRENT", &ts));
        write_loglines(true, naming, &ts, &[THREE, FOUR, FIVE, SIX]);
        assert!(contains("00000", &ts, ONE));
        assert!(contains("00000", &ts, TWO));
        assert!(contains("00001", &ts, THREE));
        assert!(contains("00001", &ts, FOUR));
        assert!(contains("CURRENT", &ts, FIVE));
        assert!(contains("CURRENT", &ts, SIX));

        // ensure this produces 12/34/56/78/9
        write_loglines(true, naming, &ts, &[SEVEN, EIGHT, NINE]);
        assert!(contains("00002", &ts, FIVE));
        assert!(contains("00002", &ts, SIX));
        assert!(contains("00003", &ts, SEVEN));
        assert!(contains("00003", &ts, EIGHT));
        assert!(contains("CURRENT", &ts, NINE));
    }

    #[test]
    fn test_rotate_no_append_timestamps() {
        // we use timestamp as discriminant to allow repeated runs
        let ts = Local::now()
            .format("false-timestamps-%Y-%m-%d_%H-%M-%S")
            .to_string();

        let basename = String::from(DIRECTORY).add("/").add(
            &Path::new(&std::env::args().next().unwrap())
                .file_stem().unwrap(/*cannot fail*/)
                .to_string_lossy().to_string(),
        );
        let naming = Naming::Timestamps;

        // ensure we start with -/-/-
        assert!(list_rotated_files(&basename, &ts).is_empty());
        assert!(not_exists("CURRENT", &ts));

        // ensure this produces -/-/ONE
        write_loglines(false, naming, &ts, &[ONE]);
        assert!(list_rotated_files(&basename, &ts).is_empty());
        assert!(contains("CURRENT", &ts, ONE));

        std::thread::sleep(std::time::Duration::from_secs(2));
        // ensure this produces ONE/-/TWO
        write_loglines(false, naming, &ts, &[TWO]);
        assert_eq!(list_rotated_files(&basename, &ts).len(), 1);
        assert!(contains("CURRENT", &ts, TWO));

        std::thread::sleep(std::time::Duration::from_secs(2));
        // ensure this produces ONE/TWO/THREE
        write_loglines(false, naming, &ts, &[THREE]);
        assert_eq!(list_rotated_files(&basename, &ts).len(), 2);
        assert!(contains("CURRENT", &ts, THREE));
    }

    #[test]
    fn test_rotate_with_append_timestamps() {
        // we use timestamp as discriminant to allow repeated runs
        let ts = Local::now()
            .format("true-timestamps-%Y-%m-%d_%H-%M-%S")
            .to_string();

        let basename = String::from(DIRECTORY).add("/").add(
            &Path::new(&std::env::args().next().unwrap())
                .file_stem().unwrap(/*cannot fail*/)
                .to_string_lossy().to_string(),
        );
        let naming = Naming::Timestamps;

        // ensure we start with -/-/-
        assert!(list_rotated_files(&basename, &ts).is_empty());
        assert!(not_exists("CURRENT", &ts));

        // ensure this produces 12/-/3
        write_loglines(true, naming, &ts, &[ONE, TWO, THREE]);
        assert_eq!(list_rotated_files(&basename, &ts).len(), 1);
        assert!(contains("CURRENT", &ts, THREE));

        // ensure this produces 12/34/56
        write_loglines(true, naming, &ts, &[FOUR, FIVE, SIX]);
        assert!(contains("CURRENT", &ts, FIVE));
        assert!(contains("CURRENT", &ts, SIX));
        assert_eq!(list_rotated_files(&basename, &ts).len(), 2);

        // ensure this produces 12/34/56/78/9
        write_loglines(true, naming, &ts, &[SEVEN, EIGHT, NINE]);
        assert_eq!(list_rotated_files(&basename, &ts).len(), 4);
        assert!(contains("CURRENT", &ts, NINE));
    }

    #[test]
    fn issue_38() {
        const NUMBER_OF_FILES: usize = 5;
        const NUMBER_OF_PSEUDO_PROCESSES: usize = 11;
        const ISSUE_38: &str = "issue_38";
        const LOG_FOLDER: &str = "log_files/issue_38";

        for _ in 0..NUMBER_OF_PSEUDO_PROCESSES {
            let flwb = crate::writers::file_log_writer::FileLogWriter::builder(
                FileSpec::default()
                    .directory(LOG_FOLDER)
                    .discriminant(ISSUE_38),
            )
            .rotate(
                Criterion::Size(500),
                Naming::Timestamps,
                Cleanup::KeepLogFiles(NUMBER_OF_FILES),
            )
            .o_append(false);

            #[cfg(feature = "async")]
            let flwb = flwb.write_mode(FlWriteMode::BufferAsync(5, 5, 400));

            let flw = flwb.try_build().unwrap();

            // write some lines, but not enough to rotate
            for i in 0..4 {
                flw.write(
                    &mut DeferredNow::new(),
                    &log::Record::builder()
                        .args(format_args!("{}", i))
                        .level(log::Level::Error)
                        .target("myApp")
                        .file(Some("server.rs"))
                        .line(Some(144))
                        .module_path(Some("server"))
                        .build(),
                )
                .unwrap();
            }
            flw.flush().ok();
        }

        // give the cleanup thread a short moment of time
        std::thread::sleep(std::time::Duration::from_millis(50));

        let fn_pattern = String::with_capacity(180)
            .add(
                &String::from(LOG_FOLDER).add("/").add(
                    &Path::new(&std::env::args().next().unwrap())
            .file_stem().unwrap(/*cannot fail*/)
            .to_string_lossy().to_string(),
                ),
            )
            .add("_")
            .add(ISSUE_38)
            .add("_r[0-9]*")
            .add(".log");

        assert_eq!(
            glob::glob(&fn_pattern)
                .unwrap()
                .filter_map(Result::ok)
                .count(),
            NUMBER_OF_FILES
        );
    }

    #[test]
    fn test_reset() {
        #[cfg(feature = "async")]
        let flwrite_mode = FlWriteMode::BufferAsync(6, 7, 8);
        #[cfg(not(feature = "async"))]
        let flwrite_mode = FlWriteMode::Buffer(4);
        let flw = super::FileLogWriter::builder(
            FileSpec::default()
                .directory(DIRECTORY)
                .discriminant("test_reset-1"),
        )
        .rotate(
            Criterion::Size(28),
            Naming::Numbers,
            Cleanup::KeepLogFiles(20),
        )
        .append()
        .write_mode(flwrite_mode)
        .try_build()
        .unwrap();

        flw.write(
            &mut DeferredNow::new(),
            &log::Record::builder()
                .args(format_args!("{}", "test_reset-1"))
                .level(log::Level::Error)
                .target("test_reset")
                .file(Some("server.rs"))
                .line(Some(144))
                .module_path(Some("server"))
                .build(),
        )
        .unwrap();

        println!("FileLogWriter {:?}", flw);

        flw.reset(
            &super::FileLogWriter::builder(
                FileSpec::default()
                    .directory(DIRECTORY)
                    .discriminant("test_reset-2"),
            )
            .rotate(
                Criterion::Size(28),
                Naming::Numbers,
                Cleanup::KeepLogFiles(20),
            )
            .write_mode(flwrite_mode),
        )
        .unwrap();
        flw.write(
            &mut DeferredNow::new(),
            &log::Record::builder()
                .args(format_args!("{}", "test_reset-2"))
                .level(log::Level::Error)
                .target("test_reset")
                .file(Some("server.rs"))
                .line(Some(144))
                .module_path(Some("server"))
                .build(),
        )
        .unwrap();
        println!("FileLogWriter {:?}", flw);

        assert!(flw
            .reset(
                &super::FileLogWriter::builder(
                    FileSpec::default()
                        .directory(DIRECTORY)
                        .discriminant("test_reset-3"),
                )
                .rotate(
                    Criterion::Size(28),
                    Naming::Numbers,
                    Cleanup::KeepLogFiles(20),
                )
                .write_mode(FlWriteMode::DontBuffer),
            )
            .is_err());
    }

    fn remove(s: &str, discr: &str) {
        std::fs::remove_file(get_hackyfilepath(s, discr)).unwrap();
    }

    fn not_exists(s: &str, discr: &str) -> bool {
        !get_hackyfilepath(s, discr).exists()
    }

    fn contains(s: &str, discr: &str, text: &str) -> bool {
        match std::fs::read_to_string(get_hackyfilepath(s, discr)) {
            Err(_) => false,
            Ok(s) => s.contains(text),
        }
    }

    fn get_hackyfilepath(infix: &str, discr: &str) -> Box<Path> {
        let arg0 = std::env::args().next().unwrap();
        let mut s_filename = Path::new(&arg0)
            .file_stem()
            .unwrap()
            .to_string_lossy()
            .to_string();
        s_filename += "_";
        s_filename += discr;
        s_filename += "_r";
        s_filename += infix;
        s_filename += ".log";
        let mut path_buf = PathBuf::from(DIRECTORY);
        path_buf.push(s_filename);
        path_buf.into_boxed_path()
    }

    fn write_loglines(append: bool, naming: Naming, discr: &str, texts: &[&'static str]) {
        let flw = get_file_log_writer(append, naming, discr);
        for text in texts {
            flw.write(
                &mut DeferredNow::new(),
                &log::Record::builder()
                    .args(format_args!("{}", text))
                    .level(log::Level::Error)
                    .target("myApp")
                    .file(Some("server.rs"))
                    .line(Some(144))
                    .module_path(Some("server"))
                    .build(),
            )
            .unwrap();
        }
    }

    fn get_file_log_writer(
        append: bool,
        naming: Naming,
        discr: &str,
    ) -> crate::writers::FileLogWriter {
        super::FileLogWriter::builder(FileSpec::default().directory(DIRECTORY).discriminant(discr))
            .rotate(
                Criterion::Size(if append { 28 } else { 10 }),
                naming,
                Cleanup::Never,
            )
            .o_append(append)
            .try_build()
            .unwrap()
    }

    fn list_rotated_files(basename: &str, discr: &str) -> Vec<String> {
        let fn_pattern = String::with_capacity(180)
            .add(basename)
            .add("_")
            .add(discr)
            .add("_r2[0-9]*") // Year 3000 problem!!!
            .add(".log");

        glob::glob(&fn_pattern)
            .unwrap()
            .map(|r| r.unwrap().into_os_string().to_string_lossy().to_string())
            .collect()
    }
}
