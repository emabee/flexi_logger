use log::LogRecord;
use chrono::Local;
use std::thread;

/// A logline-formatter that produces log lines like <br>
/// ```INFO [my_prog::some_submodule] Task successfully read from conf.json```
pub fn default_format(record: &LogRecord) -> String {
    format!("{} [{}] {}", record.level(), record.location().module_path(), record.args())
}


/// A logline-formatter that produces log lines like
/// <br>
/// ```[2016-01-13 15:25:01.640870 +01:00] INFO [src/foo/bar:26] Task successfully read from conf.json```
/// <br>
/// i.e. with timestamp and file location.
pub fn opt_format(record: &LogRecord) -> String {
    format!("[{}] {} [{}:{}] {}",
            Local::now().format("%Y-%m-%d %H:%M:%S%.6f %:z"),
            record.level(),
            record.location().file(),
            record.location().line(),
            &record.args())
}


/// A logline-formatter that produces log lines like
/// <br>
/// ```[2016-01-13 15:25:01.640870 +01:00] INFO [foo::bar] src/foo/bar.rs:26: Task successfully read from conf.json```
/// <br>
/// i.e. with timestamp, module path and file location.
pub fn detailed_format(record: &LogRecord) -> String {
    format!("[{}] {} [{}] {}:{}: {}",
            Local::now().format("%Y-%m-%d %H:%M:%S%.6f %:z"),
            record.level(),
            record.location().module_path(),
            record.location().file(),
            record.location().line(),
            &record.args())
}


/// A logline-formatter that produces log lines like
/// <br>
/// ```[2016-01-13 15:25:01.640870 +01:00] T[taskreader] INFO [src/foo/bar:26] Task successfully read from conf.json```
/// <br>
/// i.e. with timestamp, thread name and file location.
pub fn with_thread(record: &LogRecord) -> String {
    format!("[{}] T[{:?}] {} [{}:{}] {}",
            Local::now().format("%Y-%m-%d %H:%M:%S%.6f %:z"),
            thread::current().name().unwrap_or("<unnamed>"),
            record.level(),
            record.location().file(),
            record.location().line(),
            &record.args())
}
