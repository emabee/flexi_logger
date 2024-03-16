mod test_utils;

use std::{
    io::Write,
    path::{Path, PathBuf},
};

const CTRL_INDEX: &str = "CTRL_INDEX";
const CRASHFILE: &str = "CRASHFILE";
const RUNS: usize = 3;
const MILLIS: u64 = 50;

// use the same technique as test_utils::dispatch to launch itself in child mode,
// but do it twice:
//   controller starts parent, parent starts child
//   controller keeps running and verifies that the child's panic file is created (or not),
//   parent terminates directly and thus destroys the stderr of child, thus forcing child to panic
#[test]
fn main() {
    match std::env::var(CTRL_INDEX).as_ref() {
        Err(_) => {
            controller();
        }
        Ok(s) if s == "parent" => {
            parent(false);
        }
        Ok(s) if s == "parent_panic" => {
            parent(true);
        }
        Ok(s) if s == "child" => {
            child(false);
        }
        Ok(s) if s == "child_panic" => {
            child(true);
        }
        Ok(s) => panic!("Unexpected value {s}"),
    }
}

fn controller() {
    let progpath = std::env::args().next().unwrap();

    std::fs::create_dir_all(crashdump_file().parent().unwrap()).unwrap();

    std::fs::remove_file(crashdump_file()).ok();

    // First run: don't panic
    std::process::Command::new(progpath.clone())
        .env(CTRL_INDEX, "parent")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    // wait until child has terminated
    std::thread::sleep(std::time::Duration::from_millis(MILLIS * (RUNS as u64 + 5)));

    // check that no crashdump_file was written
    assert!(!Path::new(&crashdump_file()).try_exists().unwrap());

    // Second run: panic
    std::process::Command::new(progpath)
        .env(CTRL_INDEX, "parent_panic")
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();

    // wait until child has terminated
    std::thread::sleep(std::time::Duration::from_millis(MILLIS * (RUNS as u64 + 5)));

    // check that crashdump_file was written
    assert!(Path::new(&crashdump_file()).try_exists().unwrap());
}

fn parent(panic: bool) {
    let progpath = std::env::args().next().unwrap();
    // spawn child and terminate directly, thus destroying the child's stderr
    std::process::Command::new(progpath)
        .env(CTRL_INDEX, if panic { "child_panic" } else { "child" })
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap();
}

fn child(panic: bool) {
    let original_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic| {
        let backtrace = std::backtrace::Backtrace::capture();

        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .write(true)
            .truncate(true)
            .open(crashdump_file())
            .unwrap();
        file.write_all(format!("Panic occured:\n{}\n{}\n", panic, backtrace).as_bytes())
            .unwrap();
        file.flush().unwrap();

        original_hook(panic);
    }));

    let _logger = flexi_logger::Logger::try_with_str("info")
        .unwrap()
        .log_to_stderr()
        .panic_if_error_channel_is_broken(panic)
        .start()
        .unwrap();

    for i in 0..RUNS {
        log::info!("log test ({i})"); // <-- causes panic when parent terminated
        std::thread::sleep(std::time::Duration::from_millis(MILLIS));
    }
}

// controller is first caller and writes name to env, all other calls should find the env
// and take the value from there
fn crashdump_file() -> PathBuf {
    match std::env::var(CRASHFILE) {
        Ok(s) => Path::new(&s).to_path_buf(),
        Err(_) => {
            let progname = PathBuf::from(std::env::args().next().unwrap())
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();
            let path = test_utils::file(&format!("./{progname}.log"));
            std::env::set_var(CRASHFILE, &path);
            path
        }
    }
}
