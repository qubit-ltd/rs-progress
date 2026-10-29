use std::sync::atomic::AtomicU64;
use std::sync::atomic::Ordering;

use crate::error::StartError;

/// Allocates a nonzero operation ID from the supplied counter.
///
/// Returns [`StartError::OperationIdExhausted`] when the counter holds zero.
pub(crate) fn allocate_operation_id_from(counter: &AtomicU64) -> Result<u64, StartError> {
    loop {
        let current = counter.load(Ordering::Relaxed);
        if current == 0 {
            return Err(StartError::OperationIdExhausted);
        }
        let next = current.checked_add(1).unwrap_or(0);
        if counter
            .compare_exchange_weak(current, next, Ordering::Relaxed, Ordering::Relaxed)
            .is_ok()
        {
            return Ok(current);
        }
    }
}
