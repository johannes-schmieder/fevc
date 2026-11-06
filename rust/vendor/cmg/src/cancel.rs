//! Cooperative cancellation for bounded construction, application and PCG.
//!
//! Construction kernels count conservative primitive units (arithmetic, record
//! visits, comparisons and moves) while they work. The meter polls the caller's
//! sticky, allocation-free cancellation query whenever the running count crosses
//! a fixed threshold and before every allocation. Work between polls is therefore
//! bounded by the threshold plus one bounded operation, regardless of row, group,
//! level or factor-block boundaries. Units are a polling cadence, not an
//! admission budget: nothing is charged, refused or reported. Allocator,
//! operating-system and cleanup latency are not bounded.

use crate::CmgError;

/// Counted units between two cancellation polls on the serial meter.
pub(crate) const POLL_UNITS: u32 = 1 << 16;

/// A caller-owned, sticky cancellation query. It must be cheap, allocation-free
/// and remain true once it has returned true for the current construction.
pub type Cancellation<'a> = &'a (dyn Fn() -> bool + Sync);

pub(crate) struct CancelMeter<'a> {
    cancel: Cancellation<'a>,
    pending: u32,
    threshold: u32,
}

impl<'a> CancelMeter<'a> {
    pub(crate) fn new(cancel: Cancellation<'a>) -> Self {
        Self {
            cancel,
            pending: 0,
            threshold: POLL_UNITS,
        }
    }

    /// Poll at every work and allocation boundary. Tests use this to cancel
    /// at each bounded operation of a small fixture.
    #[cfg(test)]
    pub(crate) fn every_operation(cancel: Cancellation<'a>) -> Self {
        Self {
            cancel,
            pending: 0,
            threshold: 1,
        }
    }

    pub(crate) fn cancelled(phase: &'static str) -> CmgError {
        CmgError::Cancelled { phase }
    }

    #[inline]
    fn poll(&mut self, phase: &'static str) -> Result<(), CmgError> {
        self.pending = 0;
        #[cfg(test)]
        let _probe = probe::enter(phase);
        if (self.cancel)() {
            Err(Self::cancelled(phase))
        } else {
            Ok(())
        }
    }

    /// Count one bounded operation of at most 1024 primitive units.
    #[inline]
    pub(crate) fn work(&mut self, phase: &'static str, units: u16) -> Result<(), CmgError> {
        if units == 0 || units > 1024 {
            return Err(CmgError::InvalidHierarchy {
                context: "invalid bounded construction chunk",
            });
        }
        self.pending += u32::from(units);
        if self.pending >= self.threshold {
            self.poll(phase)
        } else {
            Ok(())
        }
    }

    /// Poll before an allocation. The requested size is admitted separately
    /// through the caller's closed-form memory bound.
    #[inline]
    pub(crate) fn allocation(
        &mut self,
        phase: &'static str,
        _bytes: usize,
    ) -> Result<(), CmgError> {
        self.poll(phase)
    }

    /// Poll unconditionally, for example before returning a completed object.
    pub(crate) fn checkpoint(&mut self, phase: &'static str) -> Result<(), CmgError> {
        self.poll(phase)
    }

    pub(crate) fn try_capacity<T>(
        &mut self,
        count: usize,
        phase: &'static str,
    ) -> Result<Vec<T>, CmgError> {
        self.work(phase, 16)?;
        self.allocation(phase, crate::storage::bytes::<T>(count)?)?;
        crate::storage::try_capacity(count, phase)
    }

    // Allocation-free heapsort. Each comparison and bounded swap is counted.
    // Callers use constant-work comparators and scalar/record Copy payloads.
    pub(crate) fn sort_by<T: Copy>(
        &mut self,
        values: &mut [T],
        compare: impl FnMut(&T, &T) -> core::cmp::Ordering,
        phase: &'static str,
    ) -> Result<(), CmgError> {
        heapsort(self, values, compare, phase)
    }
}

/// Test-only probe: the phase of the [`CancelMeter`] poll whose query runs on
/// this thread, so a witness can latch the caller's sticky query on a named
/// phase instead of a schedule-dependent query ordinal. Lane polls
/// ([`LanePoll`]) leave it unset. Allocation-free; restored on return and
/// unwind. It only observes: the production query still decides.
#[cfg(test)]
pub(crate) mod probe {
    use core::cell::Cell;

