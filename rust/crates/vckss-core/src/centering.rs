// SPDX-License-Identifier: GPL-3.0-only
use crate::error::{BackendError, Result};
use crate::interrupt::{checkpoint_chunk, InterruptCheck};
use crate::types::Centering;

pub(crate) fn mean(
    mode: Centering,
    y: &[f64],
    frequency: &[u64],
    total: u64,
    interrupt: &mut dyn InterruptCheck,
) -> Result<f64> {
    if mode == Centering::None {
        return Ok(0.0);
    }
    let anchor = y[0];
    let mut sum = crate::numerical_mc::Sum::default();
    for (row, (&value, &mass)) in y.iter().zip(frequency).enumerate() {
        checkpoint_chunk(interrupt, row, "centering_mean")?;
        sum.add((value - anchor) * (mass as f64 / total as f64));
    }
    let result = anchor + sum.get();
    if !result.is_finite() {
        return Err(BackendError::invalid(
            "centering",
            "working outcome mean is nonfinite",
        ));
    }
    Ok(result)
}

pub(crate) fn subtract(
    mode: Centering,
    y: &mut [f64],
    frequency: &[u64],
    total: u64,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    if mode == Centering::None {
        return Ok(());
    }
    let center = mean(mode, y, frequency, total, interrupt)?;
    for (row, value) in y.iter_mut().enumerate() {
        checkpoint_chunk(interrupt, row, "centering_outcome")?;
        *value -= center;
        if !value.is_finite() {
            return Err(BackendError::invalid(
                "centering",
                "centered outcome is nonfinite",
            ));
        }
    }
    Ok(())
}

pub(crate) mod jla;
