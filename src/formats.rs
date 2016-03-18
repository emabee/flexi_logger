use log::LogRecord;
use chrono::Local;

/// A logline-formatter that produces lines like <br>
/// ```INFO [my_prog::some_submodule] Task successfully read from conf.json```
pub fn default_format(record: &LogRecord) -> String {
    format!("{} [{}] {}", record.level(), record.location().module_path(), record.args())
}


/// A logline-formatter that produces lines like <br>
/// ```[2016-01-13 15:25:01.640870 +01:00] INFO [src/foo/bar:26] Task successfully read from conf.json```
pub fn opt_format(record: &LogRecord) -> String {
    format!("[{}] {} [{}:{}] {}",
            Local::now().format("%Y-%m-%d %H:%M:%S%.6f %:z"),
            record.level(),
            record.location().file(),
            record.location().line(),
            &record.args())
}


/// A logline-formatter that produces lines like <br>
/// ```[2016-01-13 15:25:01.640870 +01:00] INFO [foo::bar] src/foo/bar.rs:26: Task successfully read from conf.json```
pub fn detailed_format(record: &LogRecord) -> String {
    format!("[{}] {} [{}] {}:{}: {}",
            Local::now().format("%Y-%m-%d %H:%M:%S%.6f %:z"),
            record.level(),
            record.location().module_path(),
            record.location().file(),
            record.location().line(),
            &record.args())
}
