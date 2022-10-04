//! Error codes of `flexi_logger`.
//!
//! The following error codes are used to indicate the reason of an error.
//! More details on them can be found here.
//!
//! ## `Write`
//!
//! Writing the log line to the output failed.
//!
//! Example:
//!
//! ```text
//! [flexi_logger][ERRCODE::Write] writing log line failed, caused by Send
//! ```
//!
//! Possible reasons depend on the `WriteMode` and the output channel.
//!
//! With an asynchronous `WriteMode`, the root cause can be that the logger handle that was returned
//! from the logger initialization was not assigned to a variable to keep it alive (see also
//! [`Logger::start()`](https://docs.rs/flexi_logger/latest/flexi_logger/struct.Logger.html#method.start)).
//! It is then dropped immediately, and in its `Drop` impl it cleans up all resources,
//! including the asynchronous writer. So the next log output will fail with this error.
//!
//! ## `Flush`
//!
//! Explicit or automatic flushing of buffered log lines to the output failed.
//!
//! Example:
//!
//! ```text
//! [flexi_logger][ERRCODE::Flush] flushing primary writer failed, caused by Send
//! ```
//!
//! For possible reasons, see [Write](#write).
//!
//! ## `Format`
//!
//! The chosen format function had produced an error.
//!
//! Example:
//!
//! ```text
//! [flexi_logger][ERRCODE::Format] formatting failed, caused by ...
//! ```
//!
//! If this happens with one of `flexi_logger`s provided format functions, please open an issue.
//!
//! ## `Poison`
//!
//! Log entries can be written by all threads of your program. Loggers thus must be thread-safe,
//! by guarding their mutable parts with `Mutex`es, `RwLocks`, etc. In case that a thread panics
//! while owning one of these locks, the lock is subsequently considered "poisoned".
//!
//! A typical root cause for this is some `panic!` in a `Debug` or `Display` implementation
//! of a logged object.
//!
//! ## `LogFile`
//!
//! The `FileLogWriter` is not able to rotate the log file. The reason should be printed as well.
//!
//! ## `LogFileWatcher`
//!
//! The `FileLogWriter` is not able to watch the log file. The reason should be printed as well.
//!
//! ## `LogSpecFile`
//!
//! This error can only occur if you use `Logger::start_with_specfile`, where you specify a
//! log-specification-file that you can edit, while the program is running, to influence
//! which log lines it should write.
//!
//! Examples:
//!
//! ```text
//! [flexi_logger][ERRCODE::LogSpecFile] continuing with previous log specification,
//! because rereading the log specification file failed, caused by ...
//! ```
//!
//! The log-specification-file you chose with `Logger::start_with_specfile` cannot be opened,
//! read, or successfully parsed.
//!
//! ```text
//! [flexi_logger][ERRCODE::LogSpecFile] error while watching the specfile, caused by ...
//! ```
//!
//! Watching the log-specification-file failed.
//!
//! ## `Symlink`
//!
//! This error can only occur on unix systems, and when you use `Logger::create_symlink`, and
//! indicates an issue with creating or replacing the symbolic link to the log file.
//!
//! ## `WriterSpec`
//!
//! The code uses in some log macro call the syntax to send the log line to a certain `LogWriter`,
//! but this log writer does not exist.
//!
