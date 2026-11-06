//! Borrowed execution and canonical admission for bounded independent chunks.

use crate::{CmgError, Team};

/// Fixed logical chunking of a mutable slice for independent bounded kernels.
///
/// The caller admits and allocates the slice and any kernel scratch separately,
/// before dispatch. The dispatcher allocates no task/result/scratch banks.
/// Each kernel invocation must do bounded work on its chunk, use only disjoint
/// admitted storage and perform no allocation, nested parallel work or adaptive
/// unbounded iteration. This is a caller contract, not an allocator sandbox.
/// Chunk boundaries depend only on the slice length and `chunk_len`, never on
/// the number of workers.
pub struct ControlledStage;

impl ControlledStage {
    /// Validate a stage's logical chunk geometry. No work is charged; the
    /// returned single-use stage cannot be dispatched twice.
    pub fn admit(len: usize, chunk_len: usize) -> Result<AdmittedStage, CmgError> {
        if chunk_len == 0 {
            return Err(CmgError::InvalidHierarchy {
                context: "invalid controlled stage chunk policy",
            });
        }
        Ok(AdmittedStage {
            len,
            chunk_len,
            chunks: len.div_ceil(chunk_len),
        })
    }
}

/// A single-use, validated chunk plan for one stage.
///
/// No executor, slice or scratch owner is retained. Consuming the plan prevents
/// accidental redispatch.
#[derive(Debug)]
pub struct AdmittedStage {
    len: usize,
    chunk_len: usize,
    chunks: usize,
}

impl AdmittedStage {
    /// Execute disjoint bounded chunks and join before returning or unwinding.
    ///
    /// `cancelled` must be a cheap, allocation-free, cancellation-only query
    /// that remains true once cancellation is requested during this stage;
    /// it cannot admit work or access a serial controller. It is polled before
    /// each kernel invocation and after the join. A true result interrupts the
    /// stage and dominates kernel errors. Otherwise the earliest failing chunk
    /// in logical order wins, independent of scheduling. All admitted chunks
    /// run unless cancelled or unwinding; kernel errors do not cancel siblings.
    /// Outputs remain private partial state on error and must not be published.
    /// Dispatch installs on the team's pool; on a worker of that pool it runs
    /// inline. No callbacks or arithmetic from existing serial builders change.
    pub fn run<T: Send>(
        self,
        team: Team<'_>,
        values: &mut [T],
        cancelled: impl Fn() -> bool + Sync,
        kernel: impl Fn(usize, &mut [T]) -> Result<(), CmgError> + Sync,
    ) -> Result<(), CmgError> {
        if values.len() != self.len {
            return Err(CmgError::InvalidHierarchy {
                context: "controlled stage output length differs from admission",
            });
        }
        let lanes = team.worker_ceiling().min(self.chunks).max(1);
        let result = team
            .install(|| self.dispatch(team, values, 0, self.chunks, lanes, &cancelled, &kernel));
        if cancelled() {
            return Err(self.interrupted());
        }
        result
    }

    fn interrupted(&self) -> CmgError {
        CmgError::Cancelled {
            phase: "parallel stage execution",
        }
    }

    // The team is read only by the test-build concurrency gauge.
    #[allow(clippy::too_many_arguments)]
    #[cfg_attr(not(test), allow(clippy::only_used_in_recursion))]
    fn dispatch<T: Send, C: Fn() -> bool + Sync, F>(
        &self,
        #[cfg_attr(not(test), allow(unused_variables))] team: Team<'_>,
        values: &mut [T],
        first: usize,
        chunks: usize,
        lanes: usize,
        cancelled: &C,
        kernel: &F,
    ) -> Result<(), CmgError>
    where
        F: Fn(usize, &mut [T]) -> Result<(), CmgError> + Sync,
    {
        if lanes <= 1 || chunks <= 1 {
            #[cfg(test)]
            let _lane = team.gauge_lane();
            let mut error = None;
            for (offset, chunk) in values.chunks_mut(self.chunk_len).enumerate() {
                if cancelled() {
                    return Err(self.interrupted());
                }
                if let Err(failure) = kernel(first + offset, chunk) {
                    if error.is_none() {
                        error = Some(failure);
                    }
                }
            }
            return error.map_or(Ok(()), Err);
        }
        let left_lanes = lanes / 2;
        let left_chunks = ((chunks as u128 * left_lanes as u128) / lanes as u128) as usize;
        // Every left chunk is full; the potentially short chunk is rightmost.
        let (left, right) = values.split_at_mut(left_chunks * self.chunk_len);
        let (a, b) = rayon::join(
            || {
                self.dispatch(
                    team,
                    left,
                    first,
                    left_chunks,
                    left_lanes,
                    cancelled,
                    kernel,
                )
            },
            || {
                self.dispatch(
                    team,
                    right,
                    first + left_chunks,
                    chunks - left_chunks,
                    lanes - left_lanes,
                    cancelled,
                    kernel,
                )
            },
        );
        a.and(b)
    }
}
