use {LogConfig, FlexiLoggerError};

use chrono::Local;
use glob::glob;
use std::cmp::max;
use std::env;
use std::fs::{self, File};
use std::io::{LineWriter, Write};
use std::ops::Add;
use std::path::Path;

type FileLineWriter = LineWriter<File>;

/// Does the physical writing
pub struct FlexiWriter {
    o_flw: Option<FileLineWriter>,
    o_filename_base: Option<String>,
    use_rotating: bool,
    written_bytes: usize,
    rotate_idx: usize,
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
            });
        }

        // make sure the folder exists or can be created
        let s_directory: String = match config.directory {
            Some(ref dir) => dir.clone(),
            None => ".".to_string(),
        };
        let directory = Path::new(&s_directory);

        if let Err(e) = fs::create_dir_all(&directory) {
            return Err(FlexiLoggerError::new(format!("Log cannot be written: output directory \
                                                      \"{}\" does not exist and could not be \
                                                      created due to {}",
                                                     &directory.display(),
                                                     e)));
        };

        let o_filename_base = match fs::metadata(&directory) {
            Ok(metadata) => {
                if metadata.is_dir() {
                    Some(get_filename_base(&s_directory.clone(), &config.discriminant))
                } else {
                    return Err(FlexiLoggerError::new(format!("Log cannot be written: output \
                                                              directory \"{}\" is not a \
                                                              directory",
                                                             &directory.display())));
                }
            }
            Err(e) => {
                return Err(FlexiLoggerError::new(format!("Log cannot be written: error \
                                                          accessing output directory \"{}\": {}",
                                                         &directory.display(),
                                                         e)));
            }
        };

        let (use_rotating, rotate_idx) = match o_filename_base {
            None => (false, 0),
            Some(ref s_filename_base) => {
                match config.rotate_over_size {
                    None => (false, 0),
                    Some(_) => (true, get_next_rotate_idx(&s_filename_base, &config.suffix)),
                }
            }
        };

        let mut flexi_writer = FlexiWriter {
            o_flw: None,
            o_filename_base: o_filename_base,
            use_rotating: use_rotating,
            written_bytes: 0,
            rotate_idx: rotate_idx,
        };
        flexi_writer.mount_linewriter(&config.suffix, &config.create_symlink, config.timestamp, config.print_message);
        Ok(flexi_writer)
    }

    /// write out a log line
    pub fn write(&mut self, msgb: &[u8], config: &LogConfig) {
        // switch to next file if necessary
        if self.use_rotating && (self.written_bytes > config.rotate_over_size.unwrap()) {
            self.o_flw = None;  // close the previous file
            self.written_bytes = 0;
            self.rotate_idx += 1;
            self.mount_linewriter(&config.suffix, &config.create_symlink, config.timestamp, config.print_message);
        }

        // write out the stuff
        if let Some(ref mut lw) = self.o_flw {
            lw.write(msgb)
              .unwrap_or_else(|e| {
                  print_err!("Flexi logger: write access to file failed with {}", e);
                  0
              });
            if self.use_rotating {
                self.written_bytes += msgb.len();
            }
        };
    }

    fn mount_linewriter(&mut self, suffix: &Option<String>, create_symlink: &Option<String>, timestamp: bool,
                        print_message: bool) {
        if let None = self.o_flw {
            if let Some(ref s_filename_base) = self.o_filename_base {
                let filename = get_filename(s_filename_base, self.use_rotating, self.rotate_idx, suffix, timestamp);
                let path = Path::new(&filename);
                if print_message {
                    println!("Log is written to {}", &path.display());
                }
                self.o_flw = Some(LineWriter::new(File::create(&path).unwrap()));

                if let &Some(ref link) = create_symlink {
                    self::platform::create_symlink_if_possible(link);
                }
            }
        }
    }
}

fn get_filename_base(s_directory: &String, discriminant: &Option<String>) -> String {
    let arg0 = env::args().next().unwrap();
    let progname = Path::new(&arg0).file_stem().unwrap().to_string_lossy();
    let mut filename = String::with_capacity(180).add(&s_directory).add("/").add(&progname);
    if let Some(ref s_d) = *discriminant {
        filename = filename.add(&format!("_{}", s_d));
    }
    filename
}

fn get_filename(s_filename_base: &String, do_rotating: bool, rotate_idx: usize, o_suffix: &Option<String>,
                timestamp: bool)
                -> String {
    let mut filename = String::with_capacity(180).add(&s_filename_base);
    if timestamp {
        filename = filename.add(&Local::now().format("_%Y-%m-%d_%H-%M-%S").to_string())
    };
    if do_rotating {
        filename = filename.add(&format!("_r{:0>5}", rotate_idx))
    };
    if let &Some(ref suffix) = o_suffix {
        filename = filename.add(".").add(suffix);
    }
    filename
}

fn get_filename_pattern(s_filename_base: &String, o_suffix: &Option<String>) -> String {
    let mut filename = String::with_capacity(180).add(&s_filename_base);
    filename = filename.add("_r*");
    if let &Some(ref suffix) = o_suffix {
        filename = filename.add(".").add(suffix);
    }
    filename
}

fn get_next_rotate_idx(s_filename_base: &String, o_suffix: &Option<String>) -> usize {
    let mut rotate_idx = 0;
    let fn_pattern = get_filename_pattern(s_filename_base, o_suffix);
    match glob(&fn_pattern) {
        Err(e) => {
            print_err!("Is this ({}) really a directory? Listing failed with {}", fn_pattern, e);
        }
        Ok(globresults) => {
            for globresult in globresults {
                match globresult {
                    Err(e) => print_err!("Error occured when reading directory for log files: {:?}", e),
                    Ok(pathbuf) => {
                        let filename = pathbuf.file_stem().unwrap().to_string_lossy();
                        let mut it = filename.rsplit("_r");
                        let idx: usize = it.next().unwrap().parse().unwrap_or(0);
                        rotate_idx = max(rotate_idx, idx);
                    }
                }
            }
        }
    }
    rotate_idx + 1
}


mod platform {
    pub fn create_symlink_if_possible(link: &String) {
        linux_create_symlink(link);
    }

    #[cfg(target_os = "linux")]
    fn linux_create_symlink(link: &String) {
        use std::os::unix::fs as unix_fs;
        if fs::metadata(link).is_ok() {
            // old symlink must be removed before creating a new one
            let _ = fs::remove_file(link);
        }

        if let Err(e) = unix_fs::symlink(&path, link) {
            print_err!("Can not create symlink \"{}\" for path \"{}\": {}", link, &path.display(), e);
        }
    }

    // And this function only gets compiled if the target OS is *not* linux
    #[cfg(not(target_os = "linux"))]
    #[allow(unused_variables)]
    fn linux_create_symlink(link: &String) {}
}
