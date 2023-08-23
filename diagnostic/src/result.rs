use super::{CriticalResult, WarningResult};

// Used for operations that have both critical and non-critical errors
pub type Result<T, E> = CriticalResult<WarningResult<T, E>, E>;
