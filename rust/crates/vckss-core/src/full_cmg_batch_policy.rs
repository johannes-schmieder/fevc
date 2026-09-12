//! Thread-aware automatic widths for full CMG; no public tuning option.
use crate::error::{BackendError, ErrorCode, Result};

// k=2 was frozen before the accepted mixed-degree end-to-end comparisons.
pub(crate) const SELECTED_K: usize = 2;

pub(crate) fn caps(probes: usize, threads: usize, k: usize) -> Result<(usize, usize, usize)> {
    if probes == 0 || threads == 0 || !matches!(k, 1 | 2) {
        return Err(BackendError::invalid(
            "full_cmg_batches",
            "invalid probe/thread/k count",
        ));
    }
    let quantum = threads
        .checked_mul(4)
        .and_then(|v| v.checked_mul(k))
        .ok_or_else(|| {
            BackendError::new(
                ErrorCode::ResourceLimit,
                "full_cmg_batches",
                "batch quantum overflow",
            )
        })?;
    let leverage = probes.min(32.max(quantum));
    let target = probes.min(32.max(quantum / 2));
    let maximum = leverage.max(target.checked_mul(2).ok_or_else(|| {
        BackendError::new(
            ErrorCode::ResourceLimit,
            "full_cmg_batches",
            "target RHS overflow",
        )
    })?);
    Ok((leverage, target, maximum))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn arbitrary_threads_probes_and_tails_are_bounded() {
        for t in [1, 2, 3, 4, 7, 14, 28, 64] {
            for p in [
                1, 2, 3, 4, 5, 7, 8, 9, 21, 31, 32, 33, 63, 64, 65, 199, 200, 201,
            ] {
                for k in [1, 2] {
                    let (l, r, m) = caps(p, t, k).unwrap();
                    assert_eq!(l, p.min(32.max(4 * k * t)));
                    assert_eq!(r, p.min(32.max(2 * k * t)));
                    assert_eq!(m, l.max(2 * r));
                    assert!(l > 0 && r > 0 && l <= p && r <= p);
                    assert_eq!((0..p).step_by(l).map(|i| l.min(p - i)).sum::<usize>(), p);
                    assert_eq!(
                        (0..p).step_by(r).map(|i| 2 * r.min(p - i)).sum::<usize>(),
                        2 * p
                    );
                }
            }
        }
    }
    #[test]
    fn invalid_and_overflow_counts_fail_before_work() {
        for (p, t, k) in [(0, 1, 1), (1, 0, 1), (1, 1, 0), (1, 1, 3)] {
            assert!(caps(p, t, k).is_err());
        }
        assert_eq!(
            caps(200, usize::MAX, 2).unwrap_err().code,
            ErrorCode::ResourceLimit
        );
        assert_eq!(caps(200, 28, 1).unwrap(), (112, 56, 112));
        assert_eq!(caps(200, 28, 2).unwrap(), (200, 112, 224));
    }
}
