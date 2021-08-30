use chrono::{DateTime, Local};

/// Deferred timestamp creation.
///
/// Is used to ensure that a log record that is sent to multiple outputs
/// (in maybe different formats) always uses the same timestamp.
#[derive(Debug)]
pub struct DeferredNow(Option<DateTime<Local>>);
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
    pub fn now(&'a mut self) -> &'a DateTime<Local> {
        self.0.get_or_insert_with(Local::now)
    }
}
