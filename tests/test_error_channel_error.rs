mod test_utils;

use std::{
    env,
    fs::{create_dir_all, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
    process::{Command, Stdio},
    str::FromStr,
    thread::sleep,
    time::Duration,
};

const TEST_CONTROL: &str = "TEST_CONTROL";
const LOG_FOLDER: &str = "LOG_FOLDER";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    NormalNoPanic,
    NormalPanic,
    DuplNoPanic,
    DuplPanic,
}
impl Mode {
    fn do_panic(&self) -> bool {
        match self {
            Mode::NormalNoPanic | Mode::DuplNoPanic => false,
            Mode::NormalPanic | Mode::DuplPanic => true,
        }
    }
    fn duplicate(&self) -> bool {
        match self {
            Mode::NormalNoPanic | Mode::NormalPanic => false,
            Mode::DuplNoPanic | Mode::DuplPanic => true,
        }
    }
    fn to_n(&self) -> u8 {
        match self {
            Mode::NormalNoPanic => 0,
            Mode::NormalPanic => 1,
            Mode::DuplNoPanic => 2,
            Mode::DuplPanic => 3,
        }
    }
}
enum CTRL {
    Parent(Mode),
    Child(Mode),
}
impl FromStr for CTRL {
    type Err = ();
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "A" => Ok(CTRL::Parent(Mode::NormalNoPanic)),
            "B" => Ok(CTRL::Parent(Mode::NormalPanic)),
            "C" => Ok(CTRL::Parent(Mode::DuplNoPanic)),
            "D" => Ok(CTRL::Parent(Mode::DuplPanic)),
            "E" => Ok(CTRL::Child(Mode::NormalNoPanic)),
            "F" => Ok(CTRL::Child(Mode::NormalPanic)),
            "G" => Ok(CTRL::Child(Mode::DuplNoPanic)),
            "H" => Ok(CTRL::Child(Mode::DuplPanic)),
            _ => Err(()),
        }
    }
}
impl ToString for CTRL {
    fn to_string(&self) -> String {
        match self {
            CTRL::Parent(Mode::NormalNoPanic) => "A".to_string(),
            CTRL::Parent(Mode::NormalPanic) => "B".to_string(),
            CTRL::Parent(Mode::DuplNoPanic) => "C".to_string(),
            CTRL::Parent(Mode::DuplPanic) => "D".to_string(),
            CTRL::Child(Mode::NormalNoPanic) => "E".to_string(),
            CTRL::Child(Mode::NormalPanic) => "F".to_string(),
            CTRL::Child(Mode::DuplNoPanic) => "G".to_string(),
            CTRL::Child(Mode::DuplPanic) => "H".to_string(),
        }
    }
}

// use the same technique as test_utils::dispatch to launch itself in child mode,
// but do it twice:
//   controller starts parent, parent starts child
//   controller keeps running and verifies that the child's panic file is created (or not),
//   parent terminates directly and thus destroys the stderr of child, thus forcing child to panic
#[test]
fn main() {
    match env::var(TEST_CONTROL).as_ref() {
        Err(_) => {
            controller();
        }
        Ok(s) => match s.parse::<CTRL>() {
            Ok(v) => match v {
                CTRL::Parent(m) => parent(m),
                CTRL::Child(m) => child(m),
            },
            Err(()) => panic!("Unexpected value {s}"),
        },
    }
}

fn controller() {
    let progpath = env::args().next().unwrap();
    create_dir_all(crashdump_file(0).parent().unwrap()).unwrap();

    println!("Starting at {}", chrono::Local::now());

    for mode in [
        Mode::NormalNoPanic,
        Mode::DuplNoPanic,
        Mode::NormalPanic,
        Mode::DuplPanic,
    ] {
        println!("Testing mode {mode:?} at {}", chrono::Local::now());
        let mut parent = Command::new(progpath.clone())
            .env(TEST_CONTROL, CTRL::Parent(mode).to_string())
            .stdout(Stdio::null())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap();
        assert!(parent.wait().expect("failed to wait on parent").success());

        sleep(Duration::from_millis(50));
        match mode {
            Mode::NormalNoPanic | Mode::DuplNoPanic => {
                // check that no crashdump_file was written
                assert!(!Path::new(&crashdump_file(mode.to_n()))
                    .try_exists()
                    .unwrap());
            }
            Mode::NormalPanic | Mode::DuplPanic => {
                // check that crashdump_file was written
                assert!(Path::new(&crashdump_file(mode.to_n()))
                    .try_exists()
                    .unwrap());
            }
        }
    }
}

fn parent(mode: Mode) {
    let progpath = env::args().next().unwrap();
    // we don't want to wait here, and it's not an issue because this is not a long running program
    #[allow(clippy::zombie_processes)]
    // spawn child and terminate directly, thus destroying the child's stderr
    Command::new(progpath)
        .env(TEST_CONTROL, CTRL::Child(mode).to_string())
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
}

fn child(mode: Mode) {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        let backtrace = std::backtrace::Backtrace::capture();

        let mut file = OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(crashdump_file(mode.to_n()))
            .unwrap();
        file.write_all(format!("Panic occured:\n{panic}\n{backtrace}\n").as_bytes())
            .unwrap();
        file.flush().unwrap();

        original_hook(panic);
    }));

    let mut logger = flexi_logger::Logger::try_with_str("info")
        .unwrap()
        .panic_if_error_channel_is_broken(mode.do_panic());
    logger = if mode.duplicate() {
        logger
            .log_to_file(
                flexi_logger::FileSpec::default()
                    .directory(test_utils::dir())
                    .basename(basename(mode.to_n()))
                    .suppress_timestamp(),
            )
            .duplicate_to_stderr(flexi_logger::Duplicate::All)
    } else {
        logger.log_to_stderr()
    };
    let _logger_handle = logger.start().unwrap();

    for i in 0..3 {
        log::info!("log test ({i})"); // <-- may cause panic when parent terminated
        sleep(Duration::from_millis(50));
    }
}

fn crashdump_file(n: u8) -> PathBuf {
    let mut folder = log_folder();
    folder.push(format!("crashdump_{n}.log"));
    folder
}
fn basename(n: u8) -> String {
    format!("log_{n}")
}

// controller is first caller and writes name to env, all other calls should find the env
// and take the value from there
fn log_folder() -> PathBuf {
    match env::var(LOG_FOLDER) {
        Ok(s) => Path::new(&s).to_path_buf(),
        Err(_) => {
            let path = test_utils::dir();
            env::set_var(LOG_FOLDER, &path);
            path
        }
    }
}

// fn prog_name() -> String {
//     PathBuf::from(env::args().next().unwrap())
//         .file_name()
//         .unwrap()
//         .to_string_lossy()
//         .to_string()
// }
