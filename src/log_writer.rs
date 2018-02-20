/// Boxed instances of LogWriter can be used as additional log targets
pub trait LogWriter: Sync + Send {
    /// write out a log line
    fn write(&self, msgb: &[u8]);
}
