//! Internal cooperative-cancellation checkpoints for VCkss-owned execution.

use std::sync::atomic::{AtomicBool, Ordering};

use crate::CmgError;

#[inline]
pub(crate) fn checkpoint(
    cancellation: Option<&AtomicBool>,
    phase: &'static str,
) -> Result<(), CmgError> {
    if cancellation.is_some_and(|flag| flag.load(Ordering::Acquire)) {
        Err(CmgError::Cancelled { phase })
    } else {
        Ok(())
    }
}