    std::thread_local! {
        static PHASE: Cell<Option<&'static str>> = const { Cell::new(None) };
    }

    pub(super) struct Polling(Option<&'static str>);

    pub(super) fn enter(phase: &'static str) -> Polling {
        Polling(PHASE.with(|current| current.replace(Some(phase))))
    }

    impl Drop for Polling {
        fn drop(&mut self) {
            PHASE.with(|current| current.set(self.0));
        }
    }

    /// The phase of the meter poll querying on this thread, if any.
    #[cfg(feature = "parallel")]
    pub(crate) fn phase() -> Option<&'static str> {
        PHASE.with(Cell::get)
    }
}

/// Allocation-free heapsort counted through any progress: every sift step
/// (two comparisons and one swap) and every extraction is one bounded
/// operation. With a total comparator the result equals any other sort.
pub(crate) fn heapsort<T: Copy, P: Progress>(
    progress: &mut P,
    values: &mut [T],
    mut compare: impl FnMut(&T, &T) -> core::cmp::Ordering,
    phase: &'static str,
) -> Result<(), CmgError> {
    fn sift<T: Copy, P: Progress>(
        progress: &mut P,
        values: &mut [T],
        mut root: usize,
        end: usize,
        compare: &mut impl FnMut(&T, &T) -> core::cmp::Ordering,
        phase: &'static str,
    ) -> Result<(), CmgError> {
        loop {
            progress.advance(phase, 64)?;
            // root < end and slice lengths cannot exceed isize::MAX bytes.
            let child = root * 2 + 1;
            if child >= end {
                return Ok(());
            }
            let mut largest = child;
            if child + 1 < end && compare(&values[child], &values[child + 1]).is_lt() {
                largest += 1;
            }
            if !compare(&values[root], &values[largest]).is_lt() {
                return Ok(());
            }
            values.swap(root, largest);
            root = largest;
        }
    }
    progress.advance(phase, 16)?;
    for root in (0..values.len() / 2).rev() {
        sift(progress, values, root, values.len(), &mut compare, phase)?;
    }
    for end in (1..values.len()).rev() {
        progress.advance(phase, 16)?;
        values.swap(0, end);
        sift(progress, values, 0, end, &mut compare, phase)?;
    }
    Ok(())
}

/// Polling policy of one construction or preparation kernel.
///
/// Every kernel has one implementation generic over its meter. [`Unmetered`]
/// is zero-sized and compiles every hook away, preserving the ordinary
/// allocation, initialization and sorting code; [`CancelMeter`] counts the
/// same bounded operations and polls at the fixed cadence and before every
/// allocation. Arithmetic and storage layouts never depend on the meter.
pub(crate) trait Meter {
    /// Whether this meter polls (static specialization switch).
    const POLLS: bool;

