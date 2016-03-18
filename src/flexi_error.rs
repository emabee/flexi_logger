use std::fmt;


/// Describes errors in the initialization of flexi_logger.
#[derive(Debug)]
pub struct FlexiLoggerError {
    message: String,
}
impl FlexiLoggerError {
    /// Constructs an instance from a String.
    pub fn new(s: String) -> FlexiLoggerError {
        FlexiLoggerError { message: s }
    }
}
impl fmt::Display for FlexiLoggerError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.message)
    }
}
