/// The naming convention for rotated log files.
///
/// Common rule for all variants is that the names of the current output file
/// and the rotated log files only differ in the infix.
///
/// See [`Logger::log_to_file`](crate::Logger::log_to_file)
/// for a description of how the filename is built, including the infix.
///
/// See the variants for how the infix is used by them.
///
/// Used in [`Logger::rotate`](crate::Logger::rotate).
#[derive(Copy, Clone, Debug)]
pub enum Naming {
    /// Logs are written to a file with infix `rCURRENT`.
    ///
    /// File rotation renames this file to a name with a timestamp-infix
    /// like `"r2023-01-27_14-41-08"`, logging continues with a fresh file with infix `rCURRENT`.
    ///
    /// If multiple rotations happen within the same second, extended infixes are used like
    /// `"r2023-01-27_14-41-08.restart-0001"`.
    ///
    /// Same as
    /// ```rust
    /// # use flexi_logger::Naming;
    /// # let dummy =
    /// Naming::TimestampsCustomFormat {
    ///    current_infix: Some("rCURRENT"),
    ///     format: "r%Y-%m-%d_%H-%M-%S",
    /// }
    /// # ;
    /// ```
    Timestamps,

    /// Logs are written to a file with a timestamp-infix, like `"r2023-01-27_14-41-08"`.
    ///
    /// File rotation switches over to the next file.
    ///
    /// If multiple rotations happen within the same second, extended infixes are used like
    /// `"r2023-01-27_14-41-08.restart-0001"`.
    ///
    /// Same as
    /// ```rust
    /// # use flexi_logger::Naming;
    /// # let dummy =
    /// Naming::TimestampsCustomFormat {
    ///    current_infix: None,
    ///     format: "r%Y-%m-%d_%H-%M-%S",
    /// }
    /// # ;
    /// ```
    TimestampsDirect,

    /// Defines the infixes for the file to which the logs are written, and for the rotated files.
    TimestampsCustomFormat {
        /// Controls if a special infix is used for the file to which the logs are currently
        /// written.
        ///
        /// If `Some(infix)` is given, then it is taken as static infix for the file
        /// to which the logs are written.
        /// File rotation renames this file to a file with a timestamp infix.
        /// If this file already exists, an extended infix is used like
        /// `"2024-06-09.restart-0001"`.
        ///
        /// If `None` is given, then the logs will be directly written to a file with timestamp infix.
        /// File rotation only switches over to a new file with a fresh timestamp infix.
        /// If this file already exists, e.g. because rotation is triggered more frequently
        /// than the timestamp varies (according to the pattern), then an extended infix is used like
        /// `"2024-06-09.restart-0001"`.
        current_infix: Option<&'static str>,
        /// The format of the timestamp infix.
        ///
        /// See <https://docs.rs/chrono/latest/chrono/format/strftime/index.html> for a list of
        /// supported specifiers.
        ///
        /// **Make sure to use a format**
        ///
        /// - that is compatible to your file system(s) (e.g., don't use slashes),
        /// - that can be used by
        ///   [chrono::NaiveDateTime](https://docs.rs/chrono/latest/chrono/naive/struct.NaiveDateTime.html#method.parse_from_str)
        ///   or [chrono::NaiveDate](https://docs.rs/chrono/latest/chrono/naive/struct.NaiveDate.html#method.parse_from_str)
        ///
        /// Further, if you choose `current_infix` = `None` or `Some("")`, make sure to rotate only
        /// by [age](crate::Criterion::Age), and choose an age that is not smaller than what
        /// is expressed in the infix (e.g., don't rotate by minute if the infix only shows days).
        ///
        /// Examples:
        ///
        /// `"%Y-%m-%d"` produces timestamp infixes like `"2024-06-09"`.
        ///
        /// `"%Y-%m-%d_%H-%M-%S"` produces timestamp infixes like `"2024-06-09_13-24-35"`.
        format: &'static str,
    },

    /// Logs are written to a file with infix `rCURRENT`.
    ///
    /// File rotation renames this file to a name with a number-infix
    /// like `"r00000"`, `"r00001"`, etc.,
    /// logging continues with a fresh file with infix `rCURRENT`.
    Numbers,

    /// Logs are written to a file with a number-infix,
    /// like `"r00000"`, `"r00001"`, etc.
    ///
    /// File rotation switches over to the next file.
    NumbersDirect,

    /// Allows to specify custom infix and treat each file with basename as log file
    CustomFormat(CustomFormatter),
}

impl Naming {
    pub(crate) fn writes_direct(self) -> bool {
        matches!(
            self,
            Naming::NumbersDirect
                | Naming::TimestampsDirect
                | Naming::TimestampsCustomFormat {
                    current_infix: None | Some(""),
                    format: _
                }
        )
    }
}

/// Custom Formatter
#[derive(Copy, Clone, Debug)]
pub struct CustomFormatter {
    format_fn: fn(Option<String>) -> String,
}

impl CustomFormatter {
    /// Instantiate custom formatter
    pub fn new(format_fn: fn(Option<String>) -> String) -> Self {
        CustomFormatter { format_fn }
    }

    /// call custom formatter
    pub fn call(&self, o_last_infix: Option<String>) -> String {
        (self.format_fn)(o_last_infix)
    }
}