    /// Count one bounded operation of 1 through 1024 primitive units.
    fn work(&mut self, phase: &'static str, units: u16) -> Result<(), CmgError>;

    /// Poll before an allocation of `bytes` requested bytes.
    fn allocation(&mut self, phase: &'static str, bytes: usize) -> Result<(), CmgError>;

    /// Poll unconditionally, for example before publishing a completed object.
    fn checkpoint(&mut self, phase: &'static str) -> Result<(), CmgError>;

    /// Exact-capacity storage after a checked layout.
    fn try_capacity<T>(&mut self, count: usize, phase: &'static str) -> Result<Vec<T>, CmgError>;

    /// Construction fill: one counted operation per value when polling.
    fn try_filled<T: Clone>(
        &mut self,
        count: usize,
        value: T,
        phase: &'static str,
    ) -> Result<Vec<T>, CmgError>;

    /// Exact-capacity copy of a slice.
    fn copy_slice<T: Copy>(
        &mut self,
        values: &[T],
        phase: &'static str,
    ) -> Result<Vec<T>, CmgError>;

    /// Sort with a total-order comparator. Polling meters use counted
    /// allocation-free heapsort; the ordinary path uses the standard unstable
    /// sort. Comparators are total, so both produce identical sequences.
    fn sort_by<T: Copy>(
        &mut self,
        values: &mut [T],
        compare: impl FnMut(&T, &T) -> core::cmp::Ordering,
        phase: &'static str,
    ) -> Result<(), CmgError>;

    /// The sticky query, for parallel lanes that poll independently.
    #[cfg(feature = "parallel")]
    fn query(&self) -> Option<Cancellation<'_>>;

    /// Count `units` primitive units in bounded steps of at most 1024.
    #[inline(always)]
    fn items(&mut self, phase: &'static str, mut units: usize) -> Result<(), CmgError> {
        if Self::POLLS {
            while units > 0 {
                let step = units.min(1024);
                // step is in 1..=1024.
                self.work(phase, step as u16)?;
                units -= step;
            }
        }
        Ok(())
    }

    /// After joined parallel lanes: poll once when polling, so a lane that
    /// stopped early on the sticky query is reported as cancellation.
    #[cfg(feature = "parallel")]
    #[inline(always)]
    fn joined(&mut self, phase: &'static str) -> Result<(), CmgError> {
        if Self::POLLS {
            self.checkpoint(phase)
        } else {
            Ok(())
        }
    }
}

/// Progress inside one traversal: a row, a factor row, a reduction leaf.
///
/// Serial kernels pass their [`Meter`]; parallel lanes pass a [`LanePoll`].
/// A kernel counts its traversal while it runs, in segments of at most 1024
/// units (see [`chunked`]), never once before or after an unbounded loop.
pub(crate) trait Progress {
    /// Whether traversals are split into counted segments.
    const POLLS: bool;

    /// Count `units` primitive units of work just performed.
    fn advance(&mut self, phase: &'static str, units: usize) -> Result<(), CmgError>;
}

impl<M: Meter> Progress for M {
    const POLLS: bool = M::POLLS;

    #[inline(always)]
    fn advance(&mut self, phase: &'static str, units: usize) -> Result<(), CmgError> {
        self.items(phase, units)
    }
}

/// One parallel lane's progress: counts units and polls the sticky query each
/// [`POLL_UNITS`] counted units. A true query stops the lane with
/// `Cancelled`; the joining coordinator then reports the cancellation and
/// discards the lane's outputs. Lanes without a query only segment.
#[cfg(feature = "parallel")]
pub(crate) struct LanePoll<'a> {
    query: Option<Cancellation<'a>>,
    pending: usize,
    stopped: bool,
}

#[cfg(feature = "parallel")]
impl<'a> LanePoll<'a> {
    pub(crate) fn new(query: Option<Cancellation<'a>>) -> Self {
        Self {
            query,
            pending: 0,
            stopped: false,
        }
    }

    /// Whether this lane has observed cancellation. A stopped lane's every
    /// later advance fails without querying again.
    pub(crate) fn stopped(&self) -> bool {
        self.stopped
    }
}

#[cfg(feature = "parallel")]
impl Progress for LanePoll<'_> {
    const POLLS: bool = true;

    #[inline]
    fn advance(&mut self, phase: &'static str, units: usize) -> Result<(), CmgError> {
        if self.stopped {
            return Err(CancelMeter::cancelled(phase));
        }
        let Some(cancel) = self.query else {
            return Ok(());
        };
        self.pending = self.pending.saturating_add(units);
        if self.pending >= POLL_UNITS as usize {
            self.pending = 0;
            if cancel() {
                self.stopped = true;
                return Err(CancelMeter::cancelled(phase));
            }
        }
        Ok(())
    }
}

/// Run `body` over `0..len` in consecutive segments whose `weight`-unit items
/// total at most 1024 units, counting every segment after it runs. Without
/// polling, `body` runs once over the whole range, so an ordinary loop is
/// unchanged. Segmenting only splits a forward loop; accumulators carried
/// across segments keep their operation order, so arithmetic never changes.
#[inline(always)]
pub(crate) fn chunked<P: Progress>(
    progress: &mut P,
    phase: &'static str,
    len: usize,
    weight: usize,
    mut body: impl FnMut(core::ops::Range<usize>) -> Result<(), CmgError>,
) -> Result<(), CmgError> {
    if !P::POLLS {
        return body(0..len);
    }
    let weight = weight.max(1);
    let step = (1024 / weight).max(1);
    let mut start = 0;
    while start < len {
        let end = len.min(start + step);
        body(start..end)?;
        progress.advance(phase, (end - start) * weight)?;
        start = end;
    }
    Ok(())
}

/// The lane tiles of one `columns`-wide row at `weight` units per lane: the
/// whole row without polling, or while one row fits a segment; otherwise
/// consecutive tiles of at most `1024 / weight` lanes. Lanes are independent,
/// so a tile only splits a row's lane loop and no lane's arithmetic changes.
#[inline(always)]
pub(crate) fn lane_tiles<P: Progress>(
    columns: usize,
    weight: usize,
) -> impl Iterator<Item = core::ops::Range<usize>> {
    let tile = if P::POLLS {
        (1024 / weight.max(1)).max(1)
    } else {
        columns.max(1)
    };
    (0..columns)
        .step_by(tile)
        .map(move |start| start..columns.min(start + tile))
}

/// Run `body(rows, lanes)` over `rows` rows of `columns` lanes at `weight`
/// units per lane, covering rows in order and each row's lanes in ascending
/// order. Rows narrow enough to share a segment arrive as whole segments of
/// rows with every lane (`0..columns`), exactly as in [`chunked`], so a kernel
/// keeps one tight loop per segment; a row wider than one segment arrives as
/// one row per [`lane_tiles`] tile, each counted after it runs, so the work
/// between counts stays bounded at any batch width. Without polling every row
/// arrives in one call.
#[inline(always)]
pub(crate) fn row_lanes<P: Progress>(
    progress: &mut P,
    phase: &'static str,
    rows: usize,
    columns: usize,
    weight: usize,
    mut body: impl FnMut(core::ops::Range<usize>, core::ops::Range<usize>) -> Result<(), CmgError>,
) -> Result<(), CmgError> {
    let weight = weight.max(1);
    let per_row = columns.saturating_mul(weight);
    if !P::POLLS || per_row <= 1024 {
        // Inlining the segment body keeps a constant width visible to the
        // kernel, so narrow rows keep their specialized loops when polling.
        return chunked(
            progress,
            phase,
            rows,
            per_row,
            #[inline(always)]
            |range| body(range, 0..columns),
        );
    }
    for row in 0..rows {
        for lanes in lane_tiles::<P>(columns, weight) {
            let units = lanes.len() * weight;
            body(row..row + 1, lanes)?;
            progress.advance(phase, units)?;
        }
    }
    Ok(())
}

/// Zero `values` in counted chunks; explicit fill work is polled like any other.
#[inline(always)]
pub(crate) fn zero<M: Meter>(
    meter: &mut M,
    phase: &'static str,
    values: &mut [f64],
) -> Result<(), CmgError> {
    chunked(meter, phase, values.len(), 1, |range| {
        values[range].fill(0.0);
        Ok(())
    })
}

/// The ordinary, non-polling meter.
pub(crate) struct Unmetered;

impl Meter for Unmetered {
    const POLLS: bool = false;

    #[inline(always)]
    fn work(&mut self, _: &'static str, _: u16) -> Result<(), CmgError> {
        Ok(())
    }

    #[inline(always)]
    fn allocation(&mut self, _: &'static str, _: usize) -> Result<(), CmgError> {
        Ok(())
    }

    #[inline(always)]
    fn checkpoint(&mut self, _: &'static str) -> Result<(), CmgError> {
        Ok(())
    }

    #[inline]
    fn try_capacity<T>(&mut self, count: usize, phase: &'static str) -> Result<Vec<T>, CmgError> {
        crate::storage::try_capacity(count, phase)
    }

    #[inline]
    fn try_filled<T: Clone>(
        &mut self,
        count: usize,
        value: T,
        phase: &'static str,
    ) -> Result<Vec<T>, CmgError> {
        crate::storage::try_filled(count, value, phase)
    }

    #[inline]
    fn copy_slice<T: Copy>(
        &mut self,
        values: &[T],
        phase: &'static str,
    ) -> Result<Vec<T>, CmgError> {
        let mut result = crate::storage::try_capacity(values.len(), phase)?;
        result.extend_from_slice(values);
        Ok(result)
    }

    #[inline]
    fn sort_by<T: Copy>(
        &mut self,
        values: &mut [T],
        compare: impl FnMut(&T, &T) -> core::cmp::Ordering,
        _: &'static str,
    ) -> Result<(), CmgError> {
        values.sort_unstable_by(compare);
        Ok(())
    }

    #[cfg(feature = "parallel")]
    #[inline(always)]
    fn query(&self) -> Option<Cancellation<'_>> {
        None
    }
}

impl Meter for CancelMeter<'_> {
    const POLLS: bool = true;

    #[inline]
    fn work(&mut self, phase: &'static str, units: u16) -> Result<(), CmgError> {
        CancelMeter::work(self, phase, units)
    }

    #[inline]
    fn allocation(&mut self, phase: &'static str, bytes: usize) -> Result<(), CmgError> {
        CancelMeter::allocation(self, phase, bytes)
    }

    fn checkpoint(&mut self, phase: &'static str) -> Result<(), CmgError> {
        CancelMeter::checkpoint(self, phase)
    }

    fn try_capacity<T>(&mut self, count: usize, phase: &'static str) -> Result<Vec<T>, CmgError> {
        CancelMeter::try_capacity(self, count, phase)
    }

    fn try_filled<T: Clone>(
        &mut self,
        count: usize,
        value: T,
        phase: &'static str,
    ) -> Result<Vec<T>, CmgError> {
        let mut result = CancelMeter::try_capacity(self, count, phase)?;
        // Eight units per value, counted in segments of 128 values.
        chunked(self, phase, count, 8, |range| {
            result.resize(result.len() + range.len(), value.clone());
            Ok(())
        })?;
        Ok(result)
    }

    fn copy_slice<T: Copy>(
        &mut self,
        values: &[T],
        phase: &'static str,
    ) -> Result<Vec<T>, CmgError> {
        let mut result = CancelMeter::try_capacity(self, values.len(), phase)?;
        // Eight units per value, counted in segments of 128 values.
        chunked(self, phase, values.len(), 8, |range| {
            result.extend_from_slice(&values[range]);
            Ok(())
        })?;
        Ok(result)
    }

    fn sort_by<T: Copy>(
        &mut self,
        values: &mut [T],
        compare: impl FnMut(&T, &T) -> core::cmp::Ordering,
        phase: &'static str,
    ) -> Result<(), CmgError> {
        CancelMeter::sort_by(self, values, compare, phase)
    }

    #[cfg(feature = "parallel")]
    #[inline]
    fn query(&self) -> Option<Cancellation<'_>> {
        Some(self.cancel)
    }
}

