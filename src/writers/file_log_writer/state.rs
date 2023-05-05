use super::{
    config::{FileLogWriterConfig, RotationConfig},
    threads::{start_cleanup_thread, MessageToCleanupThread},
};
use crate::{
    util::{eprint_err, ErrorCode},
    Age, Cleanup, Criterion, FileSpec, FlexiLoggerError, Naming,
};
use chrono::{DateTime, Datelike, Local, Timelike};
#[cfg(feature = "external_rotation")]
use notify::{watcher, DebouncedEvent, RecursiveMode, Watcher};
use std::cmp::max;
use std::fs::{remove_file, File, OpenOptions};
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::iter::Chain;
use std::ops::Add;
use std::path::{Path, PathBuf};
use std::vec::IntoIter;
#[cfg(feature = "external_rotation")]
use std::{
    ops::Deref,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
};
const CURRENT_INFIX: &str = "_rCURRENT";
fn number_infix(idx: u32) -> String {
    format!("_r{idx:0>5}")
}

//  Describes the latest existing numbered log file.
#[derive(Clone, Copy, Debug)]
enum IdxState {
    // We rotate to numbered files, and no rotated numbered file exists yet
    Start,
    // highest index of rotated numbered files
    Idx(u32),
}

// Created_at is needed both for
//      is_rotation_necessary() -> if Criterion::Age -> NamingState::CreatedAt
//      and rotate_to_date()    -> if Naming::Timestamps -> RollState::Age
#[derive(Debug)]
enum NamingState {
    CreatedAt,
    IdxState(IdxState),
}

#[derive(Debug)]
enum RollState {
    Size(u64, u64), // max_size, current_size
    Age(Age),
    AgeOrSize(Age, u64, u64), // age, max_size, current_size
}

#[derive(Debug)]
struct CleanupThreadHandle {
    sender: std::sync::mpsc::Sender<MessageToCleanupThread>,
    join_handle: std::thread::JoinHandle<()>,
}

#[derive(Debug)]
struct RotationState {
    naming_state: NamingState,
    roll_state: RollState,
    created_at: DateTime<Local>,
    cleanup: Cleanup,
    o_cleanup_thread_handle: Option<CleanupThreadHandle>,
}
impl RotationState {
    fn size_rotation_necessary(max_size: u64, current_size: u64) -> bool {
        current_size > max_size
    }

    fn age_rotation_necessary(&self, age: Age) -> bool {
        let now = Local::now();
        match age {
            Age::Day => {
                self.created_at.year() != now.year()
                    || self.created_at.month() != now.month()
                    || self.created_at.day() != now.day()
            }
            Age::Hour => {
                self.created_at.year() != now.year()
                    || self.created_at.month() != now.month()
                    || self.created_at.day() != now.day()
                    || self.created_at.hour() != now.hour()
            }
            Age::Minute => {
                self.created_at.year() != now.year()
                    || self.created_at.month() != now.month()
                    || self.created_at.day() != now.day()
                    || self.created_at.hour() != now.hour()
                    || self.created_at.minute() != now.minute()
            }
            Age::Second => {
                self.created_at.year() != now.year()
                    || self.created_at.month() != now.month()
                    || self.created_at.day() != now.day()
                    || self.created_at.hour() != now.hour()
                    || self.created_at.minute() != now.minute()
                    || self.created_at.second() != now.second()
            }
        }
    }

    fn rotation_necessary(&self) -> bool {
        match &self.roll_state {
            RollState::Size(max_size, current_size) => {
                Self::size_rotation_necessary(*max_size, *current_size)
            }
            RollState::Age(age) => self.age_rotation_necessary(*age),
            RollState::AgeOrSize(age, max_size, current_size) => {
                Self::size_rotation_necessary(*max_size, *current_size)
                    || self.age_rotation_necessary(*age)
            }
        }
    }

    fn shutdown(&mut self) {
        // this sets o_cleanup_thread_handle in self.state.o_rotation_state to None:
        let o_cleanup_thread_handle = self.o_cleanup_thread_handle.take();
        if let Some(cleanup_thread_handle) = o_cleanup_thread_handle {
            cleanup_thread_handle
                .sender
                .send(MessageToCleanupThread::Die)
                .ok();
            cleanup_thread_handle.join_handle.join().ok();
        }
    }
}

