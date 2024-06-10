mod list_and_cleanup;
mod numbers;
mod timestamps;

use super::config::{FileLogWriterConfig, RotationConfig};
use crate::{
    util::{eprint_err, ErrorCode},
    Age, Cleanup, Criterion, FlexiLoggerError, LogfileSelector, Naming,
};
use chrono::{DateTime, Datelike, Local, Timelike};
#[cfg(feature = "async")]
use std::thread::JoinHandle;
use std::{
    fs::{remove_file, File, OpenOptions},
    io::{BufRead, BufReader, BufWriter, Write},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use timestamps::{
    collision_free_infix_for_rotated_file, infix_from_timestamp, latest_timestamp_file,
    rcurrents_creation_timestamp,
};

#[cfg(feature = "async")]
const ASYNC_FLUSHER: &str = "flexi_logger-fs-async_flusher";

#[cfg(feature = "async")]
use {
    crate::util::{ASYNC_FLUSH, ASYNC_SHUTDOWN},
    crossbeam_channel::Sender as CrossbeamSender,
    crossbeam_queue::ArrayQueue,
};

#[cfg(feature = "async")]
const ASYNC_WRITER: &str = "flexi_logger-fs-async_writer";

const CURRENT_INFIX: &str = "_rCURRENT";

#[derive(Debug)]
enum NamingState {
    // Contains the timestamp of the current output file (read from its name),
    // plus the optional current infix (_with_ underscore),
    // and the format of the timestamp infix (_with_ underscore)
    Timestamps(DateTime<Local>, Option<String>, InfixFormat),

    // contains the index to which we will rotate
    NumbersRCurrent(u32),

    // contains the index of the current output file
    NumbersDirect(u32),
}
impl NamingState {
    pub(crate) fn writes_direct(&self) -> bool {
        matches!(
            self,
            NamingState::NumbersDirect(_) | NamingState::Timestamps(_, None, _)
        )
    }
}

#[derive(Clone, Debug)]
pub(super) enum InfixFormat {
    Std,
    Custom(String),
}
impl InfixFormat {
    const STD_INFIX_FORMAT: &'static str = "_r%Y-%m-%d_%H-%M-%S";
    pub(super) fn custom(f: &str) -> Self {
        let mut fmt = "_".to_string();
        fmt.push_str(f);
        Self::Custom(fmt)
    }
    fn format(&self) -> &str {
        match self {
            Self::Std => Self::STD_INFIX_FORMAT,
            Self::Custom(fmt) => fmt,
        }
    }
}

#[derive(Debug)]
enum RollState {
    Size {
        max_size: u64,
        current_size: u64,
    },
    Age {
        age: Age,
        created_at: DateTime<Local>,
    },
    AgeOrSize {
        age: Age,
        created_at: DateTime<Local>,
        max_size: u64,
        current_size: u64,
    },
}
impl RollState {
    fn new(criterion: Criterion, append: bool, path: &Path) -> Result<RollState, std::io::Error> {
        let current_size = if append {
            std::fs::metadata(path)?.len()
        } else {
            0
        };
        let created_at = get_creation_timestamp(path);

        Ok(match criterion {
            Criterion::Age(age) => RollState::Age { age, created_at },
            Criterion::Size(max_size) => RollState::Size {
                max_size,
                current_size,
            },
            Criterion::AgeOrSize(age, max_size) => RollState::AgeOrSize {
                age,
                created_at,
                max_size,
                current_size,
            },
        })
    }

    fn rotation_necessary(&self) -> bool {
        match &self {
            RollState::Size {
                max_size,
                current_size,
            } => Self::size_rotation_necessary(*max_size, *current_size),
            RollState::Age { age, created_at } => Self::age_rotation_necessary(*age, created_at),
            RollState::AgeOrSize {
                age,
                created_at,
                max_size,
                current_size,
            } => {
                Self::size_rotation_necessary(*max_size, *current_size)
                    || Self::age_rotation_necessary(*age, created_at)
            }
        }
    }

    fn size_rotation_necessary(max_size: u64, current_size: u64) -> bool {
        current_size > max_size
    }

    fn age_rotation_necessary(age: Age, created_at: &DateTime<Local>) -> bool {
        let now = Local::now();
        match age {
            Age::Day => {
                created_at.year() != now.year()
                    || created_at.month() != now.month()
                    || created_at.day() != now.day()
            }
            Age::Hour => {
                created_at.year() != now.year()
                    || created_at.month() != now.month()
                    || created_at.day() != now.day()
                    || created_at.hour() != now.hour()
            }
            Age::Minute => {
                created_at.year() != now.year()
                    || created_at.month() != now.month()
                    || created_at.day() != now.day()
                    || created_at.hour() != now.hour()
                    || created_at.minute() != now.minute()
            }
            Age::Second => {
                created_at.year() != now.year()
                    || created_at.month() != now.month()
                    || created_at.day() != now.day()
                    || created_at.hour() != now.hour()
                    || created_at.minute() != now.minute()
                    || created_at.second() != now.second()
            }
        }
    }

    fn reset_size_and_date(&mut self, path: &Path) {
        match self {
            RollState::Size {
                max_size: _,
                current_size,
            } => {
                *current_size = 0;
            }
            RollState::Age { age: _, created_at } => {
                *created_at = get_creation_timestamp(path);
            }
            RollState::AgeOrSize {
                age: _,
                created_at,
                max_size: _,
                current_size,
            } => {
                *created_at = get_creation_timestamp(path);
                *current_size = 0;
            }
        }
    }

    fn increase_size(&mut self, add: u64) {
        if let RollState::Size {
            max_size: _,
            ref mut current_size,
        }
        | RollState::AgeOrSize {
            age: _,
            created_at: _,
            max_size: _,
            ref mut current_size,
        } = *self
        {
            *current_size += add;
        }
    }
}

#[derive(Debug)]
struct RotationState {
    naming_state: NamingState,
    roll_state: RollState,
    cleanup: Cleanup,
    o_cleanup_thread_handle: Option<list_and_cleanup::CleanupThreadHandle>,
}
impl RotationState {
    fn shutdown(&mut self) {
        // this sets o_cleanup_thread_handle in self.state.o_rotation_state to None:
        let o_cleanup_thread_handle = self.o_cleanup_thread_handle.take();

        if let Some(cleanup_thread_handle) = o_cleanup_thread_handle {
            cleanup_thread_handle.shutdown();
        }
    }
}

enum Inner {
    Initial(Option<RotationConfig>, bool),
    Active(Option<RotationState>, Box<dyn Write + Send>, PathBuf),
}
impl std::fmt::Debug for Inner {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> Result<(), std::fmt::Error> {
        match self {
            Self::Initial(o_rot, b) => f.write_fmt(format_args!("Initial({o_rot:?}, {b}) ")),
            Self::Active(o_rot, _, _) => {
                f.write_fmt(format_args!("Active({o_rot:?}, <some-writer>) "))
            }
        }
    }
}

// The mutable state of a FileLogWriter.
#[derive(Debug)]
pub(super) struct State {
    config: FileLogWriterConfig,
    inner: Inner,
}
impl State {
    pub(super) fn new(
        config: FileLogWriterConfig,
        o_rotation_config: Option<RotationConfig>,
        cleanup_in_background_thread: bool,
    ) -> Self {
        Self {
            config,
            inner: Inner::Initial(o_rotation_config, cleanup_in_background_thread),
        }
    }

    fn initialize(&mut self) -> Result<(), std::io::Error> {
        if let Inner::Initial(o_rotation_config, cleanup_in_background_thread) = &self.inner {
            self.inner = match o_rotation_config {
                None => {
                    // no rotation
                    let (write, path) = open_log_file(&self.config, None)?;
                    Inner::Active(None, write, path)
                }
                Some(rotate_config) => {
                    self.initialize_with_rotation(rotate_config, *cleanup_in_background_thread)?
                }
            };
        }
        Ok(())
    }

    #[allow(clippy::too_many_lines)]
    fn initialize_with_rotation(
        &self,
        rotate_config: &RotationConfig,
        cleanup_in_background_thread: bool,
    ) -> Result<Inner, std::io::Error> {
        let (naming_state, infix) = match rotate_config.naming {
            Naming::TimestampsDirect => {
                let ts =
                    latest_timestamp_file(&self.config, !self.config.append, &InfixFormat::Std);
                (
                    NamingState::Timestamps(ts, None, InfixFormat::Std),
                    infix_from_timestamp(&ts, self.config.use_utc, &InfixFormat::Std),
                )
            }
            Naming::Timestamps => (
                NamingState::Timestamps(
                    rcurrents_creation_timestamp(
                        &self.config,
                        CURRENT_INFIX,
                        !self.config.append,
                        None,
                        &InfixFormat::Std,
                    )?,
                    Some(CURRENT_INFIX.to_string()),
                    InfixFormat::Std,
                ),
                CURRENT_INFIX.to_string(),
            ),
            Naming::TimestampsCustomFormat {
                current_infix: o_current_token,
                format: ts_fmt,
            } => {
                if let Some(current_token) = o_current_token {
                    let current_infix = prepend_underscore(current_token);
                    let naming_state = NamingState::Timestamps(
                        rcurrents_creation_timestamp(
                            &self.config,
                            &current_infix,
                            !self.config.append,
                            None,
                            &InfixFormat::custom(ts_fmt),
                        )?,
                        Some(current_infix.clone()),
                        InfixFormat::custom(ts_fmt),
                    );
                    (naming_state, current_infix)
                } else {
                    let fmt = InfixFormat::custom(ts_fmt);
                    let ts = latest_timestamp_file(&self.config, !self.config.append, &fmt);
                    let naming_state = NamingState::Timestamps(ts, None, fmt.clone());
                    let infix = infix_from_timestamp(&ts, self.config.use_utc, &fmt);
                    (naming_state, infix)
                }
            }
            Naming::Numbers => (
                NamingState::NumbersRCurrent(numbers::index_for_rcurrent(
                    &self.config,
                    None,
                    !self.config.append,
                )?),
                CURRENT_INFIX.to_string(),
            ),
            Naming::NumbersDirect => {
                let idx = match numbers::get_highest_index(&self.config.file_spec) {
                    None => 0,
                    Some(idx) => {
                        if self.config.append {
                            idx
                        } else {
                            idx + 1
                        }
                    }
                };
                (NamingState::NumbersDirect(idx), numbers::number_infix(idx))
            }
        };
        let (write, path) = open_log_file(&self.config, Some(&infix))?;
        let roll_state = RollState::new(rotate_config.criterion, self.config.append, &path)?;
        let o_cleanup_thread_handle = if rotate_config.cleanup.do_cleanup() {
            list_and_cleanup::remove_or_compress_too_old_logfiles(
                &None,
                &rotate_config.cleanup,
                &self.config.file_spec,
                rotate_config.naming.writes_direct(),
            )?;
            if cleanup_in_background_thread {
                Some(list_and_cleanup::start_cleanup_thread(
                    rotate_config.cleanup,
                    self.config.file_spec.clone(),
                    rotate_config.naming.writes_direct(),
                )?)
            } else {
                None
            }
        } else {
            None
        };
        Ok(Inner::Active(
            Some(RotationState {
                naming_state,
                roll_state,
                cleanup: rotate_config.cleanup,
                o_cleanup_thread_handle,
            }),
            write,
            path,
        ))
    }

    pub fn config(&self) -> &FileLogWriterConfig {
        &self.config
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        if let Inner::Active(_, ref mut file, _) = self.inner {
            file.flush()
        } else {
            Ok(())
        }
    }

    #[inline]
    pub(super) fn mount_next_linewriter_if_necessary(
        &mut self,
        force: bool,
    ) -> Result<(), FlexiLoggerError> {
        if let Inner::Active(
            Some(ref mut rotation_state),
            ref mut current_write,
            ref mut current_path,
        ) = self.inner
        {
            if force || rotation_state.roll_state.rotation_necessary() {
                let infix = match rotation_state.naming_state {
                    NamingState::Timestamps(ref mut ts, ref o_current_infix, ref fmt) => {
                        match o_current_infix {
                            Some(current_infix) => {
                                *ts = rcurrents_creation_timestamp(
                                    &self.config,
                                    current_infix,
                                    true,
                                    Some(ts),
                                    fmt,
                                )?;
                                current_infix.clone()
                            }
                            None => {
                                *ts = Local::now();
                                collision_free_infix_for_rotated_file(
                                    &self.config.file_spec,
                                    &infix_from_timestamp(
                                        ts,
                                        self.config.use_utc,
                                        &InfixFormat::Std,
                                    ),
                                )
                            }
                        }
                    }
                    NamingState::NumbersRCurrent(ref mut idx_state) => {
                        *idx_state =
                            numbers::index_for_rcurrent(&self.config, Some(*idx_state), true)?;
                        CURRENT_INFIX.to_string()
                    }
                    NamingState::NumbersDirect(ref mut idx_state) => {
                        *idx_state += 1;
                        numbers::number_infix(*idx_state)
                    }
                };
                let (new_write, new_path) = open_log_file(&self.config, Some(&infix))?;

                *current_write = new_write;
                *current_path = new_path;

                rotation_state.roll_state.reset_size_and_date(current_path);

                list_and_cleanup::remove_or_compress_too_old_logfiles(
                    &rotation_state.o_cleanup_thread_handle,
                    &rotation_state.cleanup,
                    &self.config.file_spec,
                    rotation_state.naming_state.writes_direct(),
                )?;
            }
        }

        Ok(())
    }

    pub(super) fn write_buffer(&mut self, buf: &[u8]) -> std::io::Result<()> {
        if let Inner::Initial(_, _) = self.inner {
            self.initialize()?;
        }

        // rotate if necessary
        self.mount_next_linewriter_if_necessary(false)
            .unwrap_or_else(|e| {
                eprint_err(ErrorCode::LogFile, "can't open file", &e);
            });

        if let Inner::Active(ref mut o_rotation_state, ref mut log_file, ref _path) = self.inner {
            log_file.write_all(buf)?;

            if let Some(ref mut rotation_state) = o_rotation_state {
                rotation_state.roll_state.increase_size(buf.len() as u64);
            };
        }
        Ok(())
    }

    pub fn reopen_outputfile(&mut self) -> Result<(), std::io::Error> {
        if let Inner::Active(_, ref mut file, ref p_path) = self.inner {
            match OpenOptions::new().create(true).append(true).open(p_path) {
                Ok(f) => {
                    // proved to work on standard windows, linux, mac
                    *file = Box::new(f);
                }
                Err(_unexpected_error) => {
                    // there are environments, like github's windows container,
                    // where this extra step helps to overcome the _unexpected_error
                    let mut dummy = PathBuf::from(p_path);
                    dummy.set_extension("ShortLivingTempFileForReOpen");
                    *file = Box::new(OpenOptions::new().create(true).append(true).open(&dummy)?);
                    remove_file(&dummy)?;

                    *file = Box::new(OpenOptions::new().create(true).append(true).open(p_path)?);
                }
            }
        }
        Ok(())
    }

    pub fn existing_log_files(&self, selector: &LogfileSelector) -> Vec<PathBuf> {
        list_and_cleanup::existing_log_files(&self.config.file_spec, selector)
    }

    pub fn validate_logs(&mut self, expected: &[(&'static str, &'static str, &'static str)]) {
        if let Inner::Initial(_, _) = self.inner {
            self.initialize().expect("validate_logs: initialize failed");
        };
        if let Inner::Active(ref o_rotation_state, _, ref path) = self.inner {
            let rotation_possible = o_rotation_state.is_some();
            let f = File::open(path.clone()).unwrap_or_else(|e| {
                panic!(
                    "validate_logs: can't open file {} due to {e:?}",
                    path.display(),
                )
            });
            let mut reader = BufReader::new(f);
            validate_logs_in_file(&mut reader, path, expected, rotation_possible);
        } else {
            unreachable!("oiuoiuoiusdsaaöld");
        }
    }

    pub fn shutdown(&mut self) {
        if let Inner::Active(ref mut o_rotation_state, ref mut writer, _) = self.inner {
            if let Some(ref mut rotation_state) = o_rotation_state {
                rotation_state.shutdown();
            }
            writer.flush().ok();
        }
    }
}

fn prepend_underscore(infix: &str) -> String {
    if infix.is_empty() {
        infix.to_string()
    } else {
        let mut infix_with_underscore = "_".to_string();
        infix_with_underscore.push_str(infix);
        infix_with_underscore
    }
}

fn validate_logs_in_file(
    reader: &mut dyn BufRead,
    path: &Path,
    expected: &[(&'static str, &'static str, &'static str)],
    rotation_possible: bool,
) {
    let warning = if rotation_possible {
        "Warning: Validation is not fully implemented for rotation, old files are ignored"
    } else {
        ""
    };

    let mut buf = String::new();
    for tuple in expected {
        buf.clear();
        reader
            .read_line(&mut buf)
            .expect("validate_logs: can't read file");
        assert!(
            buf.contains(tuple.0),
            "Did not find tuple.0 = {} in file {}; {}",
            tuple.0,
            path.display(),
            warning
        );
        assert!(
            buf.contains(tuple.1),
            "Did not find tuple.1 = {} in file {}; {}",
            tuple.1,
            path.display(),
            warning
        );
        assert!(
            buf.contains(tuple.2),
            "Did not find tuple.2 = {} in file {}; {}",
            tuple.2,
            path.display(),
            warning
        );
    }
    buf.clear();
    reader
        .read_line(&mut buf)
        .expect("validate_logs: can't read file");
    assert!(buf.is_empty(), "Found more log lines than expected: {buf} ");
}

#[allow(clippy::type_complexity)]
fn open_log_file(
    config: &FileLogWriterConfig,
    o_infix: Option<&str>,
) -> Result<(Box<dyn Write + Send>, PathBuf), std::io::Error> {
    let path = config.file_spec.as_pathbuf(o_infix);

    if config.print_message {
        println!("Log is written to {}", &path.display());
    }
    if let Some(ref link) = config.o_create_symlink {
        self::platform::create_symlink_if_possible(link, &path);
    }

    let logfile = OpenOptions::new()
        .write(true)
        .create(true)
        .append(config.append)
        .truncate(!config.append)
        .open(&path)?;

    let w: Box<dyn Write + Send> = if let Some(capacity) = config.write_mode.buffersize() {
        Box::new(BufWriter::with_capacity(capacity, logfile))
    } else {
        Box::new(logfile)
    };
    Ok((w, path))
}

fn get_creation_timestamp(path: &Path) -> DateTime<Local> {
    // On windows, we know that try_get_creation_date() returns a result, but it is wrong.
    if cfg!(target_os = "windows") {
        get_current_timestamp()
    } else {
        // On all others of the many platforms, we give the real creation date a try,
        // and fall back if it is not available.
        try_get_creation_timestamp(path)
            .or_else(|_| try_get_modification_timestamp(path))
            .unwrap_or_else(|_| get_current_timestamp())
    }
}
fn try_get_creation_timestamp(path: &Path) -> Result<DateTime<Local>, FlexiLoggerError> {
    Ok(std::fs::metadata(path)?.created()?.into())
}
fn try_get_modification_timestamp(path: &Path) -> Result<DateTime<Local>, FlexiLoggerError> {
    let md = std::fs::metadata(path)?;
    let d = md.created().or_else(|_| md.modified())?;
    Ok(d.into())
}
fn get_current_timestamp() -> DateTime<Local> {
    Local::now()
}

#[cfg(feature = "async")]
pub(super) fn start_async_fs_writer(
    am_state: Arc<Mutex<State>>,
    message_capa: usize,
    a_pool: Arc<ArrayQueue<Vec<u8>>>,
) -> (CrossbeamSender<Vec<u8>>, Mutex<Option<JoinHandle<()>>>) {
    let (sender, receiver) = crossbeam_channel::unbounded::<Vec<u8>>();
    (
        sender,
        Mutex::new(Some(
            std::thread::Builder::new()
                .name(ASYNC_WRITER.to_string())
                .spawn(move || loop {
                    match receiver.recv() {
                        Err(_) => break,
                        Ok(mut message) => {
                            let mut state = am_state.lock().unwrap(/* ok */);
                            match message.as_ref() {
                                ASYNC_FLUSH => {
                                    state.flush().unwrap_or_else(|e| {
                                        eprint_err(ErrorCode::Flush, "flushing failed", &e);
                                    });
                                }
                                ASYNC_SHUTDOWN => {
                                    state.shutdown();
                                    break;
                                }
                                _ => {
                                    state.write_buffer(&message).unwrap_or_else(|e| {
                                        eprint_err(ErrorCode::Write, "writing failed", &e);
                                    });
                                }
                            }
                            if message.capacity() <= message_capa {
                                message.clear();
                                a_pool.push(message).ok();
                            }
                        }
                    }
                })
                .expect("Couldn't spawn flexi_logger-async_file_log_writer"),
        )),
    )
}

pub(super) fn start_sync_flusher(am_state: Arc<Mutex<State>>, flush_interval: std::time::Duration) {
    let builder = std::thread::Builder::new().name("flexi_logger-flusher".to_string());
    #[cfg(not(feature = "dont_minimize_extra_stacks"))]
    let builder = builder.stack_size(128);
    builder.spawn(move || {
        let (_tx, rx) = std::sync::mpsc::channel::<()>();
            loop {
                rx.recv_timeout(flush_interval).ok();
                (*am_state).lock().map_or_else(
                    |_e| (),
                    |mut state| {
                        state.flush().ok();
                    },
                );
            }
        })
        .unwrap(/* yes, let's panic if the thread can't be spawned */);
}

#[cfg(feature = "async")]
pub(crate) fn start_async_fs_flusher(
    async_writer: CrossbeamSender<Vec<u8>>,
    flush_interval: std::time::Duration,
) {
    use crate::util::eprint_msg;

    let builder = std::thread::Builder::new().name(ASYNC_FLUSHER.to_string());
    #[cfg(not(feature = "dont_minimize_extra_stacks"))]
    let builder = builder.stack_size(128);
    builder.spawn(move || {
            let (_tx, rx) = std::sync::mpsc::channel::<()>();
            loop {
                if let Err(std::sync::mpsc::RecvTimeoutError::Disconnected) =
                    rx.recv_timeout(flush_interval)
                {
                    eprint_msg(ErrorCode::Flush, "Flushing unexpectedly stopped working");
                    break;
                }

                async_writer.send(ASYNC_FLUSH.to_vec()).ok();
            }
        })
        .unwrap(/* yes, let's panic if the thread can't be spawned */);
}

mod platform {
    #[cfg(target_family = "unix")]
    use crate::util::{eprint_err, ErrorCode};
    use std::path::Path;

    pub fn create_symlink_if_possible(link: &Path, path: &Path) {
        unix_create_symlink(link, path);
    }

    #[cfg(target_family = "unix")]
    fn unix_create_symlink(link: &Path, logfile: &Path) {
        if std::fs::symlink_metadata(link).is_ok() {
            // remove old symlink before creating a new one
            if let Err(e) = std::fs::remove_file(link) {
                eprint_err(ErrorCode::Symlink, "cannot delete symlink to log file", &e);
            }
        }

        // create new symlink
        if let Err(e) = std::os::unix::fs::symlink(logfile, link) {
            eprint_err(ErrorCode::Symlink, "cannot create symlink to logfile", &e);
        }
    }

    #[cfg(not(target_family = "unix"))]
    fn unix_create_symlink(_: &Path, _: &Path) {}
}