// Workspace preparation fills large zeroed banks. Its polling meter counts
// them in chunks rather than per value; the ordinary path is one reserve and
// one fill with the caller's allocation-failure context.

/// Values initialized per counted workspace operation (8 units each).
const FILL_CHUNK: usize = 128;

/// Exact-capacity workspace storage; allocator failure reports `context`.
pub(crate) fn try_reserved<T, M: Meter>(
    meter: &mut M,
    len: usize,
    context: &'static str,
) -> Result<Vec<T>, CmgError> {
    meter.allocation(context, len.saturating_mul(core::mem::size_of::<T>()))?;
    let mut values = Vec::new();
    values
        .try_reserve_exact(len)
        .map_err(|_| CmgError::AllocationFailed { context })?;
    Ok(values)
}

/// `len` copies of `value` in exact-capacity workspace storage.
pub(crate) fn try_filled<T: Clone, M: Meter>(
    meter: &mut M,
    len: usize,
    value: T,
    context: &'static str,
) -> Result<Vec<T>, CmgError> {
    let mut values = try_reserved(meter, len, context)?;
    if M::POLLS {
        while values.len() < len {
            let next = (len - values.len()).min(FILL_CHUNK);
            // next <= 128, so the operation is at most 1024 units.
            meter.work(context, 8 * next as u16)?;
            values.resize(values.len() + next, value.clone());
        }
    } else {
        values.resize(len, value);
    }
    Ok(values)
}

