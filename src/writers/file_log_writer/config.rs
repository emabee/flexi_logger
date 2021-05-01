use crate::{Cleanup, Criterion, FileSpec, Naming};
use std::path::PathBuf;

// Describes how rotation should work
#[derive(Clone)]
pub(crate) struct RotationConfig {
    // Defines if rotation should be based on size or date
    pub(crate) criterion: Criterion,
    // Defines if rotated files should be numbered or get a date-based name
    pub(crate) naming: Naming,
    // Defines the cleanup strategy
    pub(crate) cleanup: Cleanup,
}

// The immutable configuration of a FileLogWriter.
pub(crate) struct Config {
    pub(crate) print_message: bool,
    pub(crate) append: bool,
    pub(crate) o_buffersize: Option<usize>,
    pub(crate) file_spec: FileSpec,
    pub(crate) o_create_symlink: Option<PathBuf>,
}
