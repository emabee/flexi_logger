use crate::{Cleanup, Criterion, Naming};
use std::path::PathBuf;

// Describes how rotation should work
pub(crate) struct RotationConfig {
    // Defines if rotation should be based on size or date
    pub(crate) criterion: Criterion,
    // Defines if rotated files should be numbered or get a date-based name
    pub(crate) naming: Naming,
    // Defines the cleanup strategy
    pub(crate) cleanup: Cleanup,
}
#[derive(Clone)]
pub(crate) struct FilenameConfig {
    pub(crate) directory: PathBuf,
    pub(crate) file_basename: String,
    pub(crate) suffix: String,
    pub(crate) use_timestamp: bool,
}

// The immutable configuration of a FileLogWriter.
pub(crate) struct Config {
    pub(crate) print_message: bool,
    pub(crate) append: bool,
    pub(crate) o_buffersize: Option<usize>,
    pub(crate) filename_config: FilenameConfig,
    pub(crate) o_create_symlink: Option<PathBuf>,
    pub(crate) line_ending: &'static [u8],
}
impl Config {
    // Factory method; uses the same defaults as Logger.
    pub fn default() -> Self {
        Self {
            print_message: false,
            filename_config: FilenameConfig {
                directory: PathBuf::from("."),
                file_basename: String::new(),
                suffix: "log".to_string(),
                use_timestamp: true,
            },
            append: false,
            o_buffersize: None,
            o_create_symlink: None,
            line_ending: super::UNIX_LINE_ENDING,
        }
    }
}
