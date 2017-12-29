use log::Record;
use chrono::Local;
use std::thread;

/// A logline-formatter that produces log lines like <br>
/// ```INFO [my_prog::some_submodule] Task successfully read from conf.json```
pub fn default_format(record: &Record) -> String {
    format!("{} [{}] {}", record.level(), record.module_path().unwrap_or("<unnamed>"), record.args())
}


/// A logline-formatter that produces log lines like
/// <br>
/// ```[2016-01-13 15:25:01.640870 +01:00] INFO [src/foo/bar:26] Task successfully read from conf.json```
/// <br>
/// i.e. with timestamp and file location.
pub fn opt_format(record: &Record) -> String {
    format!("[{}] {} [{}:{}] {}",
            Local::now().format("%Y-%m-%d %H:%M:%S%.6f %:z"),
            record.level(),
            record.file().unwrap_or("<unnamed>"),
            record.line().unwrap_or(0),
            &record.args())
}


/// A logline-formatter that produces log lines like
/// <br>
/// ```[2016-01-13 15:25:01.640870 +01:00] INFO [foo::bar] src/foo/bar.rs:26: Task successfully read from conf.json```
/// <br>
/// i.e. with timestamp, module path and file location.
pub fn detailed_format(record: &Record) -> String {
    format!("[{}] {} [{}] {}:{}: {}",
            Local::now().format("%Y-%m-%d %H:%M:%S%.6f %:z"),
            record.level(),
            record.module_path().unwrap_or("<unnamed>"),
            record.file().unwrap_or("<unnamed>"),
            record.line().unwrap_or(0),
            &record.args())
}


/// A logline-formatter that produces log lines like
/// <br>
/// ```[2016-01-13 15:25:01.640870 +01:00] T[taskreader] INFO [src/foo/bar:26] Task successfully read from conf.json```
/// <br>
/// i.e. with timestamp, thread name and file location.
pub fn with_thread(record: &Record) -> String {
    format!("[{}] T[{:?}] {} [{}:{}] {}",
            Local::now().format("%Y-%m-%d %H:%M:%S%.6f %:z"),
            thread::current().name().unwrap_or("<unnamed>"),
            record.level(),
            record.file().unwrap_or("<unnamed>"),
            record.line().unwrap_or(0),
            &record.args())
}
