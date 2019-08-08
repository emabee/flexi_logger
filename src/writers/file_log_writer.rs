use crate::deferred_now::DeferredNow;
use crate::flexi_error::FlexiLoggerError;
use crate::formats::default_format;
use crate::logger::{Age, Cleanup, Criterion, Naming};
use crate::primary_writer::buffer_with;
use crate::writers::log_writer::LogWriter;
use crate::FormatFunction;
use chrono::{DateTime, Datelike, Local, Timelike};
use log::Record;

use std::borrow::BorrowMut;
use std::cmp::max;
use std::env;
use std::fs::{File, OpenOptions};
#[cfg(feature = "ziplogs")]
use std::io::Read;
use std::io::{BufRead, BufReader, Write};
use std::ops::{Add, Deref, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const CURRENT_INFIX: &str = "_rCURRENT";
fn number_infix(idx: u32) -> String {
    format!("_r{:0>5}", idx)
}

// Describes how rotation should work
struct RotationConfig {
    // Defines if rotation should be based on size or date
    criterion: Criterion,
    // Defines if rotated files should be numbered or get a date-based name
    naming: Naming,
    // Defines the cleanup strategy
    cleanup: Cleanup,
}
struct FilenameConfig {
    directory: PathBuf,
    file_basename: String,
    suffix: String,
    use_timestamp: bool,
}
// The immutable configuration of a FileLogWriter.
struct FileLogWriterConfig {
    format: FormatFunction,
    print_message: bool,
    append: bool,
    filename_config: FilenameConfig,
    o_create_symlink: Option<PathBuf>,
    use_windows_line_ending: bool,
}
impl FileLogWriterConfig {
    // Factory method; uses the same defaults as Logger.
    pub fn default() -> FileLogWriterConfig {
        FileLogWriterConfig {
            format: default_format,
            print_message: false,
            filename_config: FilenameConfig {
                directory: PathBuf::from("."),
                file_basename: String::new(),
                suffix: "log".to_string(),
                use_timestamp: true,
            },
            append: false,
            o_create_symlink: None,
            use_windows_line_ending: false,
        }
    }
}

/// Builder for `FileLogWriter`.
pub struct FileLogWriterBuilder {
    discriminant: Option<String>,
    config: FileLogWriterConfig,
    o_rotation_config: Option<RotationConfig>,
    max_log_level: log::LevelFilter,
}

/// Simple methods for influencing the behavior of the `FileLogWriter`.
impl FileLogWriterBuilder {
    /// Makes the `FileLogWriter` print an info message to stdout
    /// when a new file is used for log-output.
    pub fn print_message(mut self) -> FileLogWriterBuilder {
        self.config.print_message = true;
        self
    }

    /// Makes the `FileLogWriter` use the provided format function for the log entries,
    /// rather than the default ([formats::default_format](fn.default_format.html)).
    pub fn format(mut self, format: FormatFunction) -> FileLogWriterBuilder {
        self.config.format = format;
        self
    }

    /// Specifies a folder for the log files.
    ///
    /// If the specified folder does not exist, the initialization will fail.
    /// By default, the log files are created in the folder where the program was started.
    pub fn directory<P: Into<PathBuf>>(mut self, directory: P) -> FileLogWriterBuilder {
        self.config.filename_config.directory = directory.into();
        self
    }

    /// Specifies a suffix for the log files. The default is "log".
    pub fn suffix<S: Into<String>>(mut self, suffix: S) -> FileLogWriterBuilder {
        self.config.filename_config.suffix = suffix.into();
        self
    }

    /// Makes the logger not include a timestamp into the names of the log files
    pub fn suppress_timestamp(mut self) -> FileLogWriterBuilder {
        self.config.filename_config.use_timestamp = false;
        self
    }

    /// Use rotation to prevent indefinite growth of log files.
    ///
    /// By default, the log file is fixed while your program is running and will grow indefinitely.
    /// With this option being used, when the log file reaches the specified criterion,
    /// the file will be closed and a new file will be opened.
    ///
    /// Note that also the filename pattern changes:
    ///
    /// - by default, no timestamp is added to the filename
    /// - the logs are always written to a file with infix `_rCURRENT`
    /// - when the rotation criterion is fulfilled, it is closed and renamed to a file
    ///   with another infix (see `Naming`),
    ///   and then the logging continues again to the (fresh) file with infix `_rCURRENT`.
    ///
    /// Example:
    ///
    /// After some logging with your program `my_prog` and rotation with `Naming::Numbers`,
    /// you will find files like
    ///
    /// ```text
    /// my_prog_r00000.log
    /// my_prog_r00001.log
    /// my_prog_r00002.log
    /// my_prog_rCURRENT.log
    /// ```
    ///
    /// The cleanup parameter allows defining the strategy for dealing with older files.
    /// See [Cleanup](enum.Cleanup.html) for details.
    pub fn rotate(
        mut self,
        criterion: Criterion,
        naming: Naming,
        cleanup: Cleanup,
    ) -> FileLogWriterBuilder {
        self.o_rotation_config = Some(RotationConfig {
            criterion,
            naming,
            cleanup,
        });
        self.config.filename_config.use_timestamp = false;
        self
    }

    /// Makes the logger append to the given file, if it exists; by default, the file would be
    /// truncated.
    pub fn append(mut self) -> FileLogWriterBuilder {
        self.config.append = true;
        self
    }

    /// The specified String is added to the log file name.
    pub fn discriminant<S: Into<String>>(mut self, discriminant: S) -> FileLogWriterBuilder {
        self.discriminant = Some(discriminant.into());
        self
    }

    /// The specified String will be used on linux systems to create in the current folder
    /// a symbolic link to the current log file.
    pub fn create_symlink<P: Into<PathBuf>>(mut self, symlink: P) -> FileLogWriterBuilder {
        self.config.o_create_symlink = Some(symlink.into());
        self
    }

    /// Use Windows line endings, rather than just `\n`.
    pub fn use_windows_line_ending(mut self) -> FileLogWriterBuilder {
        self.config.use_windows_line_ending = true;
        self
    }

    /// Produces the FileLogWriter.
    pub fn try_build(mut self) -> Result<FileLogWriter, FlexiLoggerError> {
        // make sure the folder exists or create it
        let p_directory = Path::new(&self.config.filename_config.directory);
        std::fs::create_dir_all(&p_directory)?;
        if !std::fs::metadata(&p_directory)?.is_dir() {
            return Err(FlexiLoggerError::BadDirectory);
        };

        let arg0 = env::args().nth(0).unwrap_or_else(|| "rs".to_owned());
        self.config.filename_config.file_basename =
            Path::new(&arg0).file_stem().unwrap(/*cannot fail*/).to_string_lossy().to_string();

        if let Some(discriminant) = self.discriminant {
            self.config.filename_config.file_basename += &format!("_{}", discriminant);
        }
        if self.config.filename_config.use_timestamp {
            self.config.filename_config.file_basename +=
                &Local::now().format("_%Y-%m-%d_%H-%M-%S").to_string();
        };

        Ok(FileLogWriter {
            state: Mutex::new(FileLogWriterState::try_new(
                &self.config,
                &self.o_rotation_config,
            )?),
            config: self.config,
            max_log_level: self.max_log_level,
        })
    }
}

/// Alternative set of methods to control the behavior of the `FileLogWriterBuilder`.
/// Use these methods when you want to control the settings flexibly,
/// e.g. with commandline arguments via `docopts` or `clap`.
impl FileLogWriterBuilder {
    /// With true, makes the FileLogWriterBuilder print an info message to stdout, each time
    /// when a new file is used for log-output.
    pub fn o_print_message(mut self, print_message: bool) -> FileLogWriterBuilder {
        self.config.print_message = print_message;
        self
    }

    /// Specifies a folder for the log files.
    ///
    /// If the specified folder does not exist, the initialization will fail.
    /// With None, the log files are created in the folder where the program was started.
    pub fn o_directory<P: Into<PathBuf>>(mut self, directory: Option<P>) -> FileLogWriterBuilder {
        self.config.filename_config.directory = directory
            .map(Into::into)
            .unwrap_or_else(|| PathBuf::from("."));
        self
    }

    /// With true, makes the FileLogWriterBuilder include a timestamp into the names of the log files.
    pub fn o_timestamp(mut self, use_timestamp: bool) -> FileLogWriterBuilder {
        self.config.filename_config.use_timestamp = use_timestamp;
        self
    }

    /// By default, and with None, the log file will grow indefinitely.
    /// If a rotate_config is set, when the log file reaches or exceeds the specified size,
    /// the file will be closed and a new file will be opened.
    /// Also the filename pattern changes: instead of the timestamp, a serial number
    /// is included into the filename.
    ///
    /// The size is given in bytes, e.g. `o_rotate_over_size(Some(1_000))` will rotate
    /// files once they reach a size of 1 kB.
    ///
    /// The cleanup strategy allows delimiting the used space on disk.
    pub fn o_rotate(
        mut self,
        rotate_config: Option<(Criterion, Naming, Cleanup)>,
    ) -> FileLogWriterBuilder {
        match rotate_config {
            Some((criterion, naming, cleanup)) => {
                self.o_rotation_config = Some(RotationConfig {
                    criterion,
                    naming,
                    cleanup,
                });
                self.config.filename_config.use_timestamp = false;
            }
            None => {
                self.o_rotation_config = None;
                self.config.filename_config.use_timestamp = true;
            }
        }
        self
    }

    /// If append is set to true, makes the logger append to the given file, if it exists.
    /// By default, or with false, the file would be truncated.
    pub fn o_append(mut self, append: bool) -> FileLogWriterBuilder {
        self.config.append = append;
        self
    }

    /// The specified String is added to the log file name.
    pub fn o_discriminant<S: Into<String>>(
        mut self,
        discriminant: Option<S>,
    ) -> FileLogWriterBuilder {
        self.discriminant = discriminant.map(Into::into);
        self
    }

    /// If a String is specified, it will be used on linux systems to create in the current folder
    /// a symbolic link with this name to the current log file.
    pub fn o_create_symlink<S: Into<PathBuf>>(
        mut self,
        symlink: Option<S>,
    ) -> FileLogWriterBuilder {
        self.config.o_create_symlink = symlink.map(Into::into);
        self
    }
}

//  Describes the latest existing numbered log file.
#[derive(Clone, Copy)]
enum IdxState {
    // We rotate to numbered files, and no rotated numbered file exists yet
    Start,
    // highest index of rotated numbered files
    Idx(u32),
}

// Created_at is needed both for
//      is_rotation_necessary() -> if Criterion::Age -> NamingState::CreatedAt
//      and rotate_to_date()    -> if Naming::Timestamps -> RollState::Age
enum NamingState {
    CreatedAt,
    IdxState(IdxState),
}

enum RollState {
    Size(u64, u64), // max_size, current_size
    Age(Age),
}

struct RotationState {
    naming_state: NamingState,
    roll_state: RollState,
    created_at: DateTime<Local>,
    cleanup: Cleanup,
}
impl RotationState {
    fn rotation_necessary(&self) -> bool {
        match &self.roll_state {
            RollState::Size(max_size, current_size) => current_size > max_size,
            RollState::Age(age) => {
                let now = Local::now();
                match age {
                    Age::Day => self.created_at.num_days_from_ce() != now.num_days_from_ce(),
                    Age::Hour => {
                        self.created_at.num_days_from_ce() != now.num_days_from_ce()
                            || self.created_at.hour() != now.hour()
                    }
                    Age::Minute => {
                        self.created_at.num_days_from_ce() != now.num_days_from_ce()
                            || self.created_at.hour() != now.hour()
                            || self.created_at.minute() != now.minute()
                    }
                    Age::Second => {
                        self.created_at.num_days_from_ce() != now.num_days_from_ce()
                            || self.created_at.hour() != now.hour()
                            || self.created_at.minute() != now.minute()
                            || self.created_at.second() != now.second()
                    }
                }
            }
        }
    }
}

// The mutable state of a FileLogWriter.
struct FileLogWriterState {
    o_log_file: Option<File>,
    o_rotation_state: Option<RotationState>,
    line_ending: &'static [u8],
}
impl FileLogWriterState {
    // If rotate, the logger writes into a file with infix `_rCURRENT`.
    fn try_new(
        config: &FileLogWriterConfig,
        o_rotation_config: &Option<RotationConfig>,
    ) -> Result<FileLogWriterState, FlexiLoggerError> {
        let (log_file, o_rotation_state) = match o_rotation_config {
            None => {
                let (log_file, _created_at, _p_path) = open_log_file(config, false)?;
                (log_file, None)
            }
            Some(rotate_config) => {
                // first rotate, then open the log file
                let naming_state = match rotate_config.naming {
                    Naming::Timestamps => {
                        if !config.append {
                            rotate_output_file_to_date(
                                &get_creation_date(&get_filepath(
                                    Some(CURRENT_INFIX),
                                    &config.filename_config,
                                ))?,
                                config,
                            )?;
                        }
                        NamingState::CreatedAt
                    }
                    Naming::Numbers => {
                        let mut rotation_state = get_highest_rotate_idx(&config.filename_config);
                        if !config.append {
                            rotation_state = rotate_output_file_to_idx(rotation_state, config)?;
                        }
                        NamingState::IdxState(rotation_state)
                    }
                };
                let (log_file, created_at, p_path) = open_log_file(config, true)?;

                let cleanup = rotate_config.cleanup;

                let roll_state = match &rotate_config.criterion {
                    Criterion::Age(age) => RollState::Age(*age),
                    Criterion::Size(size) => {
                        let written_bytes = if config.append {
                            std::fs::metadata(&p_path)?.len()
                        } else {
                            0
                        };
                        RollState::Size(*size, written_bytes)
                    } // max_size, current_size
                };

                (
                    log_file,
                    Some(RotationState {
                        naming_state,
                        roll_state,
                        created_at,
                        cleanup,
                    }),
                )
            }
        };

        Ok(FileLogWriterState {
            o_log_file: Some(log_file),
            o_rotation_state,
            line_ending: if config.use_windows_line_ending {
                b"\r\n"
            } else {
                b"\n"
            },
        })
    }

    // With rotation, the logger always writes into a file with infix `_rCURRENT`.
    // On overflow, an existing `_rCURRENT` file is renamed to the next numbered file,
    // before writing into `_rCURRENT` goes on.
    #[inline]
    fn mount_next_linewriter_if_necessary(
        &mut self,
        config: &FileLogWriterConfig,
    ) -> Result<(), FlexiLoggerError> {
        if let Some(ref mut rotation_state) = self.o_rotation_state {
            if rotation_state.rotation_necessary() {
                self.o_log_file = None; // close the output file

                match rotation_state.naming_state {
                    NamingState::CreatedAt => {
                        rotate_output_file_to_date(&rotation_state.created_at, config)?;
                    }
                    NamingState::IdxState(ref mut idx_state) => {
                        *idx_state = rotate_output_file_to_idx(*idx_state, config)?;
                    }
                }

                let (line_writer, created_at, _) = open_log_file(config, true)?;
                self.o_log_file = Some(line_writer);
                rotation_state.created_at = created_at;
                if let RollState::Size(_max_size, ref mut current_size) = rotation_state.roll_state
                {
                    *current_size = 0;
                }

                let cleanup_config: &Cleanup = &rotation_state.cleanup;
                let filename_config: &FilenameConfig = &config.filename_config;
                remove_or_zip_too_old_logfiles(cleanup_config, filename_config)?;
            }
        }

        Ok(())
    }

    fn write_buffer(&mut self, buf: &[u8]) -> std::io::Result<()> {
        self.o_log_file
            .as_mut()
            .expect("FlexiLogger: log_file unexpectedly not available")
            .write_all(buf)?;

        if let Some(ref mut rotation_state) = self.o_rotation_state {
            if let RollState::Size(_max_size, ref mut current_size) = rotation_state.roll_state {
                *current_size += buf.len() as u64;
            }
        };
        Ok(())
    }
}

fn get_filepath(o_infix: Option<&str>, config: &FilenameConfig) -> PathBuf {
    let mut s_filename = String::with_capacity(
        config.file_basename.len() + o_infix.map(str::len).unwrap_or(0) + 1 + config.suffix.len(),
    ) + &config.file_basename;
    if let Some(infix) = o_infix {
        s_filename += infix;
    };
    s_filename += ".";
    s_filename += &config.suffix;
    let mut p_path = config.directory.to_path_buf();
    p_path.push(s_filename);
    p_path
}

fn open_log_file(
    config: &FileLogWriterConfig,
    with_rotation: bool,
) -> Result<(File, DateTime<Local>, PathBuf), FlexiLoggerError> {
    let o_infix = if with_rotation {
        Some(CURRENT_INFIX)
    } else {
        None
    };
    let p_path = get_filepath(o_infix, &config.filename_config);
    if config.print_message {
        println!("Log is written to {}", &p_path.display());
    }
    if let Some(ref link) = config.o_create_symlink {
        self::platform::create_symlink_if_possible(link, &p_path);
    }

    let log_file = OpenOptions::new()
        .write(true)
        .create(true)
        .append(config.append)
        .truncate(!config.append)
        .open(&p_path)?;

    Ok((log_file, get_creation_date(&p_path)?, p_path))
}

fn get_highest_rotate_idx(filename_config: &FilenameConfig) -> IdxState {
    match list_of_log_and_zip_files(filename_config) {
        Err(e) => {
            eprintln!("[flexi_logger] listing rotated log files failed with {}", e);
            IdxState::Start // hope and pray ...??
        }
        Ok(globresults) => {
            let mut highest_idx = IdxState::Start;
            for globresult in globresults {
                match globresult {
                    Err(e) => eprintln!(
                        "[flexi_logger] error when reading directory for log files: {:?}",
                        e
                    ),
                    Ok(pathbuf) => {
                        let filename = pathbuf.file_stem().unwrap().to_string_lossy();
                        let mut it = filename.rsplit("_r");
                        let idx: u32 = it.next().unwrap().parse().unwrap_or(0);
                        highest_idx = match highest_idx {
                            IdxState::Start => IdxState::Idx(idx),
                            IdxState::Idx(prev) => IdxState::Idx(max(prev, idx)),
                        };
                    }
                }
            }
            highest_idx
        }
    }
}

fn list_of_log_and_zip_files(
    filename_config: &FilenameConfig,
) -> Result<std::iter::Chain<glob::Paths, glob::Paths>, FlexiLoggerError> {
    let fn_pattern = String::with_capacity(180)
        .add(&filename_config.file_basename)
        .add("_r[0-9]*")
        .add(".");

    let mut log_pattern = filename_config.directory.clone();
    log_pattern.push(fn_pattern.clone().add(&filename_config.suffix));
    let mut zip_pattern = filename_config.directory.clone();
    zip_pattern.push(fn_pattern.clone().add("zip"));
    Ok(glob::glob(&log_pattern.as_os_str().to_string_lossy())?
        .chain(glob::glob(&zip_pattern.as_os_str().to_string_lossy())?))
}

fn remove_or_zip_too_old_logfiles(
    cleanup_config: &Cleanup,
    filename_config: &FilenameConfig,
) -> Result<(), FlexiLoggerError> {
    let (log_limit, zip_limit) = match *cleanup_config {
        Cleanup::Never => {
            return Ok(());
        }
        Cleanup::KeepLogFiles(log_limit) => (log_limit, 0),
        #[cfg(feature = "ziplogs")]
        Cleanup::KeepZipFiles(zip_limit) => (0, zip_limit),
        #[cfg(feature = "ziplogs")]
        Cleanup::KeepLogAndZipFiles(log_limit, zip_limit) => (log_limit, zip_limit),
    };
    // list files by name, in ascending order
    let mut file_list: Vec<_> = list_of_log_and_zip_files(&filename_config)?
        .filter_map(Result::ok)
        .collect();
    file_list.sort_unstable();
    let total_number_of_files = file_list.len();

    // now do the work
    for (index, file) in file_list.iter().enumerate() {
        if total_number_of_files - index > log_limit + zip_limit {
            // delete (zip or log)
            std::fs::remove_file(&file)?;
        } else if total_number_of_files - index > log_limit {
            // zip, if not yet zipped
            #[cfg(feature = "ziplogs")]
            {
                if let Some(extension) = file.extension() {
                    if extension != "zip" {
                        let mut old_file = File::open(file)?;
                        let mut zip_file = file.clone();
                        zip_file.set_extension("zip");
                        let mut zip = zip::ZipWriter::new(File::create(zip_file)?);

                        let options = zip::write::FileOptions::default()
                            .compression_method(zip::CompressionMethod::Bzip2);
                        zip.start_file(file.file_name().unwrap().to_string_lossy(), options)?;
                        {
                            // streaming does not work easily :-(
                            // std::io::copy(&mut old_file, &mut zip)?;
                            let mut buf = Vec::<u8>::new();
                            old_file.read_to_end(&mut buf)?;
                            zip.write_all(&buf)?;
                        }
                        zip.finish()?;
                        std::fs::remove_file(&file)?;
                    }
                }
            }
        }
    }

    Ok(())
}

// Moves the current file to the timestamp of the CURRENT file's creation date.
fn rotate_output_file_to_date(
    creation_date: &DateTime<Local>,
    config: &FileLogWriterConfig,
) -> Result<(), FlexiLoggerError> {
    let current_path = get_filepath(Some(CURRENT_INFIX), &config.filename_config);

    let mut rotated_path = get_filepath(
        Some(&creation_date.format("_r%Y-%m-%d_%H-%M-%S").to_string()),
        &config.filename_config,
    );
    // Check that the target of rename does not yet exist
    let mut i = 0_u32;
    while (*rotated_path).exists() {
        rotated_path = get_filepath(
            Some(
                &creation_date
                    .format("_r%Y-%m-%d_%H-%M-%S")
                    .to_string()
                    .add(&format!("-restart-{}", i)),
            ),
            &config.filename_config,
        );
        i += 1;
    }

    match std::fs::rename(&current_path, &rotated_path) {
        Ok(()) => Ok(()),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                // current did not exist, so we had nothing to do
                Ok(())
            } else {
                Err(FlexiLoggerError::Io(e))
            }
        }
    }
}