/// Test support: a polling meter recording every counted operation, which
/// cancels at a chosen one-based operation. `max_units` is the largest
/// single count a kernel reported before splitting, so a segment wider than
/// one bounded operation is visible. Kernels write partial results before the
/// cancellation, so tests observe how much of a traversal ran.
#[cfg(test)]
#[derive(Default)]
pub(crate) struct Recorder {
    pub(crate) advances: usize,
    pub(crate) units: usize,
    pub(crate) max_units: usize,
    pub(crate) stop_at: Option<usize>,
}

#[cfg(test)]
impl Meter for Recorder {
    const POLLS: bool = true;

    fn work(&mut self, phase: &'static str, units: u16) -> Result<(), CmgError> {
        self.advances += 1;
        self.units += usize::from(units);
        if self.stop_at == Some(self.advances) {
            Err(CancelMeter::cancelled(phase))
        } else {
            Ok(())
        }
    }

    fn items(&mut self, phase: &'static str, mut units: usize) -> Result<(), CmgError> {
        self.max_units = self.max_units.max(units);
        while units > 0 {
            let step = units.min(1024);
            self.work(phase, step as u16)?;
            units -= step;
        }
        Ok(())
    }

    fn allocation(&mut self, _: &'static str, _: usize) -> Result<(), CmgError> {
        Ok(())
    }

