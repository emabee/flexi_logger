use time::OffsetDateTime;

/// Deferred timestamp creation.
///
/// Is used to ensure that a log record that is sent to multiple outputs
/// (in maybe different formats) always uses the same timestamp.
#[derive(Debug)]
pub struct DeferredNow(Option<OffsetDateTime>);
impl Default for DeferredNow {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> DeferredNow {
    /// Constructs a new instance, but does not generate the timestamp.
    #[must_use]
    pub fn new() -> Self {
        Self(None)
    }

    /// Retrieve the timestamp.
    ///
    /// Requires mutability because the first caller will generate the timestamp.
    #[allow(clippy::missing_panics_doc)]
    pub fn now(&'a mut self) -> &'a OffsetDateTime {
        self.0.get_or_insert_with(now_local_or_utc)
    }

    /// Convert into a formatted String.
    ///
    /// # Panics
    ///
    /// if fmt has an inappropriate value
    pub fn format(&'a mut self, fmt: impl Into<time::Format>) -> String {
        self.now().format(fmt)
    }
}

pub(crate) fn now_local_or_utc() -> OffsetDateTime {
    OffsetDateTime::try_now_local().unwrap_or_else(|_| OffsetDateTime::now_utc())
}