// Moves the current file to the name with the next rotate_idx and returns the next rotate_idx.
// The current file must be closed already.
fn rotate_output_file_to_idx(
    idx_state: IdxState,
    config: &FileLogWriterConfig,
) -> Result<IdxState, FlexiLoggerError> {
    let new_idx = match idx_state {
        IdxState::Start => 0,
        IdxState::Idx(idx) => idx + 1,
    };

    match std::fs::rename(
        get_filepath(Some(CURRENT_INFIX), &config.filename_config),
        get_filepath(Some(&number_infix(new_idx)), &config.filename_config),
    ) {
        Ok(()) => Ok(IdxState::Idx(new_idx)),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                // current did not exist, so we had nothing to do
                Ok(idx_state)
            } else {
                Err(FlexiLoggerError::Io(e))
            }
        }
    }
}

// See documentation of Criterion::Age.
#[allow(unused_variables)]
fn get_creation_date(path: &PathBuf) -> Result<DateTime<Local>, FlexiLoggerError> {
    // On windows, we know that try_get_creation_date() returns a result, but it is wrong.
    // On linux, we know that try_get_creation_date() returns an error.
    #[cfg(any(target_os = "windows", target_os = "linux"))]
    return get_fake_creation_date();

    // On all others of the many platforms, we give the real creation date a try,
    // and fall back to the fake if it is not available.
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    match try_get_creation_date(path) {
        Ok(d) => Ok(d),
        Err(e) => get_fake_creation_date(),
    }
}

