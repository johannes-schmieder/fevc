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
    pub(crate) fn resolved_threads(self) -> usize {
        if self.threads == 0 {
            std::thread::available_parallelism()
                .map(usize::from)
                .unwrap_or(1)
        } else {
            self.threads
        }
    }

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

/// Explicit engineering allowances for an estimated-runtime policy.
///
/// These are configurable planning headroom, NOT measured usage, proved bounds
/// or allocator limits. Defaults are 32 MiB per pool, 2 MiB heap per worker and
/// 8 MiB requested stack per worker. Keep whole-process headroom separately.
#[cfg(feature = "parallel")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParallelRuntimeAllowance {
    /// Pool-wide runtime heap allowance in bytes.
    pub pool_heap_bytes: usize,
    /// Additional runtime heap allowance per worker in bytes.
    pub worker_heap_bytes: usize,
    /// Requested stack size per worker; must be positive. The estimated
    /// constructor sets this explicitly, overriding the platform/RUST_MIN_STACK
    /// default. Choose a larger value if the calling application requires it.
    pub worker_stack_bytes: usize,
}

#[cfg(feature = "parallel")]
impl Default for ParallelRuntimeAllowance {
    fn default() -> Self {
        Self {
            pool_heap_bytes: 32 * 1024 * 1024,
            worker_heap_bytes: 2 * 1024 * 1024,
            worker_stack_bytes: 8 * 1024 * 1024,
        }
    }
}

/// A checked planning estimate, separating known handle layouts from heuristic
/// runtime heap and requested thread stacks. This never certifies runtime bytes.
#[cfg(feature = "parallel")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ParallelRuntimeEstimate {
    threads: usize,
    worker_handles_bytes: usize,
    runtime_heap_allowance_bytes: usize,
    worker_stack_allowance_bytes: usize,
    total_bytes: usize,
}

#[cfg(feature = "parallel")]
impl ParallelRuntimeEstimate {
    /// Resolved worker count used by this estimate.
    pub const fn threads(self) -> usize {
        self.threads
    }
    /// CMG-owned handle-bank layouts. Already included when an executor or its
    /// owning solver is measured with CmgStorageReport; do not count twice.
    pub const fn worker_handles_bytes(self) -> usize {
        self.worker_handles_bytes
    }
    /// Heuristic runtime heap headroom, excluding the known handle bank.
    pub const fn runtime_heap_allowance_bytes(self) -> usize {
        self.runtime_heap_allowance_bytes
    }
    /// Sum of requested worker stack sizes. OS rounding/guard pages and actual
    /// residency are not certified. Do not add another host stack charge.
    pub const fn worker_stack_allowance_bytes(self) -> usize {
        self.worker_stack_allowance_bytes
    }
    /// Checked total of handle layouts and both allowances, used before startup.
    pub const fn total_bytes(self) -> usize {
        self.total_bytes
    }
}

#[cfg(feature = "parallel")]
impl ParallelRuntimeAllowance {
    /// Compute a checked planning estimate without starting workers. A zero
    /// thread request is resolved here; constructor admission uses that same
    /// resolved count rather than detecting available parallelism a second time.
    pub fn estimate(self, options: ParallelOptions) -> Result<ParallelRuntimeEstimate, CmgError> {
        let threads = options.validate()?.resolved_threads();
        if self.worker_stack_bytes == 0 {
            return Err(CmgError::InvalidOption {
                name: "worker_stack_bytes",
                value: 0.0,
            });
        }
        let multiply = |bytes: usize| {
            bytes
                .checked_mul(threads)
                .ok_or(CmgError::AllocationFailed {
                    context: "runtime allowance overflows",
                })
        };
        let worker_handles_bytes = crate::storage::bytes::<std::thread::JoinHandle<()>>(threads)?;
        let runtime_heap_allowance_bytes =
            crate::storage::add(self.pool_heap_bytes, multiply(self.worker_heap_bytes)?)?;
        let worker_stack_allowance_bytes = multiply(self.worker_stack_bytes)?;
        let total_bytes = crate::storage::add(
            worker_handles_bytes,
            crate::storage::add(runtime_heap_allowance_bytes, worker_stack_allowance_bytes)?,
        )?;
        Ok(ParallelRuntimeEstimate {
            threads,
            worker_handles_bytes,
            runtime_heap_allowance_bytes,
            worker_stack_allowance_bytes,
            total_bytes,
        })
    }
}

