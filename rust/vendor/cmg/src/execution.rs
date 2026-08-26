//! Optional package-owned parallel execution support.

use crate::CmgError;

/// Minimum item count for setup kernels whose parallel path pays sorting or
/// temporary sparse-storage overhead.
#[cfg(feature = "parallel")]
pub(crate) const PARALLEL_SETUP_MIN_ITEMS: usize = 65_536;

/// Options controlling optional package-owned parallel execution.
///
/// A thread count of zero selects [`std::thread::available_parallelism`]. The
/// memory budget applies to concurrently retained solver workspaces; immutable
/// graph and hierarchy storage are not included in that budget.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParallelOptions {
    /// Number of worker threads, or zero to detect the available parallelism.
    pub threads: usize,
    /// Minimum vector or row count before a within-kernel parallel path is used.
    pub min_parallel_len: usize,
    /// Fixed chunk size reserved for deterministic parallel reductions.
    pub reduction_chunk_size: usize,
    /// Optional budget for concurrently retained solver workspaces, in bytes.
    pub workspace_memory_budget_bytes: Option<usize>,
}

impl Default for ParallelOptions {
    fn default() -> Self {
        Self {
            threads: 0,
            min_parallel_len: 4_096,
            reduction_chunk_size: 16_384,
            workspace_memory_budget_bytes: None,
        }
    }
}

impl ParallelOptions {
    /// Validate the execution options without creating a thread pool.
    pub fn validate(self) -> Result<Self, CmgError> {
        if self.min_parallel_len == 0 {
            return Err(CmgError::InvalidOption {
                name: "min_parallel_len",
                value: 0.0,
            });
        }
        if self.reduction_chunk_size == 0 {
            return Err(CmgError::InvalidOption {
                name: "reduction_chunk_size",
                value: 0.0,
            });
        }
        if self.workspace_memory_budget_bytes == Some(0) {
            return Err(CmgError::InvalidOption {
                name: "workspace_memory_budget_bytes",
                value: 0.0,
            });
        }
        Ok(self)
    }
}

/// A reusable package-owned Rayon thread pool.
///
/// This type is available only with the `parallel` Cargo feature. It never
/// configures or relies on Rayon's process-wide global pool.
#[cfg(feature = "parallel")]
pub struct ParallelExecutor {
    pool: rayon::ThreadPool,
    threads: usize,
    options: ParallelOptions,
}

#[cfg(feature = "parallel")]
impl core::fmt::Debug for ParallelExecutor {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("ParallelExecutor")
            .field("threads", &self.threads)
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "parallel")]
impl ParallelExecutor {
    /// Construct an isolated thread pool with the requested execution policy.
    pub fn new(options: ParallelOptions) -> Result<Self, CmgError> {
        let options = options.validate()?;
        let threads = if options.threads == 0 {
            std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(1)
        } else {
            options.threads
        };
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|index| format!("cmg-worker-{index}"))
            .build()
            .map_err(|error| CmgError::ParallelRuntime {
                message: error.to_string(),
            })?;
        Ok(Self {
            pool,
            threads,
            options,
        })
    }

    /// Return the actual number of worker threads in this pool.
    #[must_use]
    pub const fn thread_count(&self) -> usize {
        self.threads
    }

    /// Return the validated execution options used to create this executor.
    #[must_use]
    pub const fn options(&self) -> ParallelOptions {
        self.options
    }

    /// Return the maximum number of simultaneous workspaces allowed for a
    /// batch of the supplied size.
    pub fn batch_concurrency(
        &self,
        workspace_bytes: usize,
        batch_len: usize,
    ) -> Result<usize, CmgError> {
        if batch_len == 0 {
            return Ok(0);
        }
        let thread_limit = self.threads.min(batch_len).max(1);
        let Some(budget) = self.options.workspace_memory_budget_bytes else {
            return Ok(thread_limit);
        };
        if workspace_bytes > budget {
            return Err(CmgError::MemoryBudgetExceeded {
                required_bytes: workspace_bytes,
                budget_bytes: budget,
            });
        }
        if workspace_bytes == 0 {
            return Ok(thread_limit);
        }
        Ok(thread_limit.min(budget / workspace_bytes).max(1))
    }

    pub(crate) fn install<Operation, Output>(&self, operation: Operation) -> Output
    where
        Operation: FnOnce() -> Output + Send,
        Output: Send,
    {
        self.pool.install(operation)
    }

    pub(crate) fn should_parallel(&self, length: usize) -> bool {
        self.threads > 1 && length >= self.options.min_parallel_len
    }

    pub(crate) fn work_chunk_len(&self, length: usize) -> usize {
        let target_chunks = self.threads.saturating_mul(8).max(1);
        let quotient = length / target_chunks;
        let remainder = usize::from(length % target_chunks != 0);
        quotient.saturating_add(remainder).max(1)
    }
}
