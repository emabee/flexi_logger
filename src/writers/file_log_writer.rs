use formats::default_format;
use log::Record;
use writers::log_writer::LogWriter;
use FlexiLoggerError;
use FormatFunction;

use chrono::Local;
use glob::glob;
use std::cell::RefCell;
use std::cmp::max;
use std::env;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, LineWriter, Write};
use std::ops::{Add, DerefMut};
use std::path::Path;
use std::sync::Mutex;

const CURRENT_INFIX: &str = "_rCURRENT";
fn number_infix(idx: u32) -> String {
    format!("_r{:0>5}", idx)
}

// The immutable configuration of a FileLogWriter.
struct FileLogWriterConfig {
    format: FormatFunction,
    print_message: bool,
    path_base: String,
    suffix: String,
    use_timestamp: bool,
    append: bool,
    rotate_over_size: Option<u64>,
    create_symlink: Option<String>,
}
impl FileLogWriterConfig {
    // Factory method; uses the same defaults as Logger.
    pub fn default() -> FileLogWriterConfig {
        FileLogWriterConfig {
            format: default_format,
            print_message: false,
            path_base: String::new(),
            suffix: "log".to_string(),
            use_timestamp: true,
            append: false,
            rotate_over_size: None,
            create_symlink: None,
        }
    }
}

/// Builder for `FileLogWriter`.
pub struct FileLogWriterBuilder {
    directory: Option<String>,
    discriminant: Option<String>,
    config: FileLogWriterConfig,
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
    pub fn directory<S: Into<String>>(mut self, directory: S) -> FileLogWriterBuilder {
        self.directory = Some(directory.into());
        self
    }

    /// Specifies a suffix for the log files. The default is "log".
    pub fn suffix<S: Into<String>>(mut self, suffix: S) -> FileLogWriterBuilder {
        self.config.suffix = suffix.into();
        self
    }

    /// Makes the logger not include a timestamp into the names of the log files
    pub fn suppress_timestamp(mut self) -> FileLogWriterBuilder {
        self.config.use_timestamp = false;
        self
    }