/// A reusable package-owned Rayon thread pool.
///
/// This type is available only with the `parallel` Cargo feature. It never
/// configures or relies on Rayon's process-wide global pool.
/// With supported synchronous/scoped use, external destruction waits for every
/// worker, including thread-local destructors. Detached coordinator work is not
/// supported. If such work retains the final executor owner on its own worker,
/// destruction signals termination and detaches handles to avoid self/peer
/// joins; workers then retire asynchronously after outstanding work finishes.
#[cfg(feature = "parallel")]
pub struct ParallelExecutor {
    pool: Option<rayon::ThreadPool>,
    workers: JoinedWorkers,
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
    /// Lend this pool as a borrowed team using every worker.
    #[must_use]
    pub fn team(&self) -> Team<'_> {
        Team {
            pool: self.pool.as_ref().expect("live CMG executor"),
            options: ParallelOptions {
                threads: self.threads,
                ..self.options
            },
        }
    }

    /// Rayon/crossbeam runtime heap currently has no source-proven finite
    /// admission bound. Worker handles are counted by `CmgStorageReport`, but
    /// queues, deferred reclamation, native thread resources and TLS are not.
    pub const fn runtime_allocation_bound() -> Option<usize> {
        None
    }

    /// Attempt strict runtime-heap admission before pool construction. Currently
    /// returns an uncovered-scope error for every finite budget. Use `new` only
    /// when the caller's policy explicitly permits unbounded runtime storage.
    pub fn new_with_memory_budget(
        options: ParallelOptions,
        budget_bytes: usize,
    ) -> Result<Self, CmgError> {
        options.validate()?;
        let required = Self::runtime_allocation_bound().ok_or(CmgError::InvalidHierarchy {
            context: "Rayon runtime memory coverage is unavailable",
        })?;
        crate::storage::admit(required, budget_bytes)?;
        Self::new(options)
    }

    /// Explicitly admit an ESTIMATE before worker/handle allocation, returning
    /// the charged estimate for reporting. This is not strict runtime admission
    /// and installs no allocator cap. Requests the allowance's worker stack
    /// size explicitly; the ordinary constructor's stack policy is unchanged.
    pub fn new_with_estimated_memory_budget(
        mut options: ParallelOptions,
        allowance: ParallelRuntimeAllowance,
        budget_bytes: usize,
    ) -> Result<(Self, ParallelRuntimeEstimate), CmgError> {
        let estimate = allowance.estimate(options)?;
        crate::storage::admit(estimate.total_bytes(), budget_bytes)?;
        options.threads = estimate.threads();
        let executor = Self::with_spawn_and_stack_size(
            options,
            Some(allowance.worker_stack_bytes),
            Self::spawn_worker,
        )?;
        Ok((executor, estimate))
    }

    /// Construct an isolated thread pool with the requested execution policy.
    pub fn new(options: ParallelOptions) -> Result<Self, CmgError> {
        Self::with_spawn(options, Self::spawn_worker)
    }

    fn spawn_worker(thread: rayon::ThreadBuilder) -> std::io::Result<std::thread::JoinHandle<()>> {
        let mut builder = std::thread::Builder::new();
        if let Some(name) = thread.name() {
            builder = builder.name(name.to_owned());
        }
        if let Some(size) = thread.stack_size() {
            builder = builder.stack_size(size);
        }
        builder.spawn(|| thread.run())
    }

    fn with_spawn(
        options: ParallelOptions,
        spawn: impl FnMut(rayon::ThreadBuilder) -> std::io::Result<std::thread::JoinHandle<()>>,
    ) -> Result<Self, CmgError> {
        Self::with_spawn_and_stack_size(options, None, spawn)
    }

    fn with_spawn_and_stack_size(
        options: ParallelOptions,
        stack_size: Option<usize>,
        mut spawn: impl FnMut(rayon::ThreadBuilder) -> std::io::Result<std::thread::JoinHandle<()>>,
    ) -> Result<Self, CmgError> {
        let options = options.validate()?;
        let threads = options.resolved_threads();
        let mut workers = JoinedWorkers::new(threads)?;
        let mut builder = rayon::ThreadPoolBuilder::new()
            .num_threads(threads)
            .thread_name(|index| format!("cmg-worker-{index}"));
        if let Some(size) = stack_size {
            builder = builder.stack_size(size);
        }
        let pool = builder
            .spawn_handler(|thread| {
                workers.handles.push(spawn(thread)?);
                Ok(())
            })
            .build()
            .map_err(|error| CmgError::ParallelRuntime {
                message: error.to_string(),
            })?;
        Ok(Self {
            pool: Some(pool),
            workers,
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
}

/// Without the parallel feature no team can exist; kernels keep one signature.
#[cfg(not(feature = "parallel"))]
#[derive(Clone, Copy, Debug)]
pub(crate) enum Team<'a> {
    #[allow(dead_code)]
    Never(core::convert::Infallible, core::marker::PhantomData<&'a ()>),
}

/// A borrowed Rayon pool, a positive worker ceiling and the execution policy.
///
/// A team creates no pool and owns nothing. [`ParallelExecutor::team`] lends a
/// CMG-owned pool; a host lends its own already admitted pool with
/// [`Team::new`], so CMG work runs on the host's workers instead of a nested
/// pool. Kernels install on the borrowed pool, which runs inline when the
/// caller already executes on it. Parallel eligibility depends on the worker
/// ceiling and options exactly as for an owned executor of that size.
#[cfg(feature = "parallel")]
#[derive(Clone, Copy)]
pub struct Team<'a> {
    pool: &'a rayon::ThreadPool,
    /// Validated options whose `threads` is the worker ceiling.
    options: ParallelOptions,
}

#[cfg(feature = "parallel")]
impl core::fmt::Debug for Team<'_> {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter
            .debug_struct("Team")
            .field("options", &self.options)
            .finish_non_exhaustive()
    }
}

