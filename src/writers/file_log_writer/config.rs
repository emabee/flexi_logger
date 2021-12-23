use crate::{Cleanup, Criterion, FileSpec, Naming, WriteMode};
use std::path::PathBuf;

/// Describes how rotation should work
#[derive(Clone, Debug)]
pub struct RotationConfig {
    // Defines if rotation should be based on size or date
    pub(crate) criterion: Criterion,
    // Defines if rotated files should be numbered or get a date-based name
    pub(crate) naming: Naming,
    // Defines the cleanup strategy
    pub(crate) cleanup: Cleanup,
}

/// Configuration of a `FileLogWriter`.
#[derive(Debug, Clone)]
pub struct FileLogWriterConfig {
    pub(crate) print_message: bool,
    pub(crate) append: bool,
    pub(crate) write_mode: WriteMode,
    pub(crate) file_spec: FileSpec,
    pub(crate) o_create_symlink: Option<PathBuf>,
    pub(crate) line_ending: &'static [u8],
    pub(crate) use_utc: bool,
}

impl FileLogWriterConfig {
    /// Returns the configured directory.
    #[must_use]
    pub fn directory(&self) -> &std::path::Path {
        self.file_spec.directory.as_path()
    }

    /// Returns the configured `basename` of the log file.
    #[must_use]
    pub fn basename(&self) -> &str {
        &self.file_spec.basename
    }

    /// Returns the configured `discriminant`.
    #[must_use]
    pub fn discriminant(&self) -> Option<String> {
        self.file_spec.o_discriminant.clone()
    }

    /// Returns the configured `suffix`.
    #[must_use]
    pub fn suffix(&self) -> Option<String> {
        self.file_spec.o_suffix.clone()
    }

    /// Returns `true` if UTC is enforced.
    #[must_use]
    pub fn use_utc(&self) -> bool {
        self.use_utc
    }

    /// Returns `true` if existing files are appended on program start.
    #[must_use]
    pub fn append(&self) -> bool {
        self.append
    }

    /// Returns `true` if a message should be printed on program start
    /// to which file the log is written.
    #[must_use]
    pub fn print_message(&self) -> bool {
        self.print_message
    }
}
