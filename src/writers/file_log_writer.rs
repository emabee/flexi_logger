use crate::formats::default_format;
use crate::logger::Cleanup;
use crate::logger::RotateOver;
use crate::writers::log_writer::LogWriter;
use crate::FlexiLoggerError;
use crate::FormatFunction;
use chrono::Local;
use log::Record;
use std::cell::RefCell;
use std::cmp::max;
use std::env;
use std::fs::{File, OpenOptions};
#[cfg(feature = "ziplogs")]
use std::io::Read;
use std::io::{BufRead, BufReader, LineWriter, Write};
use std::ops::{Add, DerefMut};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

const CURRENT_INFIX: &str = "_rCURRENT";
fn number_infix(idx: u32) -> String {
    format!("_r{:0>5}", idx)
}

// The immutable configuration of a FileLogWriter.
struct FileLogWriterConfig {
    format: FormatFunction,
    print_message: bool,
    directory: PathBuf,
    file_basename: String,
    suffix: String,
    use_timestamp: bool,
    append: bool,
    rotate_over: Option<RotateOver>,
    cleanup: Cleanup,
    create_symlink: Option<PathBuf>,
    use_windows_line_ending: bool,
}
impl FileLogWriterConfig {
    // Factory method; uses the same defaults as Logger.
    pub fn default() -> FileLogWriterConfig {
        FileLogWriterConfig {
            format: default_format,
            print_message: false,
            directory: PathBuf::from("."),
            file_basename: String::new(),
            suffix: "log".to_string(),
            use_timestamp: true,
            append: false,
            cleanup: Cleanup::Never,
            rotate_over: None,
            create_symlink: None,
            use_windows_line_ending: false,
        }
    }
}