#[cfg(feature = "parallel")]
impl<'a> Team<'a> {
    /// Borrow an existing pool with a worker ceiling in `1..=` its size.
    /// `options.threads` is replaced by the ceiling.
    pub fn new(
        pool: &'a rayon::ThreadPool,
        workers: usize,
        options: ParallelOptions,
    ) -> Result<Self, CmgError> {
        let options = options.validate()?;
        if workers == 0 || workers > pool.current_num_threads() {
            return Err(CmgError::InvalidHierarchy {
                context: "team worker ceiling is outside the borrowed pool",
            });
        }
        Ok(Self {
            pool,
            options: ParallelOptions {
                threads: workers,
                ..options
            },
        })
    }

    /// Maximum concurrently executing lanes of any stage.
    #[must_use]
    pub const fn worker_ceiling(&self) -> usize {
        self.options.threads
    }

    /// The execution shape: options whose `threads` is the worker ceiling.
    #[must_use]
    pub const fn options(&self) -> ParallelOptions {
        self.options
    }

    /// A one-lane view of the same pool with the same thresholds: every
    /// kernel runs on the calling lane, and fixed-chunk reductions and
    /// centering keep the team's reduction tree, so their bits equal those of
    /// the full team. Serial solves and each concurrent right-hand-side lane
    /// use it; no pool is created.
    #[must_use]
    pub const fn single_lane(&self) -> Self {
        let mut options = self.options;
        options.threads = 1;
        Self {
            pool: self.pool,
            options,
        }
    }

    /// Return the maximum number of simultaneous workspaces allowed for a
    /// batch of the supplied size.
    pub fn batch_concurrency(
        &self,
        workspace_bytes: usize,
        batch_len: usize,
    ) -> Result<usize, CmgError> {
        self.options.batch_concurrency(workspace_bytes, batch_len)
    }

    /// Run `operation` on the borrowed pool. Only the dispatchers below and
    /// [`crate::ControlledStage`] call this; kernels never fork on their own,
    /// so no CMG stage can exceed the worker ceiling.
    pub(crate) fn install<Operation, Output>(&self, operation: Operation) -> Output
    where
        Operation: FnOnce() -> Output + Send,
        Output: Send,
    {
        self.pool.install(operation)
    }

    #[cfg(test)]
    pub(crate) fn gauge_lane(&self) -> gauge::Lane {
        gauge::enter(self.pool)
    }