fn get_fake_creation_date() -> Result<DateTime<Local>, FlexiLoggerError> {
    Ok(Local::now())
}

#[cfg(not(any(target_os = "windows", target_os = "linux")))]
fn try_get_creation_date(path: &PathBuf) -> Result<DateTime<Local>, FlexiLoggerError> {
    Ok(std::fs::metadata(path)?.created()?.into())
}

/// A configurable `LogWriter` implementation that writes to a file or a sequence of files.
///
/// See the [module description](index.html) for usage guidance.
pub struct FileLogWriter {
    config: FileLogWriterConfig,
    // the state needs to be mutable; since `Log.log()` requires an unmutable self,
    // which translates into a non-mutating `LogWriter::write()`,
    // we need internal mutability and thread-safety.
    state: Mutex<FileLogWriterState>,
    max_log_level: log::LevelFilter,
}
impl FileLogWriter {
    /// Instantiates a builder for `FileLogWriter`.
    pub fn builder() -> FileLogWriterBuilder {
        FileLogWriterBuilder {
            discriminant: None,
            o_rotation_config: None,
            config: FileLogWriterConfig::default(),
            max_log_level: log::LevelFilter::Trace,
        }
    }

    /// Returns a reference to its configured output format function.
    #[inline]
    pub fn format(&self) -> FormatFunction {
        self.config.format
    }