    fn checkpoint(&mut self, _: &'static str) -> Result<(), CmgError> {
        Ok(())
    }

    fn try_capacity<T>(&mut self, count: usize, phase: &'static str) -> Result<Vec<T>, CmgError> {
        crate::storage::try_capacity(count, phase)
    }

    fn try_filled<T: Clone>(
        &mut self,
        count: usize,
        value: T,
        phase: &'static str,
    ) -> Result<Vec<T>, CmgError> {
        crate::storage::try_filled(count, value, phase)
    }

    fn copy_slice<T: Copy>(
        &mut self,
        values: &[T],
        phase: &'static str,
    ) -> Result<Vec<T>, CmgError> {
        let mut result = crate::storage::try_capacity(values.len(), phase)?;
        result.extend_from_slice(values);
        Ok(result)
    }

    fn sort_by<T: Copy>(
        &mut self,
        values: &mut [T],
        compare: impl FnMut(&T, &T) -> core::cmp::Ordering,
        phase: &'static str,
    ) -> Result<(), CmgError> {
        heapsort(self, values, compare, phase)
    }

    #[cfg(feature = "parallel")]
    fn query(&self) -> Option<Cancellation<'_>> {
        None
    }
}

/// Test support: count polls and cancel exactly at one zero-based poll.
#[cfg(test)]
pub(crate) struct StopAt {
    polls: std::sync::atomic::AtomicUsize,
    stop: Option<usize>,
}

#[cfg(test)]
impl StopAt {
    pub(crate) fn never() -> Self {
        Self::at(None)
    }

    pub(crate) fn at(stop: Option<usize>) -> Self {
        Self {
            polls: std::sync::atomic::AtomicUsize::new(0),
            stop,
        }
    }

    pub(crate) fn query(&self) -> bool {
        let index = self
            .polls
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.stop.is_some_and(|stop| index >= stop)
    }

    pub(crate) fn polls(&self) -> usize {
        self.polls.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn polls_at_threshold_and_allocations_and_reports_the_phase() {
        let stop = StopAt::never();
        let query = || stop.query();
        let mut meter = CancelMeter::new(&query);
        for _ in 0..(POLL_UNITS / 1024 - 1) {
            meter.work("phase", 1024).unwrap();
        }
        assert_eq!(stop.polls(), 0);
        meter.work("phase", 1024).unwrap();
        assert_eq!(stop.polls(), 1);
        meter.allocation("phase", 8).unwrap();
        assert_eq!(stop.polls(), 2);
        let stop = StopAt::at(Some(0));
        let query = || stop.query();
        let mut meter = CancelMeter::new(&query);
        assert_eq!(
            meter.allocation("alloc", 0),
            Err(CmgError::Cancelled { phase: "alloc" })
        );
    }

    #[test]
    fn heapsort_matches_the_standard_sort_and_stops_inside_one_long_run() {
        let mut values: Vec<u32> = (0..5000).map(|i| (i * 7919) % 5003).collect();
        let mut expected = values.clone();
        expected.sort_unstable();
        let never = || false;
        CancelMeter::new(&never)
            .sort_by(&mut values, u32::cmp, "sort")
            .unwrap();
        assert_eq!(values, expected);
        // A single long sort is still interruptible between bounded steps.
        let stop = StopAt::at(Some(1));
        let query = || stop.query();
        let mut values: Vec<u32> = (0..50_000).rev().collect();
        assert_eq!(
            CancelMeter::new(&query).sort_by(&mut values, u32::cmp, "sort"),
            Err(CmgError::Cancelled { phase: "sort" })
        );
        assert_eq!(stop.polls(), 2);
    }
}