    /// Run `operation` on the pool with this team's whole lane budget.
    pub(crate) fn with_lanes<R: Send>(&self, operation: impl FnOnce(Lanes<'_>) -> R + Send) -> R {
        let lanes = Lanes {
            count: self.worker_ceiling(),
            pool: self.pool,
        };
        self.install(|| {
            #[cfg(test)]
            let _lane = (lanes.count == 1).then(|| gauge::enter(self.pool));
            operation(lanes)
        })
    }

    /// Fork `lanes` lane bodies by balanced binary joins. At most `lanes`
    /// closures exist, so at most that many CMG lanes run concurrently.
    fn fork(lanes: usize, body: &(impl Fn() + Sync)) {
        if lanes <= 1 {
            body();
        } else {
            rayon::join(
                || Self::fork(lanes / 2, body),
                || Self::fork(lanes - lanes / 2, body),
            );
        }
    }

    /// Process `values` in fixed chunks of `chunk_len` elements; chunks are
    /// claimed in ascending order by at most `worker_ceiling` lanes, each with
    /// its own [`crate::cancel::LanePoll`]. Chunk boundaries depend only on the
    /// slice and `chunk_len`. A lane stops at its first error; all lanes join.
    /// The earliest failing chunk's non-cancellation error is returned; callers
    /// poll their meter after the join so cancellation dominates.
    pub(crate) fn chunks_mut<T: Send>(
        &self,
        values: &mut [T],
        chunk_len: usize,
        query: Option<crate::Cancellation<'_>>,
        kernel: impl Fn(usize, &mut [T], &mut crate::cancel::LanePoll<'_>) -> Result<(), CmgError>
        + Sync,
    ) -> Result<(), CmgError> {
        let chunk_len = chunk_len.max(1);
        let chunks = values.len().div_ceil(chunk_len);
        self.claim(
            chunks,
            values.chunks_mut(chunk_len).enumerate(),
            query,
            |index, chunk, poll| kernel(index, chunk, poll),
        )
    }

    /// [`Self::chunks_mut`] over two equally long slices split at the same
    /// fixed chunk boundaries; each chunk receives both disjoint parts.
    pub(crate) fn chunks_mut_pair<A: Send, B: Send>(
        &self,
        first: &mut [A],
        second: &mut [B],
        chunk_len: usize,
        query: Option<crate::Cancellation<'_>>,
        kernel: impl Fn(
            usize,
            &mut [A],
            &mut [B],
            &mut crate::cancel::LanePoll<'_>,
        ) -> Result<(), CmgError>
        + Sync,
    ) -> Result<(), CmgError> {
        if first.len() != second.len() {
            return Err(CmgError::InvalidHierarchy {
                context: "paired chunk slices differ in length",
            });
        }
        let chunk_len = chunk_len.max(1);
        let chunks = first.len().div_ceil(chunk_len);
        self.claim(
            chunks,
            first
                .chunks_mut(chunk_len)
                .zip(second.chunks_mut(chunk_len))
                .enumerate(),
            query,
            |index, (left, right), poll| kernel(index, left, right, poll),
        )
    }

    /// The one claiming loop: at most `worker_ceiling` lanes take `chunks`
    /// items in ascending order, each lane stopping at its first error.
    fn claim<X: Send>(
        &self,
        chunks: usize,
        items: impl Iterator<Item = (usize, X)> + Send,
        query: Option<crate::Cancellation<'_>>,
        kernel: impl Fn(usize, X, &mut crate::cancel::LanePoll<'_>) -> Result<(), CmgError> + Sync,
    ) -> Result<(), CmgError> {
        if chunks == 0 {
            return Ok(());
        }
        let lanes = self.worker_ceiling().min(chunks);
        let pending = std::sync::Mutex::new(items);
        let failure: std::sync::Mutex<Option<(usize, CmgError)>> = std::sync::Mutex::new(None);
        let lane = || {
            #[cfg(test)]
            let _lane = gauge::enter(self.pool);
            let mut poll = crate::cancel::LanePoll::new(query);
            loop {
                let next = pending
                    .lock()
                    .unwrap_or_else(std::sync::PoisonError::into_inner)
                    .next();
                let Some((index, chunk)) = next else {
                    return;
                };
                if let Err(error) = kernel(index, chunk, &mut poll) {
                    if !matches!(error, CmgError::Cancelled { .. }) {
                        let mut first = failure
                            .lock()
                            .unwrap_or_else(std::sync::PoisonError::into_inner);
                        if first.as_ref().is_none_or(|(earliest, _)| index < *earliest) {
                            *first = Some((index, error));
                        }
                    }
                    return;
                }
            }
        };
        self.install(|| Self::fork(lanes, &lane));
        failure
            .into_inner()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .map_or(Ok(()), |(_, error)| Err(error))
    }

    /// Reduce leaves `0..leaves` (at least one) over the fixed balanced tree:
    /// a range splits at `first + len / 2` and `merge(left, right)` combines
    /// the halves. The tree depends only on `leaves`. Subtrees fork while the
    /// lane budget exceeds one, so at most `worker_ceiling` lanes run; each
    /// lane passes one [`crate::cancel::LanePoll`] to all of its leaves.
    ///
    /// A failing leaf stops its lane: the rest of that lane's leaves are not
    /// visited and every started lane joins. A lane that observes
    /// cancellation also stops every lane at its next leaf, so untouched
    /// leaves are not scheduled. Without cancellation, the leftmost failing
    /// leaf's error is returned (left subtrees precede right ones in every
    /// merge); callers poll their meter after the join so cancellation
    /// dominates.
    pub(crate) fn reduce<R: Send>(
        &self,
        leaves: usize,
        query: Option<crate::Cancellation<'_>>,
        leaf: impl Fn(usize, &mut crate::cancel::LanePoll<'_>) -> Result<R, CmgError> + Sync,
        merge: impl Fn(R, R) -> R + Sync,
    ) -> Result<R, CmgError> {
        type Stop = std::sync::atomic::AtomicBool;
        const PHASE: &str = "CMG fixed-tree reduction";
        fn serial<R>(
            first: usize,
            last: usize,
            poll: &mut crate::cancel::LanePoll<'_>,
            stop: &Stop,
            leaf: &(impl Fn(usize, &mut crate::cancel::LanePoll<'_>) -> Result<R, CmgError> + Sync),
            merge: &(impl Fn(R, R) -> R + Sync),
        ) -> Result<R, CmgError> {
            if last - first == 1 {
                if poll.stopped() || stop.load(std::sync::atomic::Ordering::Relaxed) {
                    return Err(crate::cancel::CancelMeter::cancelled(PHASE));
                }
                let value = leaf(first, poll);
                if poll.stopped() {
                    stop.store(true, std::sync::atomic::Ordering::Relaxed);
                }
                return value;
            }
            let middle = first + (last - first) / 2;
            let left = serial(first, middle, poll, stop, leaf, merge)?;
            let right = serial(middle, last, poll, stop, leaf, merge)?;
            Ok(merge(left, right))
        }
        #[allow(clippy::too_many_arguments)]
        fn forked<R: Send>(
            pool: &rayon::ThreadPool,
            first: usize,
            last: usize,
            lanes: usize,
            query: Option<crate::Cancellation<'_>>,
            stop: &Stop,
            leaf: &(impl Fn(usize, &mut crate::cancel::LanePoll<'_>) -> Result<R, CmgError> + Sync),
            merge: &(impl Fn(R, R) -> R + Sync),
        ) -> Result<R, CmgError> {
            if lanes <= 1 || last - first == 1 {
                #[cfg(test)]
                let _lane = gauge::enter(pool);
                let _ = pool;
                let mut poll = crate::cancel::LanePoll::new(query);
                return serial(first, last, &mut poll, stop, leaf, merge);
            }
            let middle = first + (last - first) / 2;
            let (left, right) = rayon::join(
                || forked(pool, first, middle, lanes / 2, query, stop, leaf, merge),
                || {
                    forked(
                        pool,
                        middle,
                        last,
                        lanes - lanes / 2,
                        query,
                        stop,
                        leaf,
                        merge,
                    )
                },
            );
            Ok(merge(left?, right?))
        }
        debug_assert!(leaves > 0);
        let lanes = self.worker_ceiling().min(leaves);
        let stop = Stop::new(false);
        self.install(|| forked(self.pool, 0, leaves, lanes, query, &stop, &leaf, &merge))
    }

    pub(crate) fn should_parallel(&self, length: usize) -> bool {
        self.options.should_parallel(length)
    }

    pub(crate) fn work_chunk_len(&self, length: usize) -> usize {
        self.options.work_chunk_len(length)
    }
}

/// A lane budget for recursive work on a team. Forking splits the budget, so
/// at most the team's worker ceiling of closures ever run concurrently; a
/// budget of one runs both halves in sequence on the current lane.
#[cfg(feature = "parallel")]
#[derive(Clone, Copy)]
pub(crate) struct Lanes<'a> {
    count: usize,
    #[cfg_attr(not(test), allow(dead_code))]
    pool: &'a rayon::ThreadPool,
}

#[cfg(feature = "parallel")]
impl Lanes<'_> {
    /// Remaining concurrent lanes, including this one.
    pub(crate) const fn count(self) -> usize {
        self.count
    }

    /// One lane: everything below runs sequentially here.
    pub(crate) const fn single(self) -> Self {
        Self {
            count: 1,
            pool: self.pool,
        }
    }

    /// Run `left` and `right`, concurrently when more than one lane remains,
    /// splitting the budget between them; otherwise in sequence.
    pub(crate) fn join<A: Send, B: Send>(
        self,
        left: impl FnOnce(Self) -> A + Send,
        right: impl FnOnce(Self) -> B + Send,
    ) -> (A, B) {
        if self.count <= 1 {
            let a = left(self);
            return (a, right(self));
        }
        let half = Self {
            count: self.count / 2,
            pool: self.pool,
        };
        let rest = Self {
            count: self.count - self.count / 2,
            pool: self.pool,
        };
        rayon::join(
            || {
                #[cfg(test)]
                let _lane = (half.count == 1).then(|| gauge::enter(half.pool));
                left(half)
            },
            || {
                #[cfg(test)]
                let _lane = (rest.count == 1).then(|| gauge::enter(rest.pool));
                right(rest)
            },
        )
    }
}

/// Test-only concurrency gauge: the maximum number of CMG lanes that ran at
/// once on one watched pool. Every dispatcher lane enters it.
#[cfg(all(test, feature = "parallel"))]
pub(crate) mod gauge {
    use std::sync::Mutex;

    struct State {
        pool: usize,
        active: usize,
        max: usize,
    }

    static STATE: Mutex<State> = Mutex::new(State {
        pool: 0,
        active: 0,
        max: 0,
    });
    /// Serializes witnesses that watch a pool.
    pub(crate) static WITNESS: Mutex<()> = Mutex::new(());

    fn state() -> std::sync::MutexGuard<'static, State> {
        STATE
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
    }

    pub(crate) struct Lane(bool);

    std::thread_local! {
        // Nested lane entries on this thread (a one-lane primitive inside a
        // lane runs inline): the thread counts as one concurrent lane.
        static DEPTH: core::cell::Cell<usize> = const { core::cell::Cell::new(0) };
    }

    pub(crate) fn enter(pool: &rayon::ThreadPool) -> Lane {
        let mut state = state();
        let watched = state.pool == core::ptr::from_ref(pool) as usize;
        if watched {
            let depth = DEPTH.with(|depth| depth.replace(depth.get() + 1));
            if depth == 0 {
                state.active += 1;
                state.max = state.max.max(state.active);
            }
        }
        Lane(watched)
    }

    impl Drop for Lane {
        fn drop(&mut self) {
            if self.0 {
                let mut state = state();
                let depth = DEPTH.with(|depth| {
                    depth.set(depth.get() - 1);
                    depth.get()
                });
                if depth == 0 {
                    state.active -= 1;
                }
            }
        }
    }

    /// Start watching `pool` and reset its maximum.
    pub(crate) fn watch(pool: &rayon::ThreadPool) {
        let mut state = state();
        *state = State {
            pool: core::ptr::from_ref(pool) as usize,
            active: 0,
            max: 0,
        };
    }

    /// The maximum concurrent lanes since `watch`.
    pub(crate) fn max() -> usize {
        state().max
    }

    /// Lanes of the watched pool running now.
    pub(crate) fn active() -> usize {
        state().active
    }

    /// Whether this thread is inside a one-lane leaf of the watched pool.
    pub(crate) fn inside() -> bool {
        DEPTH.with(|depth| depth.get() > 0)
    }
}

/// Test-only traversal counter: entries visited in one watched slice, so a
/// witness measures the work done after cancellation, not the polls.
#[cfg(all(test, feature = "parallel"))]
pub(crate) mod visits {
    use std::sync::atomic::{AtomicUsize, Ordering};

    static WATCHED: AtomicUsize = AtomicUsize::new(0);
    static WATCHED_END: AtomicUsize = AtomicUsize::new(0);
    static COUNT: AtomicUsize = AtomicUsize::new(0);

    /// Start counting visits of `data`'s entries (hold `gauge::WITNESS`).
    pub(crate) fn watch(data: &[f64]) {
        COUNT.store(0, Ordering::SeqCst);
        WATCHED.store(data.as_ptr() as usize, Ordering::SeqCst);
    }

    pub(crate) fn unwatch() {
        WATCHED.store(0, Ordering::SeqCst);
        WATCHED_END.store(0, Ordering::SeqCst);
    }

    /// Start counting visits of any entry inside `data` (hold
    /// `gauge::WITNESS`), for kernels that work on subslices.
    pub(crate) fn watch_range<T>(data: &[T]) {
        COUNT.store(0, Ordering::SeqCst);
        WATCHED_END.store(
            data.as_ptr() as usize + core::mem::size_of_val(data),
            Ordering::SeqCst,
        );
        WATCHED.store(data.as_ptr() as usize, Ordering::SeqCst);
    }

    /// Count one visited entry if it lies inside the range of `watch_range`.
    #[inline]
    pub(crate) fn visit_within(entry: *const u8) {
        let entry = entry as usize;
        let start = WATCHED.load(Ordering::Relaxed);
        if start != 0 && start <= entry && entry < WATCHED_END.load(Ordering::Relaxed) {
            COUNT.fetch_add(1, Ordering::Relaxed);
        }
    }

    /// Count one visited entry of the slice starting at `data`.
    #[inline]
    pub(crate) fn visit(data: *const f64) {
        if WATCHED.load(Ordering::Relaxed) == data as usize {
            COUNT.fetch_add(1, Ordering::Relaxed);
        }
    }

    pub(crate) fn count() -> usize {
        COUNT.load(Ordering::SeqCst)
    }
}

/// Execution-shape rules on options whose `threads` is a resolved worker
/// ceiling (a team's options): eligibility depends on this shape only.
#[cfg(feature = "parallel")]
impl ParallelOptions {
    pub(crate) fn batch_concurrency(
        self,
        workspace_bytes: usize,
        batch_len: usize,
    ) -> Result<usize, CmgError> {
        if batch_len == 0 {
            return Ok(0);
        }
        let thread_limit = self.threads.min(batch_len).max(1);
        let Some(budget) = self.workspace_memory_budget_bytes else {
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

    pub(crate) fn should_parallel(self, length: usize) -> bool {
        self.threads > 1 && length >= self.min_parallel_len
    }

    pub(crate) fn work_chunk_len(self, length: usize) -> usize {
        let target_chunks = self.threads.saturating_mul(8).max(1);
        let quotient = length / target_chunks;
        let remainder = usize::from(length % target_chunks != 0);
        quotient.saturating_add(remainder).max(1)
    }

    /// Minimum length of a fixed-chunk team reduction.
    pub(crate) fn reduction_floor(self) -> usize {
        self.min_parallel_len
            .max(self.reduction_chunk_size.saturating_mul(8))
    }
}

// ThreadPool::drop signals termination but does not join. Rayon's failed-build
// guard also signals termination before our handle bank unwinds. Keep this bank
// alive until after that signal, including on partial startup failure.
#[cfg(feature = "parallel")]
struct JoinedWorkers {
    handles: Vec<std::thread::JoinHandle<()>>,
}

#[cfg(feature = "parallel")]
impl crate::storage::Storage for std::thread::JoinHandle<()> {
    fn visit(
        &self,
        _: &mut dyn FnMut(crate::storage::Allocation) -> Result<(), CmgError>,
    ) -> Result<(), CmgError> {
        Ok(())
    }
    fn leaf() -> bool {
        true
    }
}
#[cfg(feature = "parallel")]
impl crate::storage::Storage for ParallelExecutor {
    fn visit(
        &self,
        visitor: &mut dyn FnMut(crate::storage::Allocation) -> Result<(), CmgError>,
    ) -> Result<(), CmgError> {
        crate::storage::Storage::visit(&self.workers.handles, visitor)?;
        crate::storage::runtime(visitor)
    }
}

#[cfg(feature = "parallel")]
impl JoinedWorkers {
    fn new(threads: usize) -> Result<Self, CmgError> {
        let mut handles = Vec::new();
        handles
            .try_reserve_exact(threads)
            .map_err(|_| CmgError::AllocationFailed {
                context: "executor worker handles",
            })?;
        Ok(Self { handles })
    }

    fn join(&mut self) {
        for handle in self.handles.drain(..) {
            // Do not panic again while cleaning up a caller panic. Final drop
            // on this pool's own worker detaches before reaching this path.
            let _ = handle.join();
        }
    }
}

#[cfg(feature = "parallel")]
impl Drop for JoinedWorkers {
    fn drop(&mut self) {
        self.join();
    }
}

#[cfg(feature = "parallel")]
impl Drop for ParallelExecutor {
    fn drop(&mut self) {
        let own_worker = self
            .pool
            .as_ref()
            .is_some_and(|pool| pool.current_thread_index().is_some());
        drop(self.pool.take());
        if own_worker {
            // A caller can misuse its coordinator callback to spawn detached
            // work holding the final Arc<ParallelExecutor>. Joining this worker
            // or peers that await its exit can deadlock. Signal termination
            // above, then detach; supported external retirement still joins.
            self.workers.handles.clear();
        } else {
            self.workers.join();
        }
    }
}

#[cfg(all(test, feature = "parallel"))]
mod lifetime_tests {
    use super::*;
    use std::cell::RefCell;
    use std::sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    };

    struct Exit(Arc<AtomicUsize>);
    impl Drop for Exit {
        fn drop(&mut self) {
            self.0.fetch_add(1, Ordering::SeqCst);
        }
    }
    thread_local! { static EXIT: RefCell<Option<Exit>> = const { RefCell::new(None) }; }

    fn observed(
        threads: usize,
        fail_at: Option<usize>,
        completed: Arc<AtomicUsize>,
    ) -> Result<ParallelExecutor, CmgError> {
        let mut started = 0;
        ParallelExecutor::with_spawn(
            ParallelOptions {
                threads,
                ..Default::default()
            },
            move |thread| {
                if Some(started) == fail_at {
                    return Err(std::io::Error::other("injected startup failure"));
                }
                started += 1;
                let completed = Arc::clone(&completed);
                std::thread::Builder::new().spawn(move || {
                    EXIT.with(|slot| *slot.borrow_mut() = Some(Exit(completed)));
                    thread.run();
                })
            },
        )
    }

    #[test]
    fn explicit_stack_request_reaches_spawn_and_partial_startup_joins() {
        let completed = Arc::new(AtomicUsize::new(0));
        let mut started = 0;
        let result = ParallelExecutor::with_spawn_and_stack_size(
            ParallelOptions {
                threads: 4,
                ..Default::default()
            },
            Some(8 * 1024 * 1024),
            |thread| {
                assert_eq!(thread.stack_size(), Some(8 * 1024 * 1024));
                if started == 2 {
                    return Err(std::io::Error::other("injected startup failure"));
                }
                started += 1;
                let completed = Arc::clone(&completed);
                std::thread::Builder::new()
                    .stack_size(thread.stack_size().unwrap())
                    .spawn(move || {
                        EXIT.with(|slot| *slot.borrow_mut() = Some(Exit(completed)));
                        thread.run();
                    })
            },
        );
        assert!(result.is_err());
        assert_eq!(completed.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn retirement_waits_for_actual_workers_and_tls_across_pool_sizes() {
        for threads in [1, 4, 1, 8, 2, 4, 1] {
            let completed = Arc::new(AtomicUsize::new(0));
            let executor = observed(threads, None, Arc::clone(&completed)).unwrap();
            assert_eq!(executor.team().install(rayon::current_num_threads), threads);
            assert_eq!(executor.workers.handles.len(), threads);
            // Final ownership stays outside the pool. All entries are its real
            // workers; the completion counter runs in their TLS destructors.
            assert!(
                !executor
                    .workers
                    .handles
                    .iter()
                    .any(|h| h.thread().id() == std::thread::current().id())
            );
            drop(executor);
            assert_eq!(completed.load(Ordering::SeqCst), threads);
        }
    }

    #[test]
    fn detached_last_owner_on_worker_retires_without_self_join() {
        const CHILD: &str = "CMG_DETACHED_LAST_OWNER_CHILD";
        if std::env::var_os(CHILD).is_none() {
            // A regression must fail this bounded child, never hang the suite.
            let mut child = std::process::Command::new(std::env::current_exe().unwrap())
                .env(CHILD, "1")
                .args([
                    "execution::lifetime_tests::detached_last_owner_on_worker_retires_without_self_join",
                    "--exact",
                    "--test-threads=1",
                ])
                .spawn()
                .unwrap();
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(20);
            loop {
                if let Some(status) = child.try_wait().unwrap() {
                    assert!(
                        status.success(),
                        "detached retirement child failed: {status}"
                    );
                    return;
                }
                if std::time::Instant::now() >= deadline {
                    child.kill().unwrap();
                    let _ = child.wait();
                    panic!("detached retirement child exceeded 20 seconds");
                }
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
        }
        for threads in [1, 4] {
            let completed = Arc::new(AtomicUsize::new(0));
            let executor = Arc::new(observed(threads, None, Arc::clone(&completed)).unwrap());
            let worker_owner = Arc::clone(&executor);
            let (release, released) = std::sync::mpsc::channel();
            let (finished, finish) = std::sync::mpsc::channel();
            executor.team().install(move || {
                // Intentionally violate the synchronous callback contract
                // to verify defensive final-owner retirement containment.
                rayon::spawn(move || {
                    released
                        .recv_timeout(std::time::Duration::from_secs(5))
                        .unwrap();
                    assert!(
                        worker_owner
                            .pool
                            .as_ref()
                            .unwrap()
                            .current_thread_index()
                            .is_some()
                    );
                    drop(worker_owner);
                    finished.send(()).unwrap();
                });
            });
            drop(executor);
            release.send(()).unwrap();
            finish
                .recv_timeout(std::time::Duration::from_secs(5))
                .unwrap();
            // Retirement is asynchronous only on this unsupported path. Prove
            // all actual workers eventually finish their TLS destructors.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            while completed.load(Ordering::SeqCst) != threads {
                assert!(
                    std::time::Instant::now() < deadline,
                    "workers did not retire"
                );
                std::thread::sleep(std::time::Duration::from_millis(1));
            }
        }
    }

    #[test]
    fn partial_startup_joins_before_returning_error() {
        for failed_index in [0, 1, 3] {
            let completed = Arc::new(AtomicUsize::new(0));
            assert!(observed(4, Some(failed_index), Arc::clone(&completed)).is_err());
            assert_eq!(completed.load(Ordering::SeqCst), failed_index);
        }
    }

    #[test]
    fn scoped_panic_unwinds_and_joins_without_second_panic() {
        let completed = Arc::new(AtomicUsize::new(0));
        let result = std::panic::catch_unwind({
            let completed = Arc::clone(&completed);
            move || {
                let executor = observed(4, None, completed).unwrap();
                executor.team().install(|| panic!("scoped kernel panic"));
            }
        });
        assert!(result.is_err());
        assert_eq!(completed.load(Ordering::SeqCst), 4);
    }

    #[test]
    fn join_error_does_not_double_panic() {
        let result = std::panic::catch_unwind(|| {
            let mut workers = JoinedWorkers::new(1).unwrap();
            workers
                .handles
                .push(std::thread::spawn(|| panic!("worker panic")));
            panic!("caller panic");
        });
        assert!(result.is_err());
    }
}
