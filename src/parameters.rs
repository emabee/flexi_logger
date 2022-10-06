/// Criterion when to rotate the log file.
///
/// Used in [`Logger::rotate`](crate::Logger::rotate).
#[derive(Copy, Clone, Debug)]
pub enum Criterion {
    /// Rotate the log file when it exceeds the specified size in bytes.
    Size(u64),
    /// Rotate the log file when it has become older than the specified age.
    ///
    /// ## Minor limitation
    ///
    /// ### TL,DR
    /// the combination of `Logger::append()`
    /// with `Criterion::Age` works OK, but not perfectly correct on Windows or unix
    /// when the program is restarted.
    ///
    /// ### Details
    /// Applying the age criterion works fine while your program is running.
    /// Ideally, we should also apply it to the rCURRENT file when the program is restarted
    /// and you chose the `Logger::append()` option.
    ///
    /// Unfortunately, this does not work on Windows, and it does not work on unix,
    /// for different reasons.
    ///
    /// To minimize the impact on age-based file-rotation,
    /// `flexi_logger` uses on Windows, and on all other platforms where the creation date
    /// of a file is not available (like on Unix), the last modification date
    /// (or, if this is also not available, the current time stamp)
    /// as the created_at-info of an rCURRENT file that already exists, and the
    /// current timestamp when file rotation happens during further execution.
    /// Consequently, a left-over rCURRENT file from a previous program run will look newer
    /// than it is, and will be used longer than it should be.
    ///
    /// #### Issue on Windows
    ///
    /// For compatibility with DOS (sic!), Windows magically transfers the created_at-info
    /// of a file that is deleted (or renamed) to its successor,
    /// when the recreation happens within some seconds [\[1\]](#ref-1).
    ///
    /// If the file property were used by `flexi_logger`,
    /// the rCURRENT file would always appear to be as old as the
    /// first one that ever was created - rotation by time would completely fail.
    ///
    /// <a name="ref-1">\[1\]</a> [https://superuser.com/questions/966490/windows-7-what-is-date-created-file-property-referring-to](https://superuser.com/questions/966490/windows-7-what-is-date-created-file-property-referring-to).
    ///
    Age(Age),
    /// Rotate the file when it has either become older than the specified age, or when it has
    /// exceeded the specified size in bytes.
    ///
    /// See documentation for Age and Size.
    AgeOrSize(Age, u64),
}

/// The age after which a log file rotation will be triggered,
/// when [`Criterion::Age`] is chosen.
#[derive(Copy, Clone, Debug)]
pub enum Age {
    /// Rotate the log file when the local clock has started a new day since the
    /// current file had been created.
    Day,
    /// Rotate the log file when the local clock has started a new hour since the
    /// current file had been created.
    Hour,
    /// Rotate the log file when the local clock has started a new minute since the
    /// current file had been created.
    Minute,
    /// Rotate the log file when the local clock has started a new second since the
    /// current file had been created.
    Second,
}

/// The naming convention for rotated log files.
///
/// With file rotation, the logs are written to a file with infix `_rCURRENT`.
/// When rotation happens, the CURRENT log file will be renamed to a file with
/// another infix of the form `"_r..."`. `Naming` defines which other infix will be used.
///
/// Used in [`Logger::rotate`](crate::Logger::rotate).
#[derive(Copy, Clone, Debug)]
pub enum Naming {
    /// File rotation rotates to files with a timestamp-infix, like `"r2020-01-27_14-41-08"`.
    Timestamps,
    /// File rotation rotates to files with a number-infix.
    Numbers,
}
/// Defines the strategy for handling older log files.
///
/// Is used in [`Logger::rotate`](crate::Logger::rotate).
///
/// Note that if you use a strategy other than `Cleanup::Never`, then the cleanup work is
/// by default done in an extra thread, to minimize the impact on the program.
///
/// See [`LoggerHandle::shutdown`](crate::LoggerHandle::shutdown)
/// to avoid interrupting a currently active cleanup when your program terminates.
///
/// See
/// [`Logger::cleanup_in_background_thread`](crate::Logger::cleanup_in_background_thread)
/// if you want to control whether this extra thread is created and used.
#[derive(Copy, Clone, Debug)]
pub enum Cleanup {
    /// Older log files are not touched - they remain for ever.
    Never,

    /// The specified number of rotated log files are kept.
    /// Older files are deleted, if necessary.
    KeepLogFiles(usize),

    /// The specified number of rotated log files are compressed and kept.
    /// Older files are deleted, if necessary.
    #[cfg_attr(docsrs, doc(cfg(feature = "compress")))]
    #[cfg(feature = "compress")]
    KeepCompressedFiles(usize),

    /// Allows keeping some files as text files and some as compressed files.
    ///
    /// ## Example
    ///
    /// `KeepLogAndCompressedFiles(5,30)` ensures that the youngest five log files are
    /// kept as text files, the next 30 are kept as compressed files with additional suffix `.gz`,
    /// and older files are removed.
    #[cfg_attr(docsrs, doc(cfg(feature = "compress")))]
    #[cfg(feature = "compress")]
    KeepLogAndCompressedFiles(usize, usize),
}

impl Cleanup {
    // Returns true if some cleanup is to be done.
    #[must_use]
    pub(crate) fn do_cleanup(&self) -> bool {
        !matches!(self, Self::Never)
    }
}