    #[doc(hidden)]
    pub fn current_filename(&self) -> PathBuf {
        let o_infix = if self
            .state
            .lock()
            .unwrap()
            .deref()
            .o_rotation_state
            .is_some()
        {
            Some(CURRENT_INFIX)
        } else {
            None
        };
        let p_path = get_filepath(o_infix, &self.config.filename_config);
        p_path.clone()
    }
}

impl LogWriter for FileLogWriter {
    #[inline]
    fn write(&self, now: &mut DeferredNow, record: &Record) -> std::io::Result<()> {
        buffer_with(|tl_buf| match tl_buf.try_borrow_mut() {
            Ok(mut buffer) => {
                (self.config.format)(&mut *buffer, now, record)
                    .unwrap_or_else(|e| write_err(ERR_1, e));

                let mut state_guard = self.state.lock().unwrap();
                let state = state_guard.deref_mut();

                buffer
                    .write_all(state.line_ending)
                    .unwrap_or_else(|e| write_err(ERR_2, e));

                // rotate if necessary
                state
                    .mount_next_linewriter_if_necessary(&self.config)
                    .unwrap_or_else(|e| {
                        eprintln!("[flexi_logger] opening file failed with {}", e);
                    });

                state
                    .write_buffer(&*buffer)
                    .unwrap_or_else(|e| write_err(ERR_2, e));
                buffer.clear();
            }
            Err(_e) => {
                // We arrive here in the rare cases of recursive logging
                // (e.g. log calls in Debug or Display implementations)
                // we print the inner calls, in chronological order, before finally the
                // outer most message is printed
                let mut tmp_buf = Vec::<u8>::with_capacity(200);
                (self.config.format)(&mut tmp_buf, now, record)
                    .unwrap_or_else(|e| write_err(ERR_1, e));

                let mut state_guard = self.state.lock().unwrap();
                let state = state_guard.deref_mut();

                tmp_buf
                    .write_all(state.line_ending)
                    .unwrap_or_else(|e| write_err(ERR_2, e));

                state
                    .write_buffer(&tmp_buf)
                    .unwrap_or_else(|e| write_err(ERR_2, e));
            }
        });

        Ok(())
    }

