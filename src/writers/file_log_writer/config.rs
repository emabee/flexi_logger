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

/// The immutable configuration of a `FileLogWriter`
#[derive(Debug, Clone)]
pub struct Config {
    pub(crate) print_message: bool,
    pub(crate) append: bool,
    pub(crate) write_mode: WriteMode,
    pub(crate) file_spec: FileSpec,
    pub(crate) o_create_symlink: Option<PathBuf>,
    pub(crate) line_ending: &'static [u8],
    pub(crate) use_utc: bool,
}

impl Config {
    /// Returns `file_spec` configuration
    #[must_use]
    pub fn directory(&self) -> &std::path::Path {
        self.file_spec.directory.as_path()
    }

    /// Returns `basename` of log file
    #[must_use]
    pub fn basename(&self) -> &str {
        &self.file_spec.basename
    }

    /// Returns `discriminant`
    #[must_use]
    pub fn discriminant(&self) -> Option<String> {
        self.file_spec.o_discriminant.clone()
    }

    /// Returns `suffix`
    #[must_use]
    pub fn suffix(&self) -> Option<String> {
        self.file_spec.o_suffix.clone()
    }

    /// Returns `use_utc`
    #[must_use]
    pub fn use_utc(&self) -> bool {
        self.use_utc
    }

    /// Returns `append`
    #[must_use]
    pub fn append(&self) -> bool {
        self.append
    }

    /// Return `print_message`
    #[must_use]
    pub fn print_message(&self) -> bool {
        self.print_message
    }
}
