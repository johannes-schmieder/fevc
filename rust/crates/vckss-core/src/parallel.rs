// SPDX-License-Identifier: GPL-3.0-only

use core::ops::Range;

use crate::error::{BackendError, ErrorCode, Result};

/// Deterministic fixed-partition executor.
///
/// Results are returned in ascending partition order; scheduling therefore
/// cannot change downstream merge order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeterministicExecutor {
    threads: usize,
}

impl DeterministicExecutor {
    pub fn new(threads: usize) -> Result<Self> {
        if threads == 0 {
            return Err(BackendError::invalid(
                "parallel",
                "thread count must be positive",
            ));
        }
        Ok(Self { threads })
    }

    #[must_use]
    pub const fn threads(self) -> usize {
        self.threads
    }

    #[must_use]
    pub fn partitions(self, length: usize) -> Vec<Range<usize>> {
        if length == 0 {
            return Vec::new();
        }
        let count = self.threads.min(length);
        let quotient = length / count;
        let remainder = length % count;
        let mut start = 0_usize;
        let mut ranges = Vec::with_capacity(count);
        for index in 0..count {
            let width = quotient + usize::from(index < remainder);
            let stop = start + width;
            ranges.push(start..stop);
            start = stop;
        }
        ranges
    }

    pub fn map_partitions<T, F>(self, length: usize, function: F) -> Result<Vec<T>>
    where
        T: Send,
        F: Fn(Range<usize>) -> Result<T> + Sync,
    {
        let ranges = self.partitions(length);
        if ranges.len() <= 1 {
            return ranges.into_iter().map(function).collect();
        }

        std::thread::scope(|scope| {
            let mut handles = Vec::with_capacity(ranges.len());
            for range in ranges {
                let function_ref = &function;
                handles.push(scope.spawn(move || function_ref(range)));
            }
            handles
                .into_iter()
                .map(|handle| {
                    handle.join().map_err(|_| {
                        BackendError::new(
                            ErrorCode::Panic,
                            "parallel",
                            "worker thread panicked",
                        )
                    })?
                })
                .collect()
        })
    }
}

#[must_use]
pub fn deterministic_sum(values: &[f64]) -> f64 {
    if values.is_empty() {
        return 0.0;
    }
    let mut current = values.to_vec();
    while current.len() > 1 {
        let mut next = Vec::with_capacity(current.len().div_ceil(2));
        for pair in current.chunks(2) {
            next.push(if pair.len() == 2 {
                pair[0] + pair[1]
            } else {
                pair[0]
            });
        }
        current = next;
    }
    current[0]
}

#[must_use]
pub fn compensated_sum(values: &[f64]) -> f64 {
    let mut sum = 0.0_f64;
    let mut correction = 0.0_f64;
    for &value in values {
        let adjusted = value - correction;
        let next = sum + adjusted;
        correction = (next - sum) - adjusted;
        sum = next;
    }
    sum
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn partitions_cover_range_without_overlap() {
        let ranges = DeterministicExecutor::new(3)
            .expect("executor")
            .partitions(10);
        assert_eq!(ranges, vec![0..4, 4..7, 7..10]);
    }

    #[test]
    fn maps_in_partition_order() {
        let executor = DeterministicExecutor::new(4).expect("executor");
        let result = executor
            .map_partitions(8, |range| Ok((range.start, range.end)))
            .expect("mapping");
        assert_eq!(result, vec![(0, 2), (2, 4), (4, 6), (6, 8)]);
    }

    #[test]
    fn deterministic_sum_has_fixed_tree() {
        let values = [1.0e16, 1.0, -1.0e16, 3.0];
        assert_eq!(deterministic_sum(&values), 4.0);
    }
}
