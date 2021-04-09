use crate::flexi_error::FlexiLoggerError;
use crate::formats::default_format;
use crate::FileSpec;
use crate::FormatFunction;
use crate::{Cleanup, Criterion, Naming};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use super::{Config, FileLogWriter, RotationConfig, State};

/// Builder for [`FileLogWriter`].
#[allow(clippy::module_name_repetitions)]
pub struct FileLogWriterBuilder {
    cfg_print_message: bool,
    cfg_append: bool,
    cfg_o_buffersize: Option<usize>,
    file_spec: FileSpec,
    cfg_o_create_symlink: Option<PathBuf>,
    cfg_line_ending: &'static [u8],
    format: FormatFunction,
    o_rotation_config: Option<RotationConfig>,
    max_log_level: log::LevelFilter,
    cleanup_in_background_thread: bool,
}

/// Methods for influencing the behavior of the [`FileLogWriter`].
impl FileLogWriterBuilder {
    pub(crate) fn new(file_spec: FileSpec) -> FileLogWriterBuilder {
        FileLogWriterBuilder {
            o_rotation_config: None,
            cfg_print_message: false,
            file_spec,
            cfg_append: false,
            cfg_o_buffersize: None,
            cfg_o_create_symlink: None,
            cfg_line_ending: super::UNIX_LINE_ENDING,
            format: default_format,
            max_log_level: log::LevelFilter::Trace,
            cleanup_in_background_thread: true,
        }
    }

    /// Makes the [`FileLogWriter`] print an info message to stdout
    /// when a new file is used for log-output.
    #[must_use]
    pub fn print_message(mut self) -> Self {
        self.cfg_print_message = true;
        self
    }

    /// Makes the [`FileLogWriter`] use the provided format function for the log entries,
    /// rather than [`default_format`].
    pub fn format(mut self, format: FormatFunction) -> Self {
        self.format = format;
        self
    }