    #[inline]
    fn flush(&self) -> std::io::Result<()> {
        let mut state_guard = self.state.lock().unwrap();
        if let Some(file) = state_guard.deref_mut().o_log_file.as_mut() {
            file.flush()
        } else {
            Ok(())
        }
    }

    #[inline]
    fn max_log_level(&self) -> log::LevelFilter {
        self.max_log_level
    }

    #[doc(hidden)]
    fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) {
        let mut state_guard = self.state.lock().unwrap(); // : MutexGuard<FileLogWriterState>

        let path = get_filepath(
            state_guard
                .borrow_mut()
                .o_rotation_state
                .as_ref()
                .map(|_| CURRENT_INFIX),
            &self.config.filename_config,
        );
        let f = File::open(path).unwrap();
        let mut reader = BufReader::new(f);

        let mut buf = String::new();
        for tuple in expected {
            buf.clear();
            reader.read_line(&mut buf).unwrap();
            assert!(buf.contains(&tuple.0), "Did not find tuple.0 = {}", tuple.0);
            assert!(buf.contains(&tuple.1), "Did not find tuple.1 = {}", tuple.1);
            assert!(buf.contains(&tuple.2), "Did not find tuple.2 = {}", tuple.2);
        }

        buf.clear();
        reader.read_line(&mut buf).unwrap();
        assert!(
            buf.is_empty(),
            "Found more log lines than expected: {} ",
            buf
        );
    }
}