    /// Prevents indefinite growth of log files.
    ///
    /// By default, the log file is fixed while your program is running and will grow indefinitely.
    /// With this option being used, when the log file reaches or exceeds the specified file size,
    /// the file will be closed and a new file will be opened.
    ///
    /// The rotate-over-size is given in bytes, e.g. `rotate_over_size(1_000)` will rotate
    /// files once they reach a size of 1000 bytes.
    ///     
    /// Note that also the filename pattern changes:
    ///
    /// - by default, no timestamp is added to the filename
    /// - the logs are always written to a file with infix `_rCURRENT`
    /// - if this file exceeds the specified rotate-over-size, it is closed and renamed to a file
    ///   with a sequential number infix,
    ///   and then the logging continues again to the (fresh) file with infix `_rCURRENT`
    ///
    /// Example:
    ///
    /// After some logging with your program `my_prog`, you will find files like
    ///
    /// ```text
    /// my_prog_r00000.log
    /// my_prog_r00001.log
    /// my_prog_r00002.log
    /// my_prog_rCURRENT.log
    /// ```
    ///
    pub fn rotate_over_size(mut self, rotate_over_size: usize) -> FileLogWriterBuilder {
        self.config.rotate_over_size = Some(rotate_over_size as u64);
        self.config.use_timestamp = false;
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
    pub fn create_symlink<S: Into<String>>(mut self, symlink: S) -> FileLogWriterBuilder {
        self.config.create_symlink = Some(symlink.into());
        self
    }

    /// Produces the FileLogWriter.
    pub fn instantiate(mut self) -> Result<FileLogWriter, FlexiLoggerError> {
        // make sure the folder exists or create it
        let s_directory: String = self.directory.unwrap_or_else(|| ".".to_string());
        let p_directory = Path::new(&s_directory);
        fs::create_dir_all(&p_directory)?;
        if !fs::metadata(&p_directory)?.is_dir() {
            return Err(FlexiLoggerError::BadDirectory);
        };

        let arg0 = env::args().nth(0).unwrap_or_else(|| "rs".to_owned());
        let progname = Path::new(&arg0).file_stem().unwrap(/*cannot fail*/).to_string_lossy();

        self.config.path_base.clear();
        self.config.path_base.reserve(180);
        self.config.path_base += &s_directory;
        self.config.path_base += "/";
        self.config.path_base += &progname;

        if let Some(discriminant) = self.discriminant {
            self.config.path_base += &format!("_{}", discriminant);
        }
        if self.config.use_timestamp {
            self.config.path_base += &Local::now().format("_%Y-%m-%d_%H-%M-%S").to_string();
        };

        Ok(FileLogWriter {
            state: Mutex::new(RefCell::new(FileLogWriterState::new(&self.config)?)),
            config: self.config,
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
    pub fn o_directory<S: Into<String>>(mut self, directory: Option<S>) -> FileLogWriterBuilder {
        self.directory = directory.map(|d| d.into());
        self
    }

    /// With true, makes the FileLogWriterBuilder include a timestamp into the names of the log files.
    pub fn o_timestamp(mut self, use_timestamp: bool) -> FileLogWriterBuilder {
        self.config.use_timestamp = use_timestamp;
        self
    }

    /// By default, and with None, the log file will grow indefinitely.
    /// If a size is set, when the log file reaches or exceeds the specified size,
    /// the file will be closed and a new file will be opened.
    /// Also the filename pattern changes: instead of the timestamp, a serial number
    /// is included into the filename.
    ///
    /// The size is given in bytes, e.g. `o_rotate_over_size(Some(1_000))` will rotate
    /// files once they reach a size of 1 kB.
    pub fn o_rotate_over_size(mut self, rotate_over_size: Option<usize>) -> FileLogWriterBuilder {
        self.config.rotate_over_size = rotate_over_size.map(|r| r as u64);
        self.config.use_timestamp = false;
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
        self.discriminant = discriminant.map(|d| d.into());
        self
    }

    /// If a String is specified, it will be used on linux systems to create in the current folder
    /// a symbolic link with this name to the current log file.
    pub fn o_create_symlink<S: Into<String>>(mut self, symlink: Option<S>) -> FileLogWriterBuilder {
        self.config.create_symlink = symlink.map(|s| s.into());
        self
    }
}

// The mutable state of a FileLogWriter.
struct FileLogWriterState {
    line_writer: Option<LineWriter<File>>,
    written_bytes: u64,
    // None if no rotation is desired, or else Some(idx) where idx is the highest existing rotate_idx
    rotate_idx: Option<u32>,
    path: String,
}
impl FileLogWriterState {
    // If rotate, the logger writes into a file with infix `_rCURRENT`.
    fn new(config: &FileLogWriterConfig) -> Result<FileLogWriterState, FlexiLoggerError> {
        let rotate_idx = match config.rotate_over_size {
            None => None,
            Some(_) => Some({
                let mut rotate_idx = get_highest_rotate_idx(&config.path_base, &config.suffix);
                if !config.append {
                    rotate_idx = rotate_output_file(rotate_idx, config)?;
                }
                rotate_idx
            }),
        };

        let (line_writer, written_bytes, path) = get_linewriter(config)?;
        Ok(FileLogWriterState {
            line_writer: Some(line_writer),
            path,
            written_bytes,
            rotate_idx,
        })
    }

    fn line_writer(&mut self) -> &mut LineWriter<File> {
        self.line_writer
            .as_mut()
            .expect("FlexiLogger: line_writer unexpectedly not available")
    }

    // The logger should always write into a file with infix `_rCURRENT`.
    // On overflow, an existing `_rCURRENT` file must be renamed to the next numbered file,
    // before writing into `_rCURRENT` goes on.
    fn mount_next_linewriter(
        &mut self,
        config: &FileLogWriterConfig,
    ) -> Result<(), FlexiLoggerError> {
        self.line_writer = None; // close the output file
        self.rotate_idx = Some(rotate_output_file(self.rotate_idx.take().unwrap(), config)?);
        self.written_bytes = 0;
        let (line_writer, written_bytes, path) = get_linewriter(config)?;
        self.line_writer = Some(line_writer);
        self.written_bytes = written_bytes;
        self.path = path;
        Ok(())
    }
}

impl Write for FileLogWriterState {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.line_writer().write_all(buf)?;
        if self.rotate_idx.is_some() {
            self.written_bytes += buf.len() as u64;
        };
        Ok(buf.len())
    }

    #[inline]
    fn flush(&mut self) -> io::Result<()> {
        self.line_writer().flush()
    }
}

fn get_infix_path(infix: &str, config: &FileLogWriterConfig) -> String {
    let mut s_path = String::with_capacity(180) + &config.path_base;
    if config.rotate_over_size.is_some() {
        s_path += infix;
    };
    s_path += ".";
    s_path + &config.suffix
}

// Returns line_writer, written_bytes, path.
fn get_linewriter(
    config: &FileLogWriterConfig,
) -> Result<(LineWriter<File>, u64, String), FlexiLoggerError> {
    let s_path = get_infix_path(CURRENT_INFIX, &config);

    let (line_writer, file_size) = {
        let p_path = Path::new(&s_path);
        if config.print_message {
            println!("Log is written to {}", &p_path.display());
        }
        if let Some(ref link) = config.create_symlink {
            self::platform::create_symlink_if_possible(link, p_path);
        }

        (
            LineWriter::new(
                OpenOptions::new()
                    .write(true)
                    .create(true)
                    .append(config.append)
                    .truncate(!config.append)
                    .open(&p_path)?,
            ),
            if config.append {
                let metadata = fs::metadata(&s_path)?;
                metadata.len()
            } else {
                0
            },
        )
    };
    Ok((line_writer, file_size, s_path))
}

fn get_highest_rotate_idx(path_base: &str, suffix: &str) -> u32 {
    let mut rotate_idx = 0;
    let fn_pattern = String::with_capacity(180)
        .add(path_base)
        .add("_r*")
        .add(".")
        .add(suffix);
    match glob(&fn_pattern) {
        Err(e) => {
            eprintln!("Listing files with ({}) failed with {}", fn_pattern, e);
        }
        Ok(globresults) => {
            for globresult in globresults {
                match globresult {
                    Err(e) => eprintln!(
                        "Error occured when reading directory for log files: {:?}",
                        e
                    ),
                    Ok(pathbuf) => {
                        let filename = pathbuf.file_stem().unwrap().to_string_lossy();
                        let mut it = filename.rsplit("_r");
                        let idx: u32 = it.next().unwrap().parse().unwrap_or(0);
                        rotate_idx = max(rotate_idx, idx);
                    }
                }
            }
        }
    }
    rotate_idx
}

fn rotate_output_file(
    rotate_idx: u32,
    config: &FileLogWriterConfig,
) -> Result<u32, FlexiLoggerError> {
    // current-file must be closed already
    // move it to the name with the next rotate_idx
    match std::fs::rename(
        get_infix_path(CURRENT_INFIX, config),
        get_infix_path(&number_infix(rotate_idx), config),
    ) {
        Ok(()) => Ok(rotate_idx + 1),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                Ok(rotate_idx)
            } else {
                Err(FlexiLoggerError::Io(e))
            }
        }
    }
}

/// A configurable `LogWriter` that writes to a file or, if rotation is used, a sequence of files.
pub struct FileLogWriter {
    config: FileLogWriterConfig,
    // the state needs to be mutable; since `Log.log()` requires an unmutable self,
    // which translates into a non-mutating `LogWriter::write()`,
    // we need the internal mutability of RefCell, and we have to wrap it with a Mutex to be
    // thread-safe
    state: Mutex<RefCell<FileLogWriterState>>,
}
impl FileLogWriter {
    /// Instantiates a builder for `FileLogWriter`.
    pub fn builder() -> FileLogWriterBuilder {
        FileLogWriterBuilder {
            directory: None,
            discriminant: None,
            config: FileLogWriterConfig::default(),
        }
    }