/// Builder for `FileLogWriter`.
pub struct FileLogWriterBuilder {
    discriminant: Option<String>,
    config: FileLogWriterConfig,
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
        self.config.directory = directory.into();
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
    /// The cleanup parameter allows defining the strategy for dealing with older files.
    /// See [Cleanup](Cleanup) for details.
    pub fn rotate<R: Into<RotateOver>>(
        mut self,
        rotate_over: R,
        cleanup: Cleanup,
    ) -> FileLogWriterBuilder {
        self.config.cleanup = cleanup;
        self.config.rotate_over = Some(rotate_over.into());
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
    pub fn create_symlink<P: Into<PathBuf>>(mut self, symlink: P) -> FileLogWriterBuilder {
        self.config.create_symlink = Some(symlink.into());
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
        let p_directory = Path::new(&self.config.directory);
        std::fs::create_dir_all(&p_directory)?;
        if !std::fs::metadata(&p_directory)?.is_dir() {
            return Err(FlexiLoggerError::BadDirectory);
        };

        let arg0 = env::args().nth(0).unwrap_or_else(|| "rs".to_owned());
        self.config.file_basename =
            Path::new(&arg0).file_stem().unwrap(/*cannot fail*/).to_string_lossy().to_string();

        if let Some(discriminant) = self.discriminant {
            self.config.file_basename += &format!("_{}", discriminant);
        }
        if self.config.use_timestamp {
            self.config.file_basename += &Local::now().format("_%Y-%m-%d_%H-%M-%S").to_string();
        };  

        Ok(FileLogWriter {
            state: Mutex::new(RefCell::new(FileLogWriterState::try_new(&self.config)?)),
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
        self.config.directory = directory
            .map(Into::into)
            .unwrap_or_else(|| PathBuf::from("."));
        self
    }

    /// With true, makes the FileLogWriterBuilder include a timestamp into the names of the log files.
    pub fn o_timestamp(mut self, use_timestamp: bool) -> FileLogWriterBuilder {
        self.config.use_timestamp = use_timestamp;
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
    pub fn o_rotate<R: Into<RotateOver>>(
        mut self,
        rotate_config: Option<(R, Cleanup)>,
    ) -> FileLogWriterBuilder {
        match rotate_config {
            Some((r, c)) => {
                self.config.rotate_over = Some(r.into());
                self.config.cleanup = c;
                self.config.use_timestamp = false;
            }
            None => {
                self.config.rotate_over = None;
                self.config.cleanup = Cleanup::Never;
                self.config.use_timestamp = true;
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
        self.config.create_symlink = symlink.map(Into::into);
        self
    }
}

#[derive(Clone, Copy)]
enum Rotation {
    Start,
    Idx(u32),
}
// The mutable state of a FileLogWriter.
struct FileLogWriterState {
    line_writer: Option<LineWriter<File>>,
    created_at: std::time::SystemTime,
    written_bytes: u64,
    // Contains None if no rotation is desired,
    // or Some(rotation) if rotation is required
    //    where rotation is Rotation::Start if no rotated file exists,
    //    or Rotation::Idx(idx) where idx is the highest existing rotate_idx
    o_rotation: Option<Rotation>,
    line_ending: &'static [u8],
}
impl FileLogWriterState {
    // If rotate, the logger writes into a file with infix `_rCURRENT`.
    fn try_new(config: &FileLogWriterConfig) -> Result<FileLogWriterState, FlexiLoggerError> {
        let o_rotation = match config.rotate_over {
            None => None,
            Some(RotateOver::Size(_)) 
            // | Some(RotateOver::Duration(_)) 
            => {
                let mut rotation = get_highest_rotate_idx(&config);
                if !config.append {
                    rotation = rotate_output_file(rotation, config)?;
                }
                Some(rotation)
            }
        };

        let (line_writer, created_at, written_bytes) = get_linewriter(config)?;
        Ok(FileLogWriterState {
            line_writer: Some(line_writer),
            created_at,
            written_bytes,
            o_rotation,
            line_ending: if config.use_windows_line_ending {
                b"\r\n"
            } else {
                b"\n"
            },
        })
    }

    pub(crate) fn written_bytes(&self) -> u64 {
        self.written_bytes
    }

    fn line_writer(&mut self) -> &mut LineWriter<File> {
        self.line_writer
            .as_mut()
            .expect("FlexiLogger: line_writer unexpectedly not available")
    }

    // With rotation, the logger always writes into a file with infix `_rCURRENT`.
    // On overflow, an existing `_rCURRENT` file is renamed to the next numbered file,
    // before writing into `_rCURRENT` goes on.
    fn mount_next_linewriter(
        &mut self,
        config: &FileLogWriterConfig,
    ) -> Result<(), FlexiLoggerError> {
        self.line_writer = None; // close the output file
        self.o_rotation = Some(rotate_output_file(self.o_rotation.unwrap(), config)?);

        let (line_writer, created_at, written_bytes) = get_linewriter(config)?;
        self.line_writer = Some(line_writer);
        self.created_at = created_at;
        self.written_bytes = written_bytes;

        remove_or_zip_too_old_logfiles(&config)?;

        Ok(())
    }
}

impl Write for FileLogWriterState {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        self.line_writer().write_all(buf)?;
        if self.o_rotation.is_some() {
            self.written_bytes += buf.len() as u64;
        };
        Ok(buf.len())
    }

    #[inline]
    fn flush(&mut self) -> std::io::Result<()> {
        self.line_writer().flush()
    }
}

fn get_filepath(infix: &str, config: &FileLogWriterConfig) -> PathBuf {
    let mut s_filename =
        String::with_capacity(config.file_basename.len() + infix.len() + 1 + config.suffix.len())
            + &config.file_basename;
    if config.rotate_over.is_some() {
        s_filename += infix;
    };
    s_filename += ".";
    s_filename += &config.suffix;
    let mut p_path = config.directory.to_path_buf();
    p_path.push(s_filename);
    p_path
}

// Returns line_writer, written_bytes, path.
fn get_linewriter(
    config: &FileLogWriterConfig,
) -> Result<(LineWriter<File>, std::time::SystemTime, u64), FlexiLoggerError> {
    let p_path = get_filepath(CURRENT_INFIX, &config);
    if config.print_message {
        println!("Log is written to {}", &p_path.display());
    }
    if let Some(ref link) = config.create_symlink {
        self::platform::create_symlink_if_possible(link, &p_path);
    }

    let lw = LineWriter::new(
        OpenOptions::new()
            .write(true)
            .create(true)
            .append(config.append)
            .truncate(!config.append)
            .open(&p_path)?,
    );
    let metadata = std::fs::metadata(&p_path)?;
    let creation_st = metadata
        .created()
        .unwrap_or_else(|_| std::time::SystemTime::now());
    Ok((
        lw,
        creation_st,
        if config.append { metadata.len() } else { 0 },
    ))
}

fn get_highest_rotate_idx(config: &FileLogWriterConfig) -> Rotation {
    match list_of_log_and_zip_files(config) {
        Err(e) => {
            eprintln!("Listing files failed with {}", e);
            Rotation::Start // FIXME
        }
        Ok(globresults) => {
            let mut highest_idx = Rotation::Start;
            for globresult in globresults {
                match globresult {
                    Err(e) => eprintln!("Error when reading directory for log files: {:?}", e),
                    Ok(pathbuf) => {
                        let filename = pathbuf.file_stem().unwrap().to_string_lossy();
                        let mut it = filename.rsplit("_r");
                        let idx: u32 = it.next().unwrap().parse().unwrap_or(0);
                        highest_idx = match highest_idx {
                            Rotation::Start => Rotation::Idx(idx),
                            Rotation::Idx(prev) => Rotation::Idx(max(prev, idx)),
                        };
                    }
                }
            }
            highest_idx
        }
    }
}

fn list_of_log_and_zip_files(
    config: &FileLogWriterConfig,
) -> Result<std::iter::Chain<glob::Paths, glob::Paths>, FlexiLoggerError> {
    let fn_pattern = String::with_capacity(180)
        .add(&config.file_basename)
        .add("_r[0-9][0-9][0-9][0-9][0-9]*")
        .add(".");

    let mut log_pattern = config.directory.clone();
    log_pattern.push(fn_pattern.clone().add(&config.suffix));
    let mut zip_pattern = config.directory.clone();
    zip_pattern.push(fn_pattern.clone().add("zip"));
    Ok(glob::glob(&log_pattern.as_os_str().to_string_lossy())?
        .chain(glob::glob(&zip_pattern.as_os_str().to_string_lossy())?))
}

fn remove_or_zip_too_old_logfiles(config: &FileLogWriterConfig) -> Result<(), FlexiLoggerError> {
    let (log_limit, zip_limit) = match config.cleanup {
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
    let mut file_list: Vec<_> = list_of_log_and_zip_files(&config)?
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

// Moves the current file to the name with the next rotate_idx
// and returns the next rotate_idx.
// The current file must be closed already.
fn rotate_output_file(
    input_rotation: Rotation,
    config: &FileLogWriterConfig,
) -> Result<Rotation, FlexiLoggerError> {
    let new_rotate_idx = match input_rotation {
        Rotation::Start => 0,
        Rotation::Idx(idx) => idx + 1,
    };

    match std::fs::rename(
        get_filepath(CURRENT_INFIX, config),
        get_filepath(&number_infix(new_rotate_idx), config),
    ) {
        Ok(()) => Ok(Rotation::Idx(new_rotate_idx)),
        Err(e) => {
            if e.kind() == std::io::ErrorKind::NotFound {
                // current did not exist, so we had nothing to do
                Ok(input_rotation)
            } else {
                Err(FlexiLoggerError::Io(e))
            }
        }
    }
}

/// A configurable `LogWriter` implementation that writes to a file or a sequence of files.
///
/// See the [module description](index.html) for usage guidance.
pub struct FileLogWriter {
    config: FileLogWriterConfig,
    // the state needs to be mutable; since `Log.log()` requires an unmutable self,
    // which translates into a non-mutating `LogWriter::write()`,
    // we need the internal mutability of RefCell, and we have to wrap it with a Mutex to be
    // thread-safe
    state: Mutex<RefCell<FileLogWriterState>>,
    max_log_level: log::LevelFilter,
}
impl FileLogWriter {
    /// Instantiates a builder for `FileLogWriter`.
    pub fn builder() -> FileLogWriterBuilder {
        FileLogWriterBuilder {
            discriminant: None,
            config: FileLogWriterConfig::default(),
            max_log_level: log::LevelFilter::Trace,
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
        let path = get_filepath(CURRENT_INFIX, &self.config);
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
    fn write(&self, record: &Record) -> std::io::Result<()> {
        let mr_state = self.state.lock().unwrap(); // : MutexGuard<RefCell<FileLogWriterState>>
        let mut refmut_state = mr_state.borrow_mut(); // : RefMut<FileLogWriterState>
        let state = refmut_state.deref_mut(); // : &mut FileLogWriterState

        // switch to next file if necessary
        if let Some(ref rotate_over) = self.config.rotate_over {
            if rotate_over.rotation_necessary(state.written_bytes()
            //, state.created_at
            ) {
                state
                    .mount_next_linewriter(&self.config)
                    .unwrap_or_else(|e| {
                        eprintln!("FlexiLogger: opening file failed with {}", e);
                    });
            }
        }

        (self.config.format)(state, record)?;
        state.write_all(state.line_ending)
    }

    #[inline]
    fn flush(&self) -> std::io::Result<()> {
        let mr_state = self.state.lock().unwrap();
        let mut state = mr_state.borrow_mut();
        state.line_writer().flush()
    }

    #[inline]
    fn max_log_level(&self)  -> log::LevelFilter {
        self.max_log_level
    }
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
                    "Cannot create symlink {:?} for logfile \"{}\": {:?}",
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
    use crate::writers::log_writer::LogWriter;
    use crate::Cleanup;
    use crate::RotateOver;
    use chrono::Local;
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
    fn test_rotate_no_append() {
        // we use timestamp as discriminant to allow repeated runs
        let ts = Local::now().format("false-%Y-%m-%d_%H-%M-%S").to_string();

        // ensure we start with -/-/-
        assert!(not_exists("00000", &ts));
        assert!(not_exists("00001", &ts));
        assert!(not_exists("CURRENT", &ts));

        // ensure this produces -/-/ONE
        write_loglines(false, &ts, &[ONE]);
        assert!(not_exists("00000", &ts));
        assert!(not_exists("00001", &ts));
        assert!(contains("CURRENT", &ts, ONE));

        // ensure this produces ONE/-/TWO
        write_loglines(false, &ts, &[TWO]);
        assert!(contains("00000", &ts, ONE));
        assert!(not_exists("00001", &ts));
        assert!(contains("CURRENT", &ts, TWO));

        // ensure this also produces ONE/-/TWO
        remove("CURRENT", &ts);
        assert!(not_exists("CURRENT", &ts));
        write_loglines(false, &ts, &[TWO]);
        assert!(contains("00000", &ts, ONE));
        assert!(not_exists("00001", &ts));
        assert!(contains("CURRENT", &ts, TWO));

        // ensure this produces ONE/TWO/THREE
        write_loglines(false, &ts, &[THREE]);
        assert!(contains("00000", &ts, ONE));
        assert!(contains("00001", &ts, TWO));
        assert!(contains("CURRENT", &ts, THREE));
    }

    #[test]
    fn test_rotate_with_append() {
        // we use timestamp as discriminant to allow repeated runs
        let ts = Local::now().format("true-%Y-%m-%d_%H-%M-%S").to_string();

        // ensure we start with -/-/-
        assert!(not_exists("00000", &ts));
        assert!(not_exists("00001", &ts));
        assert!(not_exists("CURRENT", &ts));

        // ensure this produces 12/-/3
        write_loglines(true, &ts, &[ONE, TWO, THREE]);
        assert!(contains("00000", &ts, ONE));
        assert!(contains("00000", &ts, TWO));
        assert!(not_exists("00001", &ts));
        assert!(contains("CURRENT", &ts, THREE));

        // ensure this produces 12/34/56
        write_loglines(true, &ts, &[FOUR, FIVE, SIX]);
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
        write_loglines(true, &ts, &[THREE, FOUR, FIVE, SIX]);
        assert!(contains("00000", &ts, ONE));
        assert!(contains("00000", &ts, TWO));
        assert!(contains("00001", &ts, THREE));
        assert!(contains("00001", &ts, FOUR));
        assert!(contains("CURRENT", &ts, FIVE));
        assert!(contains("CURRENT", &ts, SIX));

        // ensure this produces 12/34/56/78/9
        write_loglines(true, &ts, &[SEVEN, EIGHT, NINE]);
        assert!(contains("00002", &ts, FIVE));
        assert!(contains("00002", &ts, SIX));
        assert!(contains("00003", &ts, SEVEN));
        assert!(contains("00003", &ts, EIGHT));
        assert!(contains("CURRENT", &ts, NINE));
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

    fn write_loglines(append: bool, discr: &str, texts: &[&'static str]) {
        let flw = get_file_log_writer(append, discr);
        for text in texts {
            flw.write(
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

    fn get_file_log_writer(append: bool, discr: &str) -> crate::writers::FileLogWriter {
        super::FileLogWriter::builder()
            .directory(DIRECTORY)
            .discriminant(discr)
            .rotate(
                RotateOver::Size(if append { 28 } else { 10 }),
                Cleanup::Never,
            )
            .o_append(append)
            .try_build()
            .unwrap()
    }
}