fn try_roll_state_from_criterion(
    criterion: Criterion,
    config: &FileLogWriterConfig,
    p_path: &Path,
) -> Result<RollState, std::io::Error> {
    Ok(match criterion {
        Criterion::Age(age) => RollState::Age(age),
        Criterion::Size(size) => {
            let written_bytes = if config.append {
                std::fs::metadata(p_path)?.len()
            } else {
                0
            };
            RollState::Size(size, written_bytes)
        } // max_size, current_size
        Criterion::AgeOrSize(age, size) => {
            let written_bytes = if config.append {
                std::fs::metadata(p_path)?.len()
            } else {
                0
            };
            RollState::AgeOrSize(age, size, written_bytes)
        } // age, max_size, current_size
    })
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
    #[cfg(feature = "external_rotation")]
    external_rotation: Arc<AtomicBool>,
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
            #[cfg(feature = "external_rotation")]
            external_rotation,
        }
    }

    fn initialize(&mut self) -> Result<(), std::io::Error> {
        if let Inner::Initial(o_rotation_config, cleanup_in_background_thread) = &self.inner {
            match o_rotation_config {
                None => {
                    let (log_file, _created_at, p_path) = open_log_file(&self.config, false)?;
                    self.inner = Inner::Active(None, log_file, p_path);
                }
                Some(rotate_config) => {
                    // first rotate, then open the log file
                    let naming_state = match rotate_config.naming {
                        Naming::Timestamps => {
                            if !self.config.append {
                                rotate_output_file_to_date(
                                    &get_creation_date(
                                        &self.config.file_spec.as_pathbuf(Some(CURRENT_INFIX)),
                                    ),
                                    &self.config,
                                )?;
                            }
                            NamingState::CreatedAt
                        }
                        Naming::Numbers => {
                            let mut rotation_state = get_highest_rotate_idx(&self.config.file_spec);
                            if !self.config.append {
                                rotation_state =
                                    rotate_output_file_to_idx(rotation_state, &self.config)?;
                            }
                            NamingState::IdxState(rotation_state)
                        }
                    };
                    let (log_file, created_at, p_path) = open_log_file(&self.config, true)?;

                    let roll_state = try_roll_state_from_criterion(
                        rotate_config.criterion,
                        &self.config,
                        &p_path,
                    )?;
                    let mut o_cleanup_thread_handle = None;
                    if rotate_config.cleanup.do_cleanup() {
                        remove_or_compress_too_old_logfiles(
                            &None,
                            &rotate_config.cleanup,
                            &self.config.file_spec,
                        )?;
                        if *cleanup_in_background_thread {
                            let (sender, join_handle) = start_cleanup_thread(
                                rotate_config.cleanup,
                                self.config.file_spec.clone(),
                            )?;

                            o_cleanup_thread_handle = Some(CleanupThreadHandle {
                                sender,
                                join_handle,
                            });
                        }
                    }
                    self.inner = Inner::Active(
                        Some(RotationState {
                            naming_state,
                            roll_state,
                            created_at,
                            cleanup: rotate_config.cleanup,
                            o_cleanup_thread_handle,
                        }),
                        log_file,
                        p_path,
                    );
                }
            }
        }
        Ok(())
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

    // With rotation, the logger always writes into a file with infix `_rCURRENT`.
    // On overflow, an existing `_rCURRENT` file is renamed to the next numbered file,
    // before writing into `_rCURRENT` goes on.
    #[inline]
    fn mount_next_linewriter_if_necessary(&mut self) -> Result<(), FlexiLoggerError> {
        if let Inner::Active(Some(ref mut rotation_state), ref mut file, ref mut path) = self.inner
        {
            if rotation_state.rotation_necessary() {
                match rotation_state.naming_state {
                    NamingState::CreatedAt => {
                        rotate_output_file_to_date(&rotation_state.created_at, &self.config)?;
                    }
                    NamingState::IdxState(ref mut idx_state) => {
                        *idx_state = rotate_output_file_to_idx(*idx_state, &self.config)?;
                    }
                }

                let (line_writer, created_at, p_path) = open_log_file(&self.config, true)?;
                *file = line_writer;
                *path = p_path;
                rotation_state.created_at = created_at;
                if let RollState::Size(_, ref mut current_size)
                | RollState::AgeOrSize(_, _, ref mut current_size) = rotation_state.roll_state
                {
                    *current_size = 0;
                }

                remove_or_compress_too_old_logfiles(
                    &rotation_state.o_cleanup_thread_handle,
                    &rotation_state.cleanup,
                    &self.config.file_spec,
                )?;
            }
        }

        Ok(())
    }

    pub(super) fn write_buffer(&mut self, buf: &[u8]) -> std::io::Result<()> {
        if let Inner::Initial(_, _) = self.inner {
            self.initialize()?;
        }

        #[cfg(feature = "external_rotation")]
        self.react_on_external_rotation()?;

        // rotate if necessary
        self.mount_next_linewriter_if_necessary()
            .unwrap_or_else(|e| {
                eprint_err(ErrorCode::LogFile, "can't open file", &e);
            });

        if let Inner::Active(ref mut o_rotation_state, ref mut log_file, ref _path) = self.inner {
            log_file.write_all(buf)?;
            if let Some(ref mut rotation_state) = o_rotation_state {
                if let RollState::Size(_, ref mut current_size)
                | RollState::AgeOrSize(_, _, ref mut current_size) = rotation_state.roll_state
                {
                    *current_size += buf.len() as u64;
                }
            };
        }
        Ok(())
    }

    pub fn current_filename(&self) -> PathBuf {
        let o_infix = match &self.inner {
            Inner::Initial(o_rotation_config, _) => {
                if o_rotation_config.is_some() {
                    Some(CURRENT_INFIX)
                } else {
                    None
                }
            }
            Inner::Active(o_rotation_state, _, _) => {
                if o_rotation_state.is_some() {
                    Some(CURRENT_INFIX)
                } else {
                    None
                }
            }
        };
        self.config.file_spec.as_pathbuf(o_infix)
    }

    // check if the currently used output file does still exist, and if not, then create and open it
    #[cfg(feature = "external_rotation")]
    fn react_on_external_rotation(&mut self) -> Result<(), std::io::Error> {
        if self
            .external_rotation
            .deref()
            .swap(false, Ordering::Relaxed)
        {
            if let Inner::Active(_, ref mut file, ref p_path) = self.inner {
                if std::fs::metadata(p_path).is_err() {
                    *file = Box::new(OpenOptions::new().create(true).append(true).open(&p_path)?);
                }
            }
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

    pub fn existing_log_files(&self) -> Vec<PathBuf> {
        let mut list: Vec<PathBuf> =
            list_of_log_and_compressed_files(&self.config.file_spec).collect();
        list.push(self.config.file_spec.as_pathbuf(Some(CURRENT_INFIX)));
        list
    }

    pub fn validate_logs(&mut self, expected: &[(&'static str, &'static str, &'static str)]) {
        if let Inner::Initial(_, _) = self.inner {
            self.initialize().expect("validate_logs: initialize failed");
        };
        if let Inner::Active(ref o_rotation_state, _, ref path) = self.inner {
            let rotation_possible = o_rotation_state.is_some();
            let f = File::open(path.clone()).unwrap_or_else(|e| {
                panic!(
                    "validate_logs: can't open file {} due to {:?}",
                    path.display(),
                    e
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
    with_rotation: bool,
) -> Result<(Box<dyn Write + Send>, DateTime<Local>, PathBuf), std::io::Error> {
    let path = config
        .file_spec
        .as_pathbuf(with_rotation.then_some(CURRENT_INFIX));

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
    Ok((w, get_creation_date(&path), path))
}

fn get_highest_rotate_idx(file_spec: &FileSpec) -> IdxState {
    let mut highest_idx = IdxState::Start;
    for file in list_of_log_and_compressed_files(file_spec) {
        let filename = file.file_stem().unwrap(/*ok*/).to_string_lossy();
        let mut it = filename.rsplit("_r");
        match it.next() {
            Some(next) => {
                let idx: u32 = next.parse().unwrap_or(0);
                highest_idx = match highest_idx {
                    IdxState::Start => IdxState::Idx(idx),
                    IdxState::Idx(prev) => IdxState::Idx(max(prev, idx)),
                };
            }
            None => continue, // ignore unexpected files
        }
    }
    highest_idx
}

#[allow(clippy::type_complexity)]
fn list_of_log_and_compressed_files(
    file_spec: &FileSpec,
) -> Chain<Chain<IntoIter<PathBuf>, IntoIter<PathBuf>>, IntoIter<PathBuf>> {
    let o_infix = Some("_r[0-9]*");

    let log_pattern = file_spec.as_glob_pattern(o_infix, None);
    let zip_pattern = file_spec.as_glob_pattern(o_infix, Some("zip"));
    let gz_pattern = file_spec.as_glob_pattern(o_infix, Some("gz"));

    list_of_files(&log_pattern)
        .chain(list_of_files(&gz_pattern))
        .chain(list_of_files(&zip_pattern))
}

fn list_of_files(pattern: &str) -> std::vec::IntoIter<PathBuf> {
    let mut log_files: Vec<PathBuf> = glob::glob(pattern)
        .unwrap(/* failure should be impossible */)
        .filter_map(Result::ok)
        .collect();
    log_files.reverse();
    log_files.into_iter()
}

fn remove_or_compress_too_old_logfiles(
    o_cleanup_thread_handle: &Option<CleanupThreadHandle>,
    cleanup_config: &Cleanup,
    file_spec: &FileSpec,
) -> Result<(), std::io::Error> {
    o_cleanup_thread_handle.as_ref().map_or_else(
        || remove_or_compress_too_old_logfiles_impl(cleanup_config, file_spec),
        |cleanup_thread_handle| {
            cleanup_thread_handle
                .sender
                .send(MessageToCleanupThread::Act)
                .ok();
            Ok(())
        },
    )
}

pub(crate) fn remove_or_compress_too_old_logfiles_impl(
    cleanup_config: &Cleanup,
    file_spec: &FileSpec,
) -> Result<(), std::io::Error> {
    let (log_limit, compress_limit) = match *cleanup_config {
        Cleanup::Never => {
            return Ok(());
        }
        Cleanup::KeepLogFiles(log_limit) => (log_limit, 0),

        #[cfg(feature = "compress")]
        Cleanup::KeepCompressedFiles(compress_limit) => (0, compress_limit),

        #[cfg(feature = "compress")]
        Cleanup::KeepLogAndCompressedFiles(log_limit, compress_limit) => {
            (log_limit, compress_limit)
        }
    };

    for (index, file) in list_of_log_and_compressed_files(file_spec).enumerate() {
        if index >= log_limit + compress_limit {
            // delete (log or log.gz)
            std::fs::remove_file(file)?;
        } else if index >= log_limit {
            #[cfg(feature = "compress")]
            {
                // compress, if not yet compressed
                if let Some(extension) = file.extension() {
                    if extension != "gz" {
                        let mut old_file = File::open(file.clone())?;
                        let mut compressed_file = file.clone();
                        compressed_file.set_extension("log.gz");
                        let mut gz_encoder = flate2::write::GzEncoder::new(
                            File::create(compressed_file)?,
                            flate2::Compression::fast(),
                        );
                        std::io::copy(&mut old_file, &mut gz_encoder)?;
                        gz_encoder.finish()?;
                        std::fs::remove_file(&file)?;
                    }
                }
            }
        }
    }

    Ok(())
}

// Moves the current file to the timestamp of the CURRENT file's creation date.
// If the rotation comes very fast, the new timestamp can be equal to the old one.
// To avoid file collisions, we insert an additional string to the filename (".restart-<number>").
// The number is incremented in case of repeated collisions.
// Cleaning up can leave some restart-files with higher numbers; if we still are in the same
// second, we need to continue with the restart-incrementing.
fn rotate_output_file_to_date(
    creation_date: &DateTime<Local>,
    config: &FileLogWriterConfig,
) -> Result<(), std::io::Error> {
    let current_path = config.file_spec.as_pathbuf(Some(CURRENT_INFIX));
    let rotated_path = determine_new_name_for_rotate_output_file_to_date(
        &config.file_spec,
        config.use_utc,
        creation_date,
    );

    match std::fs::rename(current_path, rotated_path) {
        Ok(()) => Ok(()),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                // current did not exist, so we had nothing to do
                Ok(())
            } else {
                Err(e)
            }
        }
    }
}

fn determine_new_name_for_rotate_output_file_to_date(
    file_spec: &FileSpec,
    use_utc: bool,
    creation_date: &DateTime<Local>,
) -> PathBuf {
    const INFIX_DATE: &str = "_r%Y-%m-%d_%H-%M-%S";
    let infix_date_string = if use_utc {
        (*creation_date).naive_utc().format(INFIX_DATE)
    } else {
        (*creation_date).format(INFIX_DATE)
    }
    .to_string();

    let mut rotated_path = file_spec.as_pathbuf(Some(&infix_date_string));
    // Search for rotated_path as is and for restart-siblings;
    // if any exists, find highest restart and add 1, else continue without restart
    let mut pattern = rotated_path.clone();
    if file_spec.o_suffix.is_some() {
        pattern.set_extension("");
    }
    let mut pattern = pattern.to_string_lossy().to_string();
    pattern.push_str(".restart-*");
    let mut restart_siblings: Vec<PathBuf> =
        glob::glob(&pattern).unwrap(/*ok*/).map(Result::unwrap).collect();
    restart_siblings.sort_unstable();
    if (*rotated_path).exists() || !restart_siblings.is_empty() {
        let mut number = if restart_siblings.is_empty() {
            0
        } else {
            rotated_path = restart_siblings.pop().unwrap(/*ok*/);
            let file_stem_string = if file_spec.o_suffix.is_some() {
                rotated_path
                    .file_stem().unwrap(/*ok*/)
                    .to_string_lossy().to_string()
            } else {
                rotated_path.to_string_lossy().to_string()
            };
            let index = file_stem_string.find(".restart-").unwrap(/*ok*/);
            file_stem_string[(index + 9)..].parse::<usize>().unwrap(/*ok*/)
        };

        while (*rotated_path).exists() {
            rotated_path = file_spec.as_pathbuf(Some(
                &infix_date_string
                    .clone()
                    .add(&format!(".restart-{number:04}")),
            ));
            number += 1;
        }
    }
    rotated_path
}

// Moves the current file to the name with the next rotate_idx and returns the next rotate_idx.
// The current file must be closed already.
fn rotate_output_file_to_idx(
    idx_state: IdxState,
    config: &FileLogWriterConfig,
) -> Result<IdxState, std::io::Error> {
    let new_idx = match idx_state {
        IdxState::Start => 0,
        IdxState::Idx(idx) => idx + 1,
    };

    match std::fs::rename(
        config.file_spec.as_pathbuf(Some(CURRENT_INFIX)),
        config.file_spec.as_pathbuf(Some(&number_infix(new_idx))),
    ) {
        Ok(()) => Ok(IdxState::Idx(new_idx)),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                // current did not exist, so we had nothing to do
                Ok(idx_state)
            } else {
                Err(e)
            }
        }
    }
}

// See documentation of Criterion::Age.
fn get_creation_date(path: &Path) -> DateTime<Local> {
    // On windows, we know that try_get_creation_date() returns a result, but it is wrong.
    if cfg!(target_os = "windows") {
        get_current_date()
    } else {
        // On all others of the many platforms, we give the real creation date a try,
        // and fall back if it is not available.
        try_get_creation_date(path)
            .or_else(|_| try_get_modification_date(path))
            .unwrap_or_else(|_| get_current_date())
    }
}

fn try_get_creation_date(path: &Path) -> Result<DateTime<Local>, FlexiLoggerError> {
    Ok(std::fs::metadata(path)?.created()?.into())
}
fn try_get_modification_date(path: &Path) -> Result<DateTime<Local>, FlexiLoggerError> {
    let md = std::fs::metadata(path)?;
    let d = md.created().or_else(|_| md.modified())?;
    Ok(d.into())
}
fn get_current_date() -> DateTime<Local> {
    Local::now()
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
