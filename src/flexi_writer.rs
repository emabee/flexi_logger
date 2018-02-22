use log_config::LogConfig;
use FlexiLoggerError;

use chrono::Local;
use glob::glob;
use std::cmp::max;
use std::env;
use std::fs::{self, File};
use std::io::{BufRead, BufReader, LineWriter, Write};
use std::ops::Add;
use std::path::Path;

type FileLineWriter = LineWriter<File>;

/// Does the physical writing
pub struct FlexiWriter {
    o_flw: Option<FileLineWriter>,
    o_filename_base: Option<String>,
    use_rotating: bool,
    written_bytes: usize,
    rotate_idx: u32,
    current_path: Option<String>,
}
impl FlexiWriter {
    pub fn new(config: &LogConfig) -> Result<FlexiWriter, FlexiLoggerError> {
        if !config.log_to_file {
            // we don't need a line-writer, so we return an empty handle
            return Ok(FlexiWriter {
                o_flw: None,
                o_filename_base: None,
                use_rotating: false,
                written_bytes: 0,
                rotate_idx: 0,
                current_path: None,
            });
        }

        // make sure the folder exists or can be created
        let s_directory: String = match config.directory {
            Some(ref dir) => dir.clone(),
            None => ".".to_string(),
        };
        let directory = Path::new(&s_directory);

        fs::create_dir_all(&directory)?;

        let o_filename_base = if fs::metadata(&directory)?.is_dir() {
            Some(get_filename_base(
                &s_directory.clone(),
                &config.discriminant,
            ))
        } else {
            return Err(FlexiLoggerError::BadDirectory);
        };

        let (use_rotating, rotate_idx) = match o_filename_base {
            None => (false, 0),
            Some(ref s_filename_base) => match config.rotate_over_size {
                None => (false, 0),
                Some(_) => (true, get_next_rotate_idx(s_filename_base, &config.suffix)),
            },
        };

        let mut flexi_writer = FlexiWriter {
            o_flw: None,
            o_filename_base: o_filename_base,
            use_rotating: use_rotating,
            written_bytes: 0,
            rotate_idx: rotate_idx,
            current_path: None,
        };
        flexi_writer.mount_linewriter(
            &config.suffix,
            &config.create_symlink,
            config.timestamp,
            config.print_message,
        );
        Ok(flexi_writer)
    }

    /// write out a log line
    pub fn write(&mut self, msgb: &[u8], config: &LogConfig) {
        // switch to next file if necessary
        if self.use_rotating && (self.written_bytes > config.rotate_over_size.unwrap()) {
            self.o_flw = None; // close the previous file
            self.written_bytes = 0;
            self.rotate_idx += 1;
            self.mount_linewriter(
                &config.suffix,
                &config.create_symlink,
                config.timestamp,
                config.print_message,
            );
        }

        // write out the message
        if let Some(ref mut lw) = self.o_flw {
            lw.write_all(msgb).unwrap_or_else(|e| {
                eprintln!("Flexi logger: write access to file failed with {}", e);
            });
            if self.use_rotating {
                self.written_bytes += msgb.len();
            }
        };
    }

    fn mount_linewriter(
        &mut self,
        suffix: &Option<String>,
        create_symlink: &Option<String>,
        timestamp: bool,
        print_message: bool,
    ) {
        if self.o_flw.is_none() {
            if let Some(ref s_filename_base) = self.o_filename_base {
                let filename = get_filename(
                    s_filename_base,
                    self.use_rotating,
                    self.rotate_idx,
                    suffix,
                    timestamp,
                );
                {
                    let path = Path::new(&filename);
                    if print_message {
                        println!("Log is written to {}", &path.display());
                    }
                    self.o_flw = Some(LineWriter::new(File::create(&path).unwrap()));
                    if let Some(ref link) = *create_symlink {
                        self::platform::create_symlink_if_possible(link, path);
                    }
                }
                self.current_path = Some(filename);
            }
        }
    }

    #[doc(hidden)]
    pub fn validate_logs(&mut self, expected: &[(&'static str, &'static str)]) -> bool {
        assert!(
            !self.current_path.is_none(),
            "validate_logs() requires std trace being directed to a file"
        );
        let path = Path::new(self.current_path.as_ref().unwrap());
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

fn get_filename_base(s_directory: &str, discriminant: &Option<String>) -> String {
    let arg0 = env::args().next().unwrap();
    let progname = Path::new(&arg0).file_stem().unwrap().to_string_lossy();
    let mut filename = String::with_capacity(180)
        .add(s_directory)
        .add("/")
        .add(&progname);
    if let Some(ref s_d) = *discriminant {
        filename = filename.add(&format!("_{}", s_d));
    }
    filename
}

fn get_filename(
    s_filename_base: &str,
    do_rotating: bool,
    rotate_idx: u32,
    o_suffix: &Option<String>,
    timestamp: bool,
) -> String {
    let mut filename = String::with_capacity(180).add(s_filename_base);
    if timestamp {
        filename = filename.add(&Local::now().format("_%Y-%m-%d_%H-%M-%S").to_string())
    };
    if do_rotating {
        filename = filename.add(&format!("_r{:0>5}", rotate_idx))
    };
    if let Some(ref suffix) = *o_suffix {
        filename = filename.add(".").add(suffix);
    }
    filename
}

fn get_filename_pattern(s_filename_base: &str, o_suffix: &Option<String>) -> String {
    let mut filename = String::with_capacity(180).add(s_filename_base);
    filename = filename.add("_r*");
    if let Some(ref suffix) = *o_suffix {
        filename = filename.add(".").add(suffix);
    }
    filename
}

fn get_next_rotate_idx(s_filename_base: &str, o_suffix: &Option<String>) -> u32 {
    let mut rotate_idx = 0;
    let fn_pattern = get_filename_pattern(s_filename_base, o_suffix);
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