    /// Returns a reference to its configured output format function.
    #[inline]
    pub fn format(&self) -> FormatFunction {
        self.config.format
    }

    // don't use this function in productive code - it exists only for flexi_loggers own tests
    #[doc(hidden)]
    pub fn validate_logs(&self, expected: &[(&'static str, &'static str, &'static str)]) -> bool {
        let guard = self.state.lock().unwrap();
        let state = guard.borrow();
        let path = Path::new(&state.path);
        let f = File::open(path).unwrap();
        let mut reader = BufReader::new(f);

        let mut line = String::new();
        for tuple in expected {
            line.clear();
            reader.read_line(&mut line).unwrap();
            assert!(
                line.contains(&tuple.0),
                "Did not find tuple.0 = {}",
                tuple.0
            );
            assert!(
                line.contains(&tuple.1),
                "Did not find tuple.1 = {}",
                tuple.1
            );
            assert!(
                line.contains(&tuple.2),
                "Did not find tuple.2 = {}",
                tuple.2
            );
        }
        false
    }
}

impl LogWriter for FileLogWriter {
    #[inline]
    fn write(&self, record: &Record) -> io::Result<()> {
        let guard = self.state.lock().unwrap(); // : MutexGuard<RefCell<FileLogWriterState>>
        let mut state = guard.borrow_mut(); // : RefMut<FileLogWriterState>
        let state = state.deref_mut(); // : &mut FileLogWriterState

        // switch to next file if necessary
        if let Some(rotate_over_size) = self.config.rotate_over_size {
            if state.written_bytes > rotate_over_size {
                state
                    .mount_next_linewriter(&self.config)
                    .unwrap_or_else(|e| {
                        eprintln!("FlexiLogger: opening file failed with {}", e);
                    });
            }
        }

        (self.config.format)(state, record)?;
        state.write_all(b"\n")
    }

    #[inline]
    fn flush(&self) -> io::Result<()> {
        let guard = self.state.lock().unwrap();
        let mut state = guard.borrow_mut();
        state.line_writer().flush()
    }
}

mod platform {
    use std::path::Path;

    pub fn create_symlink_if_possible(link: &str, path: &Path) {
        linux_create_symlink(link, path);
    }

    #[cfg(target_os = "linux")]
    fn linux_create_symlink(link: &str, path: &Path) {
        use std::fs;
        use std::os::unix::fs as unix_fs;

        if fs::metadata(link).is_ok() {
            // old symlink must be removed before creating a new one
            let _ = fs::remove_file(link);
        }

        if let Err(e) = unix_fs::symlink(&path, link) {
            eprintln!(
                "Can not create symlink \"{}\" for path \"{}\": {}",
                link,
                &path.display(),
                e
            );
        }
    }

    #[cfg(not(target_os = "linux"))]
    fn linux_create_symlink(_: &str, _: &Path) {}
}
