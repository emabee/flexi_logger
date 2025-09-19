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
    
    /// The specified number of days log files are kept.
    /// Older files are deleted, if necessary.
    #[cfg(feature = "days")]
    #[cfg_attr(docsrs, doc(cfg(feature = "days")))]
    KeepLogDays(usize),

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
