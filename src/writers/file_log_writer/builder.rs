use crate::flexi_error::FlexiLoggerError;
use crate::formats::default_format;
use crate::{Cleanup, Criterion, FileSpec, FormatFunction, Naming, WriteMode};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use super::{FileLogWriter, FileLogWriterConfig, LogWriter, RotationConfig, State};

/// Builder for [`FileLogWriter`].
#[allow(clippy::struct_excessive_bools, clippy::module_name_repetitions)]
pub struct FileLogWriterBuilder {
    cfg_print_message: bool,
    cfg_append: bool,
    cfg_write_mode: WriteMode,
    file_spec: FileSpec,
    cfg_o_create_symlink: Option<PathBuf>,
    cfg_line_ending: &'static [u8],
    format: FormatFunction,
    o_rotation_config: Option<RotationConfig>,
    max_log_level: log::LevelFilter,
    cleanup_in_background_thread: bool,
    use_utc: bool,
}

/// Methods for influencing the behavior of the [`FileLogWriter`].
impl FileLogWriterBuilder {
    pub(crate) fn new(file_spec: FileSpec) -> Self {
        Self {
            o_rotation_config: None,
            cfg_print_message: false,
            file_spec,
            cfg_append: false,
            cfg_write_mode: WriteMode::Direct,
            cfg_o_create_symlink: None,
            cfg_line_ending: super::UNIX_LINE_ENDING,
            format: default_format,
            max_log_level: log::LevelFilter::Trace,
            cleanup_in_background_thread: true,
            use_utc: false,
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
    #[must_use]
    pub fn format(mut self, format: FormatFunction) -> Self {
        self.format = format;
        self
    }

    /// Influences how the cleanup activities
    /// (finding files, deleting files, optionally compressing files) are done
    /// when rotation is used with some [`Cleanup`] variant.
    ///
    /// With the synchronous [write modes](crate::WriteMode),
    /// the cleanup activities are done by default in a dedicated background thread, to
    /// minimize the blocking impact on your application.
    /// You can avoid this extra thread by calling this method with
    /// `use_background_thread = false`; the cleanup is then done synchronously
    /// by the thread that is currently logging and - by chance - causing a file rotation.
    ///
    /// With [`WriteMode::AsyncWith`](crate::WriteMode::AsyncWith),
    /// the cleanup activities are always done by the same background thread
    /// that also does the file I/O, this method then has no effect.
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

    /// Set the maximum log level.
    ///
    /// The default is `log::LevelFilter::Trace`, i.e., all log levels are written.
    #[must_use]
    pub fn max_level(mut self, max_log_level: log::LevelFilter) -> Self {
        self.max_log_level = max_log_level;
        self
    }

    /// Enforces the use of UTC, rather than local time.
    #[must_use]
    pub fn use_utc(mut self) -> Self {
        self.file_spec.use_utc = true;
        self.use_utc = true;
        self
    }

    /// The specified String will be used on unix systems to create in the current folder
    /// a symbolic link to the current log file.
    #[must_use]
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

    /// Sets the write mode for the `FileLogWriter`.
    ///
    /// See [`WriteMode`] for more (important!) details.
    #[must_use]
    pub fn write_mode(mut self, write_mode: WriteMode) -> Self {
        self.cfg_write_mode = write_mode;
        self
    }

    pub(crate) fn assert_write_mode(&self, write_mode: WriteMode) -> Result<(), FlexiLoggerError> {
        if self.cfg_write_mode == write_mode {
            Ok(())
        } else {
            Err(FlexiLoggerError::Reset)
        }
    }

    #[must_use]
    pub(crate) fn get_write_mode(&self) -> &WriteMode {
        &self.cfg_write_mode
    }

    /// Produces the `FileLogWriter`.
    ///
    /// # Errors
    ///
    /// `FlexiLoggerError::Io` if the specified path doesn't work.
    pub fn try_build(self) -> Result<FileLogWriter, FlexiLoggerError> {
        Ok(FileLogWriter::new(
            self.try_build_state()?,
            self.max_log_level,
            self.format,
        ))
    }

    /// Produces the `FileLogWriter` and a handle that is connected with it.
    ///
    /// This allows handing out the `FileLogWriter` instance to methods that consume it, and still
    /// be able to influence it via the handle.
    ///
    /// # Errors
    ///
    /// `FlexiLoggerError::Io` if the specified path doesn't work.
    pub fn try_build_with_handle(
        self,
    ) -> Result<(ArcFileLogWriter, FileLogWriterHandle), FlexiLoggerError> {
        Ok(ArcFileLogWriter::new_with_handle(FileLogWriter::new(
            self.try_build_state()?,
            self.max_log_level,
            self.format,
        )))
    }

    pub(super) fn try_build_state(&self) -> Result<State, FlexiLoggerError> {
        // make sure the folder exists or create it
        let dir = self.file_spec.get_directory();
        let p_directory = Path::new(&dir);
        std::fs::create_dir_all(p_directory)?;
        if !std::fs::metadata(p_directory)?.is_dir() {
            return Err(FlexiLoggerError::OutputBadDirectory);
        };

        #[cfg(feature = "async")]
        let cleanup_in_background_thread = if let WriteMode::AsyncWith {
            pool_capa: _,
            message_capa: _,
            flush_interval: _,
        } = self.cfg_write_mode
        {
            false
        } else {
            self.cleanup_in_background_thread
        };
        #[cfg(not(feature = "async"))]
        let cleanup_in_background_thread = self.cleanup_in_background_thread;

        Ok(State::new(
            FileLogWriterConfig {
                print_message: self.cfg_print_message,
                append: self.cfg_append,
                line_ending: self.cfg_line_ending,
                write_mode: self.cfg_write_mode,
                file_spec: self.file_spec.clone(),
                o_create_symlink: self.cfg_o_create_symlink.clone(),
                use_utc: self.use_utc,
            },
            self.o_rotation_config.clone(),
            cleanup_in_background_thread,
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

    /// If a String is specified, it will be used on unix systems to create in the current folder
    /// a symbolic link with this name to the current log file.
    #[must_use]
    pub fn o_create_symlink<S: Into<PathBuf>>(mut self, symlink: Option<S>) -> Self {
        self.cfg_o_create_symlink = symlink.map(Into::into);
        self
    }
}

/// A shareable `FileLogWriter` with a handle.
pub struct ArcFileLogWriter(Arc<FileLogWriter>);
impl ArcFileLogWriter {
    pub(crate) fn new_with_handle(flw: FileLogWriter) -> (Self, FileLogWriterHandle) {
        let a_flw = Arc::new(flw);
        (Self(Arc::clone(&a_flw)), FileLogWriterHandle(a_flw))
    }
}
impl Clone for ArcFileLogWriter {
    fn clone(&self) -> Self {
        Self(Arc::clone(&self.0))
    }
}
impl std::io::Write for ArcFileLogWriter {
    fn write(&mut self, buffer: &[u8]) -> std::result::Result<usize, std::io::Error> {
        (*self.0).plain_write(buffer)
    }
    fn flush(&mut self) -> std::result::Result<(), std::io::Error> {
        LogWriter::flush(&*self.0)
    }
}

/// Handle to the `FileLogWriter` in an `ArcFileLogWriter`
/// that shuts down the `FileLogWriter` in its `Drop` implementation.
pub struct FileLogWriterHandle(Arc<FileLogWriter>);
impl Drop for FileLogWriterHandle {
    fn drop(&mut self) {
        self.0.shutdown();
    }
}