const ERR_1: &str = "FileLogWriter: formatting failed with ";
const ERR_2: &str = "FileLogWriter: writing failed with ";

fn write_err(msg: &str, err: std::io::Error) {
    eprintln!("[flexi_logger] {} with {}", msg, err);
}

mod platform {
    use std::path::{Path, PathBuf};

    pub fn create_symlink_if_possible(link: &PathBuf, path: &Path) {
        linux_create_symlink(link, path);
    }

    #[cfg(target_os = "linux")]
    fn linux_create_symlink(link: &PathBuf, logfile: &Path) {
        if std::fs::metadata(link).is_ok() {
            // old symlink must be removed before creating a new one
            let _ = std::fs::remove_file(link);
        }

        if let Err(e) = std::os::unix::fs::symlink(&logfile, link) {
            if !e.to_string().contains("Operation not supported") {
                eprintln!(
                    "[flexi_logger] cannot create symlink {:?} for logfile \"{}\": {:?}",
                    link,
                    &logfile.display(),
                    e
                );
            }
            // no error output if e.g. writing from a linux VM to a
            // windows host's filesystem...
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn linux_create_symlink(_: &PathBuf, _: &Path) {}
}

#[cfg(test)]
mod test {
    use crate::writers::LogWriter;
    use crate::{Cleanup, Criterion, DeferredNow, Naming};
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

        // // ensure this produces 12/34/56
        write_loglines(true, naming, &ts, &[FOUR, FIVE, SIX]);
        assert!(contains("CURRENT", &ts, FIVE));
        assert!(contains("CURRENT", &ts, SIX));
        assert_eq!(list_rotated_files(&basename, &ts).len(), 2);

        // // ensure this produces 12/34/56/78/9
        // write_loglines(true, naming, &ts, &[SEVEN, EIGHT, NINE]);
        // assert_eq!(list_rotated_files(&basename, &ts).len(), 4);
        // assert!(contains("CURRENT", &ts, NINE));
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
        let arg0 = std::env::args().nth(0).unwrap();
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
        super::FileLogWriter::builder()
            .directory(DIRECTORY)
            .discriminant(discr)
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

        println!("PATTERN = {}", fn_pattern);
        glob::glob(&fn_pattern)
            .unwrap()
            .map(|r| r.unwrap().into_os_string().to_string_lossy().to_string())
            .collect()
    }
}
