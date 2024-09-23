use super::age::Age;

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
    /// as the created_at-info of an rCURRENT file that already exists when the program is started,
    /// and the current timestamp when file rotation happens during further execution.
    /// Consequently, a left-over rCURRENT file from a previous program run will look newer
    /// than it is, and will be used longer than it should be.
    ///
    /// #### Issue on Linux
    ///
    /// Linux does not maintain a created-at property for files, only a last-changed-at property.
    ///
    /// #### Issue on Windows
    ///
    /// For compatibility with DOS (sic!), Windows magically transfers the created_at-info
    /// of a file that is deleted (or renamed) to its successor,
    /// when the recreation happens within some seconds [\[1\]](#ref-1).
    ///
    /// If the file property were used by `flexi_logger`,
    /// the rCURRENT file would always appear to be as old as the
    /// first one that ever was created - rotation by time would fail completely.
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
