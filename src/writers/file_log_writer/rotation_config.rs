use crate::{Cleanup, Criterion, Naming};

// Describes how rotation should work
#[derive(Clone, Debug)]
pub(super) struct RotationConfig {
    // Defines if rotation should be based on size or date
    pub(crate) criterion: Criterion,
    // Defines if rotated files should be numbered or get a date-based name
    pub(crate) naming: Naming,
    // Defines the cleanup strategy
    pub(crate) cleanup: Cleanup,
}