    /// When rotation is used with some [`Cleanup`] variant, then this option defines
    /// if the cleanup activities (finding files, deleting files, evtl compressing files) is done
    /// in the current thread (in the current log-call), or whether cleanup is delegated to a
    /// background thread.
    ///
    /// As of `flexi_logger` version `0.14.7`,
    /// the cleanup activities are done by default in a background thread.
    /// This minimizes the blocking impact to your application caused by IO operations.
    ///
    /// In earlier versions of `flexi_logger`, or if you call this method with
    /// `use_background_thread = false`,
    /// the cleanup is done synchronously by the thread that is currently logging and
    /// - by chance - causing a file rotation.
    #[must_use]
    pub fn cleanup_in_background_thread(mut self, use_background_thread: bool) -> Self {
        self.cleanup_in_background_thread = use_background_thread;
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
    /// See [`Cleanup`] for details.
    #[must_use]
    pub fn rotate(mut self, criterion: Criterion, naming: Naming, cleanup: Cleanup) -> Self {
        self.o_rotation_config = Some(RotationConfig {
            criterion,
            naming,
            cleanup,
        });
        self.file_spec.if_default_use_timestamp(false);
        self
    }

    /// Set the file spec.
    #[must_use]
    pub(crate) fn file_spec(mut self, mut file_spec: FileSpec) -> Self {
        if self.o_rotation_config.is_some() {
            file_spec.if_default_use_timestamp(false);
        }
        self.file_spec = file_spec;
        self
    }

    /// Makes the logger append to the given file, if it exists; by default, the file would be
    /// truncated.
    #[must_use]
    pub fn append(mut self) -> Self {
        self.cfg_append = true;
        self
    }

    /// The specified String will be used on linux systems to create in the current folder
    /// a symbolic link to the current log file.
    pub fn create_symlink<P: Into<PathBuf>>(mut self, symlink: P) -> Self {
        self.cfg_o_create_symlink = Some(symlink.into());
        self
    }

    /// Use Windows line endings, rather than just `\n`.
    #[must_use]
    pub fn use_windows_line_ending(mut self) -> Self {
        self.cfg_line_ending = super::WINDOWS_LINE_ENDING;
        self
    }

    /// Defines if and how buffering should be used.
    ///
    /// By default, every log line is directly written to the output, without buffering.
    /// This allows seeing new log lines in real time.
    ///
    /// Using buffering reduces the program's I/O overhead, and thus increases overall performance,
    /// which can be important if logging is used heavily.
    /// On the other hand, if logging is used with low frequency,
    /// the log lines can become visible in the output with significant deferral.
    /// Furthermore, if the program is closed without flushing, some log output may get lost.
    #[must_use]
    pub fn write_mode(mut self, o_buffersize: Option<usize>) -> Self {
        self.cfg_o_buffersize = o_buffersize;
        self
    }

    #[must_use]
    pub(crate) fn buffersize(&self) -> &Option<usize> {
        &self.cfg_o_buffersize
    }

    /// Produces the `FileLogWriter`.
    ///
    /// # Errors
    ///
    /// `FlexiLoggerError::Io`.
    pub fn try_build(self) -> Result<FileLogWriter, FlexiLoggerError> {
        // make sure the folder exists or create it
        let dir = self.file_spec.get_directory();
        let p_directory = Path::new(&dir);
        std::fs::create_dir_all(&p_directory)?;
        if !std::fs::metadata(&p_directory)?.is_dir() {
            return Err(FlexiLoggerError::OutputBadDirectory);
        };

        Ok(FileLogWriter::new(
            self.format,
            self.cfg_line_ending,
            Mutex::new(State::try_new(
                Config {
                    print_message: self.cfg_print_message,
                    append: self.cfg_append,
                    o_buffersize: self.cfg_o_buffersize,
                    file_spec: self.file_spec,
                    o_create_symlink: self.cfg_o_create_symlink,
                },
                self.o_rotation_config,
                self.cleanup_in_background_thread,
            )?),
            self.max_log_level,
        ))
    }
}

/// Alternative set of methods to control the behavior of the `FileLogWriterBuilder`.
/// Use these methods when you want to control the settings flexibly,
/// e.g. with commandline arguments via `docopts` or `clap`.
impl FileLogWriterBuilder {
    /// With true, makes the `FileLogWriterBuilder` print an info message to stdout, each time
    /// when a new file is used for log-output.
    #[must_use]
    pub fn o_print_message(mut self, print_message: bool) -> Self {
        self.cfg_print_message = print_message;
        self
    }

    /// By default, and with None, the log file will grow indefinitely.
    /// If a `rotate_config` is set, when the log file reaches or exceeds the specified size,
    /// the file will be closed and a new file will be opened.
    /// Also the filename pattern changes: instead of the timestamp, a serial number
    /// is included into the filename.
    ///
    /// The size is given in bytes, e.g. `o_rotate_over_size(Some(1_000))` will rotate
    /// files once they reach a size of 1 kB.
    ///
    /// The cleanup strategy allows delimiting the used space on disk.
    #[must_use]
    pub fn o_rotate(mut self, rotate_config: Option<(Criterion, Naming, Cleanup)>) -> Self {
        if let Some((criterion, naming, cleanup)) = rotate_config {
            self.o_rotation_config = Some(RotationConfig {
                criterion,
                naming,
                cleanup,
            });
            self.file_spec.if_default_use_timestamp(false);
        } else {
            self.o_rotation_config = None;
            self.file_spec.if_default_use_timestamp(true);
        }
        self
    }

    /// If append is set to true, makes the logger append to the given file, if it exists.
    /// By default, or with false, the file would be truncated.
    #[must_use]
    pub fn o_append(mut self, append: bool) -> Self {
        self.cfg_append = append;
        self
    }

    /// If a String is specified, it will be used on linux systems to create in the current folder
    /// a symbolic link with this name to the current log file.
    pub fn o_create_symlink<S: Into<PathBuf>>(mut self, symlink: Option<S>) -> Self {
        self.cfg_o_create_symlink = symlink.map(Into::into);
        self
    }
}
