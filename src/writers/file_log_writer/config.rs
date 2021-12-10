use crate::{Cleanup, Criterion, FileSpec, Naming, WriteMode};
use std::path::PathBuf;

// Describes how rotation should work
#[derive(Clone, Debug)]
pub(crate) struct RotationConfig {
    // Defines if rotation should be based on size or date
    pub(crate) criterion: Criterion,
    // Defines if rotated files should be numbered or get a date-based name
    pub(crate) naming: Naming,
    // Defines the cleanup strategy
    pub(crate) cleanup: Cleanup,
}

// The immutable configuration of a FileLogWriter.
#[derive(Debug)]
pub(crate) struct Config {
    pub(crate) print_message: bool,
    pub(crate) append: bool,
    pub(crate) write_mode: WriteMode,
    pub(crate) file_spec: FileSpec,
    pub(crate) o_create_symlink: Option<PathBuf>,
    pub(crate) line_ending: &'static [u8],
    pub(crate) use_utc: bool,
}
