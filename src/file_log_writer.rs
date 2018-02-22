use std::sync::Mutex;
use std::cell::RefCell;
use log_writer::LogWriter;
use log::Record;
use FlexiLoggerError;

use chrono::Local;
use glob::glob;
use std::cmp::max;
use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, LineWriter, Write};
use std::ops::Add;
use std::path::Path;

// The immutable config of a FileLogWriter.
pub struct FileLogWriterConfig {
    pub format: fn(&Record) -> String,
    print_message: bool,
    filename_base: String,
    suffix: String,
    use_timestamp: bool,
    rotate_over_size: Option<usize>,
    create_symlink: Option<String>,
}
impl FileLogWriterConfig {
    pub fn new(
        format: fn(&Record) -> String,
        print_message: bool,
        directory: Option<String>,
        discriminant: Option<String>,
        suffix: String,
        use_timestamp: bool,
        rotate_over_size: Option<usize>,
        create_symlink: Option<String>,
    ) -> Result<FileLogWriterConfig, FlexiLoggerError> {
        // make sure the folder exists or create it
        let s_directory: String = directory.unwrap_or(".".to_string());
        let p_directory = Path::new(&s_directory);
        fs::create_dir_all(&p_directory)?;
        if !fs::metadata(&p_directory)?.is_dir() {
            return Err(FlexiLoggerError::BadDirectory);
        };

        Ok(FileLogWriterConfig {
            format: format,
            print_message: print_message,
            filename_base: FileLogWriterConfig::get_filename_base(&s_directory, discriminant),
            suffix: suffix,
            use_timestamp: use_timestamp,
            rotate_over_size: rotate_over_size,
            create_symlink: create_symlink,
        })
    }

    fn get_filename_base(dir: &str, o_discriminant: Option<String>) -> String {
        let arg0 = env::args().next().unwrap();
        let progname = Path::new(&arg0).file_stem().unwrap().to_string_lossy();
        let mut filename = String::with_capacity(180).add(dir).add("/").add(&progname);
        if let Some(discriminant) = o_discriminant {
            filename = filename.add(&format!("_{}", discriminant));
        }
        filename
    }

    fn get_filename(&self, rotate_idx: u32) -> String {
        let mut filename = String::with_capacity(180).add(&self.filename_base);
        if self.use_timestamp {
            filename = filename.add(&Local::now().format("_%Y-%m-%d_%H-%M-%S").to_string())
        };
        if self.rotate_over_size.is_some() {
            filename = filename.add(&format!("_r{:0>5}", rotate_idx))
        };
        filename.add(".").add(&self.suffix)
    }
}

// The mutable state of a FileLogWriter.
struct FileLogWriterState {
    lw: LineWriter<File>,
    written_bytes: usize,
    rotate_idx: u32,
    current_path: String,
}
impl FileLogWriterState {
    fn new(config: &FileLogWriterConfig) -> FileLogWriterState {
        let rotate_idx = match config.rotate_over_size {
            None => 0,
            Some(_) => get_next_rotate_idx(&config.filename_base, &config.suffix),
        };

        let (lw, cp) = get_linewriter(rotate_idx, config);
        FileLogWriterState {
            lw: lw,
            current_path: cp,
            written_bytes: 0,
            rotate_idx: rotate_idx,
        }
    }

    fn mount_linewriter(&mut self, config: &FileLogWriterConfig) {
        let (lw, cp) = get_linewriter(self.rotate_idx, config);
        self.lw = lw;
        self.current_path = cp;
    }
}

/// A LogWriter that writes to a file or, if rotation is used, a sequence of files.
pub struct FileLogWriter {
    config: FileLogWriterConfig,
    state: Mutex<RefCell<FileLogWriterState>>,
}
impl FileLogWriter {
    pub fn new(config: FileLogWriterConfig) -> Result<FileLogWriter, FlexiLoggerError> {
        Ok(FileLogWriter {
            state: Mutex::new(RefCell::new(FileLogWriterState::new(&config))),
            config: config,
        })
    }

    pub fn config(&self) -> &FileLogWriterConfig {
        &(self.config)
    }

    #[doc(hidden)]
    pub fn validate_logs(&mut self, expected: &[(&'static str, &'static str)]) -> bool {
        let guard = self.state.lock().unwrap();
        let state = guard.borrow();
        let path = Path::new(&state.current_path);
        let f = File::open(path).unwrap();
        let mut reader = BufReader::new(f);

        let mut line = String::new();
        for tuple in expected {
            line.clear();
            reader.read_line(&mut line).unwrap();
            assert!(line.contains(&tuple.0));
            assert!(line.contains(&tuple.1));
        }
        false
    }
}

impl LogWriter for FileLogWriter {
    fn write(&self, record: &Record) {
        let guard = self.state.lock().unwrap();
        let mut state = guard.borrow_mut();
        // switch to next file if necessary
        if self.config.rotate_over_size.is_some()
            && (state.written_bytes > self.config.rotate_over_size.unwrap())
        {
            state.written_bytes = 0;
            state.rotate_idx += 1;
            state.mount_linewriter(&self.config);
        }

        let msg = (self.config.format)(record);
        let msgb = msg.as_bytes();
        // write out the message
        state.lw.write_all(msgb).unwrap_or_else(|e| {
            eprintln!("Flexi logger: write access to file failed with {}", e);
        });
        if self.config.rotate_over_size.is_some() {
            state.written_bytes += msgb.len();
        }
    }
}

fn get_next_rotate_idx(filename_base: &str, suffix: &str) -> u32 {
    let mut rotate_idx = 0;
    let fn_pattern = String::with_capacity(180)
        .add(filename_base)
        .add("_r*")
        .add(".")
        .add(suffix);
    match glob(&fn_pattern) {
        Err(e) => {
            eprintln!(
                "Is this ({}) really a directory? Listing failed with {}",
                fn_pattern, e
            );
        }
        Ok(globresults) => for globresult in globresults {
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
        },
    }
    rotate_idx + 1
}

fn get_linewriter(rotate_idx: u32, config: &FileLogWriterConfig) -> (LineWriter<File>, String) {
    let filename = config.get_filename(rotate_idx);
    let lw = {
        let path = Path::new(&filename);
        if config.print_message {
            println!("Log is written to {}", &path.display());
        }
        if let Some(ref link) = config.create_symlink {
            self::platform::create_symlink_if_possible(link, path);
        }
        LineWriter::new(File::create(&path).unwrap())
    };
    (lw, filename)
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
