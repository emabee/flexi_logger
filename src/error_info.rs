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
//! It is then dropped immediately, and in its `Drop` impl it cleans up all ressources,
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
//! Possible reasons depend on the `WriteMode` and the output channel.
//!
//! With an asynchronous `WriteMode`, the root cause can be that the logger handle that was returned
//! from the logger initialization was not assigned to a variable to keep it alive (see also
//! [`Logger::start()`](https://docs.rs/flexi_logger/latest/flexi_logger/struct.Logger.html#method.start)).
//! It is then dropped immediately, and in its `Drop` impl it cleans up all ressources,
//! including the asynchronous writer. So the next log output will fail with this error.
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
//! by keeping their mutable parts in `Mutex`es, `RwLocks`, etc. In case that a thread panics
//! while owning one of these locks, the lock is subsequently considered "poisoned".
//!
//! Most likely the root cause for this is some panic! in a `Debug` or `Display` implementation
//! of a logged object.
//!
//! ## `LogFile`
//!
//! The `FileLogWriter` is not able to rotate the log file. The reason should be printed as well.
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
//! This error can only occur on linux systems, and when you use `Logger::create_symlink`, and
//! indicates an issue with creating or replacing the symbolic link to the log file.
//!
//! ## `Time`
//!
//! You're running on unix, and `time::OffsetDateTime::now_local()` returns an error,
//! due to `time`'s reaction on advisory
//! [RUSTSEC-2020-0159](https://rustsec.org/advisories/RUSTSEC-2020-0159).
//! This forces `flexi_logger` to work with UTC timestamps rather than with local time.
//! See also [issue-99](https://github.com/emabee/flexi_logger/issues/99) for more details.
//!
//! You can work around this issue by either
//!
//! - applying the `unsound_local_offset` pseudo-feature of `time` (see their documentation,
//!   the last section of <https://docs.rs/time/0.3.5/time/#feature-flags>)
//! - or using `flexi_logger`'s feature `use_chrono_for_offset`, which re-introduces the
//!   susceptibility to [RUSTSEC-2020-0159](https://rustsec.org/advisories/RUSTSEC-2020-0159)
//!   (unfortunate, but maybe acceptable in most cases).
//!
//! ## `WriterSpec`
//!
//! The code uses in some log macro call the syntax to send the log line to a certain `LogWriter`,
//! but this log writer does not exist.
//!
