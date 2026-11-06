//! The one canonical edge sort: an in-place introsort by the total raw-edge
//! order, counted and polled, with no allocation. Serial construction sorts
//! on the caller's meter; with a team, partitions fork within the worker
//! ceiling and each lane polls independently.

use super::{Edge, compare_raw_edges};
use crate::CmgError;
#[cfg(feature = "parallel")]
use crate::Team;
#[cfg(feature = "parallel")]
use crate::cancel::LanePoll;
use crate::cancel::{Meter, Progress, heapsort};
#[cfg(feature = "parallel")]
use crate::execution::Lanes;
use core::cmp::Ordering;

/// Ranges at most this long are insertion-sorted.
const SMALL: usize = 16;
/// Partitions longer than this take Tukey's ninther as pivot (Bentley and
/// McIlroy, "Engineering a Sort Function", 1993); shorter ones take the
/// median of their first, middle and last records.
const NINTHER_MIN: usize = 41;
/// Counted units of pivot selection: 16 per comparison (as a partition
/// step), at most 3 comparisons for a median of three and 12 for a ninther.
const MEDIAN_UNITS: usize = 3 * 16;
const NINTHER_UNITS: usize = 4 * MEDIAN_UNITS;
/// Partitions shorter than this are not forked.
#[cfg(feature = "parallel")]
const FORK_MIN: usize = 1 << 14;
#[cfg(feature = "parallel")]
const PHASE: &str = "team edge sort";

/// Test-only probe: the partition this thread is running, as (classified
/// records, length), so a witness can fire cancellation at a poll inside
/// actual partition work and see what that partition leaves untouched.
#[cfg(all(test, feature = "parallel"))]
mod probe {
    use core::cell::Cell;

    std::thread_local! {
        static PARTITION: Cell<(usize, usize)> = const { Cell::new((0, 0)) };
    }

    pub(super) struct Partitioning((usize, usize));

    pub(super) fn enter(len: usize) -> Partitioning {
        Partitioning(PARTITION.with(|state| state.replace((0, len))))
    }

    impl Drop for Partitioning {
        fn drop(&mut self) {
            PARTITION.with(|state| state.set(self.0));
        }
    }

    pub(super) fn step() {
        PARTITION.with(|state| state.set((state.get().0 + 1, state.get().1)));
    }

    /// Records the running partition has not classified yet, if any runs.
    pub(super) fn remaining() -> Option<usize> {
        let (classified, len) = PARTITION.with(Cell::get);
        (len > 0).then(|| len - classified)
    }
}

/// Test-only count of depth-limit heapsort fallbacks and their records on
/// this thread (serial sorts), for operation-count regression witnesses.
#[cfg(test)]
mod fallbacks {
    use core::cell::Cell;

    std::thread_local! {
        static FALLBACKS: Cell<(usize, usize)> = const { Cell::new((0, 0)) };
    }

    pub(super) fn record(len: usize) {
        FALLBACKS.with(|state| state.set((state.get().0 + 1, state.get().1 + len)));
    }

    /// (fallbacks, records) since the last call, then reset.
    pub(super) fn take() -> (usize, usize) {
        FALLBACKS.with(|state| state.replace((0, 0)))
    }
}

fn depth_limit(len: usize) -> usize {
    2 * (usize::BITS - len.leading_zeros()) as usize
}

/// Sort `values` by the total raw-edge order in place on the calling lane: an
/// introsort with three-way partitions around a ninther pivot above 40
/// records and a median-of-three pivot below (see [`pivot`]), a heapsort
/// fallback at depth `2 log2(n)` and insertion sort below 17 records. Every
/// pivot selection, partition step, insertion step and heap step is counted on
/// `meter`: pivot selection after it runs (at most 192 units), a partition
/// every 64 steps (1,024 units), and the
/// meter queries every 65,536 units (about 4,096 steps), inside a partition
/// too. The comparator is total and equal records are identical, so the order
/// equals every other sort's, independent of pivots. Nothing is allocated.
pub(crate) fn sort_serial<M: Meter>(
    values: &mut [Edge],
    meter: &mut M,
    phase: &'static str,
) -> Result<(), CmgError> {
    serial_range(
        values,
        depth_limit(values.len()),
        meter,
        phase,
        compare_raw_edges,
    )
}

/// The serial introsort over any total order on edges. Production passes the
/// canonical comparator; tests also drive the identical algorithm with an
/// adaptive adversary to construct inputs that defeat its pivot rule.
fn serial_range<P: Progress, C: Fn(&Edge, &Edge) -> Ordering + Copy>(
    mut values: &mut [Edge],
    mut depth: usize,
    poll: &mut P,
    phase: &'static str,
    compare: C,
) -> Result<(), CmgError> {
    loop {
        if values.len() <= SMALL {
            return insertion(values, poll, phase, compare);
        }
        if depth == 0 {
            #[cfg(test)]
            fallbacks::record(values.len());
            return heapsort(poll, values, compare, phase);
        }
        depth -= 1;
        let (less, greater) = partition(values, poll, phase, compare)?;
        let (left, rest) = values.split_at_mut(less);
        let right = &mut rest[greater - less..];
        // Recurse into the shorter side and continue with the longer one.
        if left.len() < right.len() {
            serial_range(left, depth, poll, phase, compare)?;
            values = right;
        } else {
            serial_range(right, depth, poll, phase, compare)?;
            values = left;
        }
    }
}

/// The same sort on a team: partitions of at least 16,384 records fork while
/// lanes remain, so at most the worker ceiling of lanes run, and every started
/// lane joins before the meter's final poll. Each logical node of the
/// partition tree (a partition, an insertion leaf or a heapsort fallback)
/// queries once on entry with its own lane counter and then at the fixed
/// cadence (every 65,536 counted units, about 4,096 partition steps). The tree
/// depends only on the input, so a successful run's query total and each
/// node's logical checkpoints are the same at every worker ceiling, and work
/// between two queries of a lane stays bounded. The chronological order of
/// queries across lanes, and where a cancellation stops, are not fixed.
#[cfg(feature = "parallel")]
pub(crate) fn sort<M: Meter>(
    values: &mut [Edge],
    team: Team<'_>,
    meter: &mut M,
) -> Result<(), CmgError> {
    let query = meter.query();
    let depth = depth_limit(values.len());
    let sorted = team.with_lanes(|lanes| team_range(values, depth, lanes, query));
    meter.joined(PHASE)?;
    sorted
}

#[cfg(feature = "parallel")]
fn team_range(
    mut values: &mut [Edge],
    mut depth: usize,
    lanes: Lanes<'_>,
    query: Option<crate::Cancellation<'_>>,
) -> Result<(), CmgError> {
    loop {
        let mut node = LanePoll::new(query);
        node.advance(PHASE, crate::cancel::POLL_UNITS as usize)?;
        if values.len() <= SMALL {
            return insertion(values, &mut node, PHASE, compare_raw_edges);
        }
        if depth == 0 {
            return heapsort(&mut node, values, compare_raw_edges, PHASE);
        }
        depth -= 1;
        let (less, greater) = partition(values, &mut node, PHASE, compare_raw_edges)?;
        let total = values.len();
        let (left, rest) = values.split_at_mut(less);
        let right = &mut rest[greater - less..];
        if lanes.count() > 1 && total >= FORK_MIN {
            let (a, b) = lanes.join(
                |lanes| team_range(left, depth, lanes, query),
                |lanes| team_range(right, depth, lanes, query),
            );
            return a.and(b);
        }
        // Recurse into the shorter side and continue with the longer one.
        if left.len() < right.len() {
            team_range(left, depth, lanes.single(), query)?;
            values = right;
        } else {
            team_range(right, depth, lanes.single(), query)?;
            values = left;
        }
    }
}

fn insertion<P: Progress, C: Fn(&Edge, &Edge) -> Ordering>(
    values: &mut [Edge],
    poll: &mut P,
    phase: &'static str,
    compare: C,
) -> Result<(), CmgError> {
    for index in 1..values.len() {
        let mut current = index;
        while current > 0 && compare(&values[current], &values[current - 1]).is_lt() {
            values.swap(current, current - 1);
            current -= 1;
        }
        poll.advance(phase, 16 * (index - current + 1))?;
    }
    Ok(())
}

/// The median of three records under a total order, in at most three
/// comparisons. Equal records are identical, so the value is unique.
fn median_of_three(a: Edge, b: Edge, c: Edge, compare: impl Fn(&Edge, &Edge) -> Ordering) -> Edge {
    if compare(&a, &b).is_lt() {
        if compare(&b, &c).is_lt() {
            b
        } else if compare(&a, &c).is_lt() {
            c
        } else {
            a
        }
    } else if compare(&a, &c).is_lt() {
        a
    } else if compare(&b, &c).is_lt() {
        c
    } else {
        b
    }
}

/// The pivot of a partition of at least 17 records and its counted units.
/// Up to 40 records: the median of the first, middle and last records. Above:
/// Tukey's ninther, the median of the medians of three evenly spaced triples
/// around the first, middle and last records, with spacing `len / 8`.
fn pivot(values: &[Edge], compare: impl Fn(&Edge, &Edge) -> Ordering + Copy) -> (Edge, usize) {
    let last = values.len() - 1;
    let middle = last / 2;
    if values.len() < NINTHER_MIN {
        let pivot = median_of_three(values[0], values[middle], values[last], compare);
        return (pivot, MEDIAN_UNITS);
    }
    // len >= 41 gives s >= 5 and nine distinct ordered positions.
    let s = values.len() / 8;
    let at =
        |a: usize, b: usize, c: usize| median_of_three(values[a], values[b], values[c], compare);
    let pivot = median_of_three(
        at(0, s, 2 * s),
        at(middle - s, middle, middle + s),
        at(last - 2 * s, last - s, last),
        compare,
    );
    (pivot, NINTHER_UNITS)
}

/// Dijkstra three-way partition around [`pivot`]. Pivot selection is counted
/// after it runs (at most 12 comparisons, 192 units), then every 64
/// classified records. Returns `(less, greater)`: `values[..less] < pivot`,
/// `values[less..greater] == pivot` and `values[greater..] > pivot`.
fn partition<P: Progress, C: Fn(&Edge, &Edge) -> Ordering + Copy>(
    values: &mut [Edge],
    poll: &mut P,
    phase: &'static str,
    compare: C,
) -> Result<(usize, usize), CmgError> {
    #[cfg(all(test, feature = "parallel"))]
    let _probe = probe::enter(values.len());
    let (pivot, units) = pivot(values, compare);
    poll.advance(phase, units)?;
    let (mut less, mut index, mut greater) = (0, 0, values.len());
    let mut steps = 0;
    while index < greater {
        #[cfg(all(test, feature = "parallel"))]
        {
            crate::execution::visits::visit_within(core::ptr::from_ref(&values[index]).cast());
            probe::step();
        }
        match compare(&values[index], &pivot) {
            Ordering::Less => {
                values.swap(less, index);
                less += 1;
                index += 1;
            }
            Ordering::Greater => {
                greater -= 1;
                values.swap(index, greater);
            }
            Ordering::Equal => index += 1,
        }
        steps += 1;
        if steps == 64 {
            poll.advance(phase, 64 * 16)?;
            steps = 0;
        }
    }
    poll.advance(phase, 16 * steps.max(1))?;
    Ok((less, greater))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cancel::{CancelMeter, Unmetered};

    /// Edges with a deterministic key pattern and `duplicates` copies of each
    /// key run; weights cycle through ties, signed zeros and a subnormal so the
    /// total weight order is exercised inside equal endpoint groups.
    fn edges(count: usize, duplicates: usize) -> Vec<Edge> {
        let weights = [1.0, -0.0, 0.0, f64::from_bits(1), 3.5, 1.0, 1e300];
        (0..count)
            .map(|i| {
                let j = (i / duplicates) * 2_654_435_761 % 100_003;
                let u = (j % 997) as u32;
                Edge::from_compact_parts(u, u + 1 + (j % 13) as u32, weights[i % weights.len()])
            })
            .collect()
    }

    /// Endpoints and weight bits: `Edge`'s `PartialEq` equates +0 and -0,
    /// so every order witness compares these instead.
    fn bits(values: &[Edge]) -> Vec<(usize, usize, u64)> {
        values
            .iter()
            .map(|edge| (edge.u(), edge.v(), edge.weight().to_bits()))
            .collect()
    }

    fn key(rank: usize) -> Edge {
        let rank = rank as u32;
        Edge::from_compact_parts(rank / 4093, 4093 + rank % 4093, 1.0)
    }

    /// Inputs that stress partitioning: random with few to many duplicates,
    /// already sorted, reversed, all equal, two values, organ pipe, sawtooth
    /// and Musser's median-of-three killer.
    fn inputs(count: usize) -> Vec<(&'static str, Vec<Edge>)> {
        let mut sorted = edges(count, 1);
        sorted.sort_by(compare_raw_edges);
        let reversed: Vec<Edge> = sorted.iter().rev().copied().collect();
        let half = count / 2;
        let organ: Vec<Edge> = (0..count)
            .map(|i| key(if i < half { i } else { count - i }))
            .collect();
        let sawtooth: Vec<Edge> = (0..count).map(|i| key(i % 97)).collect();
        // Median-of-three killer (Musser): pairs pivot candidates so every
        // median is the second smallest, driving naive quicksort quadratic.
        let k = count / 2;
        let mut killer = vec![key(0); count];
        for i in 0..k {
            if i % 2 == 0 {
                killer[i] = key(i + 1);
                killer[i + 1] = key(k + i + 1);
            }
            killer[k + i] = key(2 * (i + 1));
        }
        vec![
            ("random", edges(count, 1)),
            ("random-dup7", edges(count, 7)),
            ("random-dup-all", edges(count, count)),
            ("sorted", sorted),
            ("reversed", reversed),
            ("all-equal", vec![key(5); count]),
            ("two-values", (0..count).map(|i| key(i % 2)).collect()),
            ("organ-pipe", organ),
            ("sawtooth", sawtooth),
            ("median-of-three-killer", killer),
            // Generic structured orderings: one small record moved to the end,
            // eight sorted blocks in descending block order, eight
            // interleaved ascending runs, and an adaptive adversary.
            (
                "push-front",
                (0..count)
                    .map(|i| key(if i + 1 == count { 0 } else { i + 1 }))
                    .collect(),
            ),
            (
                "descending-blocks",
                (0..count)
                    .map(|i| {
                        let block = count.div_ceil(8).max(1);
                        key((7 - (i / block).min(7)) * block + i % block)
                    })
                    .collect(),
            ),
            (
                "interleaved-runs",
                (0..count).map(|i| key(i % 8 * count + i / 8)).collect(),
            ),
            ("mcilroy-adversary", adversary(count)),
            // Duplicate endpoint groups whose weights differ only in the
            // sign of zero, interleaved with equal duplicates.
            (
                "signed-zero-groups",
                (0..count)
                    .map(|i| {
                        let u = ((i / 2) % 5) as u32;
                        Edge::from_compact_parts(u, 7, [0.0, -0.0, -0.0, 0.0, 1.0][i % 5])
                    })
                    .collect(),
            ),
        ]
    }

    /// McIlroy's killer adversary ("A Killer Adversary for Quicksort",
    /// 1999) run against this exact serial algorithm: records start as
    /// unknown "gas" and are frozen to the next smallest value only when two
    /// gas records meet, so every pivot is as small as the answers so far
    /// allow. Replaying the frozen order makes the sort take the same
    /// decisions, so the introsort exhausts its depth and falls back.
    fn adversary(count: usize) -> Vec<Edge> {
        use core::cell::{Cell, RefCell};
        let gas = count;
        let values = RefCell::new(vec![gas; count]);
        let (solid, candidate) = (Cell::new(0), Cell::new(0));
        let compare = |a: &Edge, b: &Edge| {
            let (x, y) = (a.u(), b.u());
            let mut values = values.borrow_mut();
            if values[x] == gas && values[y] == gas {
                values[if x == candidate.get() { x } else { y }] = solid.get();
                solid.set(solid.get() + 1);
            }
            if values[x] == gas {
                candidate.set(x);
            } else if values[y] == gas {
                candidate.set(y);
            }
            values[x].cmp(&values[y])
        };
        let mut identities: Vec<Edge> = (0..count)
            .map(|i| Edge::from_compact_parts(i as u32, (count + i) as u32, 1.0))
            .collect();
        serial_range(
            &mut identities,
            depth_limit(count),
            &mut Unmetered,
            "adversary",
            compare,
        )
        .unwrap();
        let mut values = values.into_inner();
        for value in values.iter_mut().filter(|value| **value == gas) {
            *value = solid.get();
            solid.set(solid.get() + 1);
        }
        values.into_iter().map(key).collect()
    }

    /// Sort serially, recording counted units, the widest single advance and
    /// the depth-limit fallbacks (calls, records).
    fn counted(raw: &[Edge]) -> (Vec<Edge>, crate::cancel::Recorder, (usize, usize)) {
        let mut values = raw.to_vec();
        let mut recorder = crate::cancel::Recorder {
            advances: 0,
            units: 0,
            max_units: 0,
            stop_at: None,
        };
        let _ = fallbacks::take();
        serial_range(
            &mut values,
            depth_limit(raw.len()),
            &mut recorder,
            "sort",
            compare_raw_edges,
        )
        .unwrap();
        (values, recorder, fallbacks::take())
    }

    /// Generic operation-count regression: on structured orderings (sorted,
    /// reversed, organ pipe, one record moved, descending blocks, interleaved
    /// runs, sawtooth, few distinct values and Musser's median-of-three
    /// killer) the ninther never exhausts the depth limit and the counted
    /// work stays within 32 units per record per level (two partition steps;
    /// first/middle/last sampling falls back on most of these and exceeds
    /// this). Every advance stays within one 1,024-unit segment.
    #[test]
    fn ninther_pivots_avoid_fallback_on_structured_orderings() {
        let count: usize = 1 << 16;
        let levels = (usize::BITS - count.leading_zeros()) as usize;
        for (name, raw) in inputs(count) {
            if name == "mcilroy-adversary" {
                continue;
            }
            let (values, recorder, fallen) = counted(&raw);
            let mut expected = raw;
            expected.sort_by(compare_raw_edges);
            assert_eq!(bits(&values), bits(&expected), "{name}");
            assert_eq!(fallen, (0, 0), "{name}: depth-limit fallbacks");
            assert!(
                recorder.units <= 32 * count * levels,
                "{name}: {} counted units",
                recorder.units
            );
            assert!(recorder.max_units <= 1024, "{name}: {}", recorder.max_units);
        }
    }

    /// The pivot is exactly the declared sample rule, computed independently:
    /// below 41 records the median of the first, middle and last; above, the
    /// median of the medians of positions (0, s, 2s), (m - s, m, m + s) and
    /// (last - 2s, last - s, last) with `s = len / 8` and `m = last / 2`.
    #[test]
    fn pivot_is_the_declared_sample_rule() {
        let median = |mut triple: [usize; 3]| {
            triple.sort_unstable();
            triple[1]
        };
        for len in [17, 23, 40, 41, 47, 64, 100, 1000, 1001, 4097] {
            for seed in 0..8_u64 {
                // Distinct ranks in a seeded order: every sample decides.
                let mut ranks: Vec<usize> = (0..len).collect();
                ranks
                    .sort_by_key(|&rank| (rank as u64 * 2_654_435_761 + seed * 40_503) % 1_000_003);
                let values: Vec<Edge> = ranks.iter().map(|&rank| key(rank)).collect();
                let (last, middle, s) = (len - 1, (len - 1) / 2, len / 8);
                let expected = if len < NINTHER_MIN {
                    median([ranks[0], ranks[middle], ranks[last]])
                } else {
                    median([
                        median([ranks[0], ranks[s], ranks[2 * s]]),
                        median([ranks[middle - s], ranks[middle], ranks[middle + s]]),
                        median([ranks[last - 2 * s], ranks[last - s], ranks[last]]),
                    ])
                };
                let (pivot, units) = pivot(&values, compare_raw_edges);
                assert_eq!(
                    bits(&[pivot]),
                    bits(&[key(expected)]),
                    "len {len}, seed {seed}"
                );
                let expected_units = if len < NINTHER_MIN {
                    MEDIAN_UNITS
                } else {
                    NINTHER_UNITS
                };
                assert_eq!(units, expected_units, "len {len}");
            }
        }
    }

    /// The ninther is not adversary-proof: an adaptive adversary built
    /// against this algorithm still exhausts the depth limit, and the
    /// heapsort fallback keeps the result exact and the counted work within
    /// O(n log n), serially and on the team.
    #[test]
    fn adaptive_adversary_still_reaches_the_depth_fallback() {
        let count: usize = 1 << 14;
        let levels = (usize::BITS - count.leading_zeros()) as usize;
        let raw = adversary(count);
        let mut expected = raw.clone();
        expected.sort_by(compare_raw_edges);
        let (values, recorder, (calls, records)) = counted(&raw);
        assert_eq!(bits(&values), bits(&expected));
        assert!(calls > 0 && records > 0, "no fallback: {calls} / {records}");
        assert!(
            recorder.units <= 256 * count * levels,
            "{} units",
            recorder.units
        );
        assert!(recorder.max_units <= 1024);
        #[cfg(feature = "parallel")]
        {
            let pool = rayon::ThreadPoolBuilder::new()
                .num_threads(4)
                .build()
                .unwrap();
            for threads in [1, 4] {
                let team = Team::new(&pool, threads, crate::ParallelOptions::default()).unwrap();
                let mut values = raw.clone();
                sort(&mut values, team, &mut Unmetered).unwrap();
                assert_eq!(bits(&values), bits(&expected), "ceiling {threads}");
            }
        }
    }

    /// Pivot selection is counted after it runs and before any record is
    /// classified: cancelling at a partition's first advance stops it with
    /// no classification, after exactly the ninther's (above 40 records) or
    /// the median of three's counted units.
    #[cfg(feature = "parallel")]
    #[test]
    fn pivot_selection_is_counted_before_classification() {
        use crate::execution::{gauge, visits};
        let _witness = gauge::WITNESS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        for (count, units) in [
            (1000, NINTHER_UNITS),
            (41, NINTHER_UNITS),
            (40, MEDIAN_UNITS),
            (17, MEDIAN_UNITS),
        ] {
            let mut values = edges(count, 1);
            let mut recorder = crate::cancel::Recorder {
                advances: 0,
                units: 0,
                max_units: 0,
                stop_at: Some(1),
            };
            visits::watch_range(&values);
            let result = serial_range(
                &mut values,
                depth_limit(count),
                &mut recorder,
                "sort",
                compare_raw_edges,
            );
            let classified = visits::count();
            visits::unwatch();
            assert!(matches!(result, Err(CmgError::Cancelled { .. })), "{count}");
            assert_eq!((recorder.advances, recorder.units), (1, units), "{count}");
            assert_eq!(
                classified, 0,
                "{count}: classified before the pivot was counted"
            );
        }
    }

    #[test]
    fn serial_sort_equals_the_standard_sort_on_adversarial_inputs() {
        for count in [0, 1, 2, 16, 17, 1_000, 50_001] {
            for (name, raw) in inputs(count) {
                let mut expected = raw.clone();
                expected.sort_by(compare_raw_edges);
                let mut actual = raw.clone();
                sort_serial(&mut actual, &mut Unmetered, "sort").unwrap();
                assert_eq!(bits(&actual), bits(&expected), "{name} {count}");
                let never = || false;
                let mut polled = raw;
                sort_serial(&mut polled, &mut CancelMeter::new(&never), "sort").unwrap();
                assert_eq!(bits(&polled), bits(&expected), "{name} {count} polled");
            }
        }
        // Signed zeros order by total_cmp inside one endpoint group.
        let mut zeros = vec![
            Edge::from_compact_parts(0, 1, 0.0),
            Edge::from_compact_parts(0, 1, -0.0),
        ];
        sort_serial(&mut zeros, &mut Unmetered, "sort").unwrap();
        assert_eq!(zeros[0].weight().to_bits(), (-0.0_f64).to_bits());
        // The bitwise comparison discriminates what `PartialEq` cannot.
        let swapped = [zeros[1], zeros[0]];
        assert_eq!(swapped, *zeros);
        assert_ne!(bits(&swapped), bits(&zeros));
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn team_sort_equals_the_standard_sort_at_every_ceiling() {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(8)
            .build()
            .unwrap();
        for (name, raw) in inputs(200_000) {
            let mut expected = raw.clone();
            expected.sort_by(compare_raw_edges);
            for threads in [1, 2, 3, 8] {
                let team = Team::new(&pool, threads, crate::ParallelOptions::default()).unwrap();
                let mut actual = raw.clone();
                sort(&mut actual, team, &mut Unmetered).unwrap();
                assert_eq!(bits(&actual), bits(&expected), "{name} ceiling {threads}");
            }
        }
    }

    /// Cancellation observed inside the first partition of a 400,000-record
    /// sort stops it there: after the observing poll the serial sort visits
    /// nothing more and each team lane at most one poll interval of partition
    /// steps (4,096) plus one 64-step segment; untouched records are never
    /// partitioned, every lane has joined, and a fresh sort is exact.
    #[cfg(feature = "parallel")]
    #[test]
    fn sorts_cancel_inside_a_partition_and_stop_untouched_work() {
        use crate::execution::{gauge, visits};
        use std::sync::atomic::{AtomicUsize, Ordering};
        let _witness = gauge::WITNESS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .unwrap();
        let raw = edges(400_000, 3);
        let mut expected = raw.clone();
        expected.sort_by(compare_raw_edges);
        let interval = crate::cancel::POLL_UNITS as usize / 16;
        for ceiling in [0, 1, 4] {
            for stop in [0, 1, 2] {
                let calls = AtomicUsize::new(0);
                let at = AtomicUsize::new(usize::MAX);
                let query = || {
                    if calls.fetch_add(1, Ordering::SeqCst) < stop {
                        return false;
                    }
                    let _ = at.compare_exchange(
                        usize::MAX,
                        visits::count(),
                        Ordering::SeqCst,
                        Ordering::SeqCst,
                    );
                    true
                };
                let mut values = raw.clone();
                visits::watch_range(&values);
                let result = if ceiling == 0 {
                    sort_serial(&mut values, &mut CancelMeter::new(&query), "sort")
                } else {
                    let team =
                        Team::new(&pool, ceiling, crate::ParallelOptions::default()).unwrap();
                    sort(&mut values, team, &mut CancelMeter::new(&query))
                };
                let total = visits::count();
                visits::unwatch();
                let label = format!("ceiling {ceiling}, stop {stop}");
                assert!(matches!(result, Err(CmgError::Cancelled { .. })), "{label}");
                let settled = calls.load(Ordering::SeqCst);
                std::thread::sleep(std::time::Duration::from_millis(5));
                assert_eq!(
                    calls.load(Ordering::SeqCst),
                    settled,
                    "{label}: lanes joined"
                );
                let at = at.load(Ordering::SeqCst);
                let lanes = ceiling.max(1);
                assert!(
                    total - at <= lanes * (interval + 64),
                    "{label}: {} steps after the observation",
                    total - at
                );
                // The first partition alone takes 400,000 steps.
                assert!(total < raw.len(), "{label}: {total} steps");
            }
        }
        let mut again = raw;
        sort_serial(&mut again, &mut CancelMeter::new(&|| false), "sort").unwrap();
        assert_eq!(bits(&again), bits(&expected));
    }

    /// What the post-fork witness observed at the firing and after the join.
    #[cfg(feature = "parallel")]
    #[derive(Default)]
    struct Barrier {
        threads: std::collections::BTreeSet<String>,
        fired: bool,
        timed_out: bool,
        visits_at_fire: usize,
        active_at_fire: usize,
        /// Queries counted under this lock, so the count at the firing
        /// includes every query that has entered the barrier.
        calls: usize,
        calls_at_fire: usize,
        /// The firing query's pre-lock ordinal plus one: the original
        /// snapshot, kept to show that an ordinal inversion makes it stale.
        ordinal_at_fire: usize,
        untouched: usize,
    }

    /// Run the post-fork cancellation witness at `ceiling` and check it. With
    /// `invert`, the first qualifying lane takes its pre-lock ordinal and is
    /// held before the barrier lock until every other lane has parked, so it
    /// arrives last carrying the oldest ordinal; the witness must still count
    /// every query. Returns the barrier state.
    #[cfg(feature = "parallel")]
    fn started_lanes_witness(ceiling: usize, invert: bool) -> Barrier {
        use crate::execution::{gauge, visits};
        use std::sync::atomic::{AtomicUsize, Ordering};
        use std::sync::{Condvar, Mutex};
        use std::time::{Duration, Instant};
        let _witness = gauge::WITNESS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .unwrap();
        let raw = edges(400_000, 3);
        let mut expected = raw.clone();
        expected.sort_by(compare_raw_edges);
        let label = format!("ceiling {ceiling}, invert {invert}");
        let barrier = Mutex::new(Barrier::default());
        let arrived = Condvar::new();
        let ordinals = AtomicUsize::new(0);
        // Serializes qualifying queries' ordinals while choosing the held
        // lane, so every other lane's parking ordinal follows the held one.
        let selection = Mutex::new(false);
        let deadline = Instant::now() + Duration::from_secs(30);
        // Wait on `arrived` until `done`, or fire the escape at the deadline.
        fn wait<'a>(
            arrived: &Condvar,
            deadline: Instant,
            mut state: std::sync::MutexGuard<'a, Barrier>,
            done: impl Fn(&Barrier) -> bool,
        ) -> std::sync::MutexGuard<'a, Barrier> {
            while !done(&state) && !state.fired {
                let now = Instant::now();
                if now >= deadline {
                    // Escape: stop every lane and fail after the join.
                    state.timed_out = true;
                    state.fired = true;
                    arrived.notify_all();
                    break;
                }
                state = arrived.wait_timeout(state, deadline - now).unwrap().0;
            }
            state
        }
        let query = || {
            // Only polls in the middle of a partition (records left to
            // classify) of a one-lane leaf, after the root partition
            // (400,000 steps) has completed. Each part is thread-local or
            // monotone, so it holds as well under the lock below.
            let remaining = probe::remaining().unwrap_or(0);
            let qualifies = gauge::inside() && remaining > 0 && visits::count() >= raw.len();
            let (ordinal, held) = if invert && qualifies {
                let mut chosen = selection.lock().unwrap();
                let ordinal = ordinals.fetch_add(1, Ordering::SeqCst);
                (ordinal, !std::mem::replace(&mut *chosen, true))
            } else {
                (ordinals.fetch_add(1, Ordering::SeqCst), false)
            };
            if held {
                // The ordinal is taken; hold before the lock until every
                // other lane has parked with a later ordinal.
                let state = barrier.lock().unwrap();
                drop(wait(&arrived, deadline, state, |state| {
                    state.threads.len() + 1 == ceiling
                }));
            }
            let mut state = barrier.lock().unwrap();
            state.calls += 1;
            if state.fired {
                return true;
            }
            if !qualifies {
                return false;
            }
            state.untouched += remaining;
            state
                .threads
                .insert(format!("{:?}", std::thread::current().id()));
            if state.threads.len() == ceiling {
                state.fired = true;
                state.visits_at_fire = visits::count();
                state.active_at_fire = gauge::active();
                state.calls_at_fire = state.calls;
                state.ordinal_at_fire = ordinal + 1;
                arrived.notify_all();
                return true;
            }
            arrived.notify_all();
            drop(wait(&arrived, deadline, state, |_| false));
            true
        };
        gauge::watch(&pool);
        let mut values = raw.clone();
        visits::watch_range(&values);
        let team = Team::new(&pool, ceiling, crate::ParallelOptions::default()).unwrap();
        let result = sort(&mut values, team, &mut CancelMeter::new(&query));
        let (total, returned_calls, active) = (
            visits::count(),
            barrier.lock().unwrap().calls,
            gauge::active(),
        );
        // Nothing runs after the return (a sleep cannot make this pass
        // falsely for a correct join; it only gives a stray lane time).
        std::thread::sleep(Duration::from_millis(20));
        let (later_total, later_calls) = (visits::count(), barrier.lock().unwrap().calls);
        visits::unwatch();
        let state = barrier.into_inner().unwrap();
        assert!(!state.timed_out, "{label}: lanes never all arrived");
        assert!(
            matches!(result, Err(CmgError::Cancelled { phase: PHASE })),
            "{label}: {result:?}"
        );
        // Distinct threads were parked inside partitions at once.
        assert_eq!(state.threads.len(), ceiling, "{label}");
        assert_eq!(state.active_at_fire, ceiling, "{label}: concurrent lanes");
        assert_eq!(gauge::max(), ceiling, "{label}: lane ceiling");
        // No partition step after the observation: each lane stopped at its
        // observing poll, so later work stayed untouched.
        assert_eq!(total, state.visits_at_fire, "{label}: steps after firing");
        assert!(state.untouched >= ceiling, "{label}: parked mid-partition");
        assert!(total < 3 * raw.len(), "{label}: {total} steps");
        // The only query after the firing is the coordinator's final poll
        // after the join: no lane reached another node.
        assert_eq!(
            returned_calls,
            state.calls_at_fire + 1,
            "{label}: polls after firing"
        );
        assert_eq!(ordinals.load(Ordering::SeqCst), returned_calls, "{label}");
        assert_eq!(active, 0, "{label}: every lane joined");
        assert_eq!(
            (later_total, later_calls),
            (total, returned_calls),
            "{label}"
        );
        // Fresh exact sorts on the same team and serially.
        let mut again = raw.clone();
        sort(&mut again, team, &mut CancelMeter::new(&|| false)).unwrap();
        assert_eq!(bits(&again), bits(&expected), "{label}: fresh team sort");
        let mut serial = raw;
        sort_serial(&mut serial, &mut Unmetered, "sort").unwrap();
        assert_eq!(bits(&serial), bits(&expected), "{label}: fresh serial sort");
        state
    }

    /// Cancellation after the fork, inside started concurrent lanes. A query
    /// parks every lane at its first poll inside actual partition work past
    /// the root partition (at the W4 ceiling, the four one-lane leaves after
    /// the second fork) until all `ceiling` lanes have arrived on distinct
    /// threads, then fires. Parking is a deterministic barrier with a bounded
    /// timeout that fails the witness instead of hanging; success depends on
    /// no sleep or schedule. Every lane stops at its observing poll: no record
    /// is visited and no later node polls after the firing, all lanes have
    /// joined on return, nothing moves afterwards, and fresh sorts are exact.
    #[cfg(feature = "parallel")]
    #[test]
    fn started_lanes_cancel_inside_partitions_after_the_fork() {
        for ceiling in [4, 1] {
            started_lanes_witness(ceiling, false);
        }
    }

    /// The witness counts queries under its barrier lock. A query's pre-lock
    /// ordinal can be older than queries that entered the lock before it; the
    /// forced inversion holds the first qualifying lane after it took its
    /// ordinal until the three other lanes have parked with later ordinals.
    /// The original snapshot (firing ordinal plus one) is then stale by at
    /// least those three queries and would reject a correct join, while the
    /// count under the lock is exact.
    #[cfg(feature = "parallel")]
    #[test]
    fn started_lanes_witness_counts_queries_under_the_lock_despite_ordinal_inversion() {
        let state = started_lanes_witness(4, true);
        assert!(
            state.ordinal_at_fire + 3 <= state.calls_at_fire,
            "inversion not forced: ordinal {} calls {}",
            state.ordinal_at_fire,
            state.calls_at_fire
        );
    }

    /// Work between two consecutive polls stays bounded on every path: at
    /// most one cadence (4,096 partition steps) plus one 64-step segment per
    /// lane, also across the many small nodes deep in the partition tree.
    /// Each query reads the step counter and records the gap under one lock,
    /// so consecutive reads are ordered: a lane preempted between reading and
    /// recording cannot store a stale reading that inflates the next gap.
    #[cfg(feature = "parallel")]
    #[test]
    fn work_between_polls_is_bounded_deep_in_the_partition_tree() {
        use crate::execution::{gauge, visits};
        use std::sync::Mutex;
        let _witness = gauge::WITNESS
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .unwrap();
        let raw = edges(1 << 20, 1);
        let interval = crate::cancel::POLL_UNITS as usize / 16;
        for ceiling in [0, 1, 4] {
            // (last reading, widest gap)
            let state = Mutex::new((0_usize, 0_usize));
            let query = || {
                let mut state = state.lock().unwrap();
                let now = visits::count();
                state.1 = state.1.max(now - state.0);
                state.0 = now;
                false
            };
            let mut values = raw.clone();
            visits::watch_range(&values);
            if ceiling == 0 {
                sort_serial(&mut values, &mut CancelMeter::new(&query), "sort").unwrap();
            } else {
                let team = Team::new(&pool, ceiling, crate::ParallelOptions::default()).unwrap();
                sort(&mut values, team, &mut CancelMeter::new(&query)).unwrap();
            }
            let total = visits::count();
            visits::unwatch();
            let (last, widest) = state.into_inner().unwrap();
            let gap = widest.max(total - last);
            assert!(
                gap <= ceiling.max(1) * (interval + 64),
                "ceiling {ceiling}: {gap} partition steps between polls"
            );
            assert!(total > 16 * raw.len(), "{total} partition steps");
        }
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn team_sort_polls_inside_partitions_and_cancels() {
        let pool = rayon::ThreadPoolBuilder::new()
            .num_threads(8)
            .build()
            .unwrap();
        let team = Team::new(&pool, 4, crate::ParallelOptions::default()).unwrap();
        let raw = edges(400_000, 3);
        let polls = std::sync::atomic::AtomicUsize::new(0);
        let count = || {
            polls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            false
        };
        let mut sorted = raw.clone();
        sort(&mut sorted, team, &mut CancelMeter::new(&count)).unwrap();
        // About 16 units per record per partition level: many lane polls.
        let total = polls.load(std::sync::atomic::Ordering::SeqCst);
        assert!(
            total > 400_000 * 16 / crate::cancel::POLL_UNITS as usize,
            "{total}"
        );
        // Every logical node polls on entry with its own counter, so the
        // schedule depends on the input only, never on the ceiling.
        for threads in [1, 2, 3, 8] {
            let team = Team::new(&pool, threads, crate::ParallelOptions::default()).unwrap();
            let polls = std::sync::atomic::AtomicUsize::new(0);
            let count = || {
                polls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                false
            };
            let mut values = raw.clone();
            sort(&mut values, team, &mut CancelMeter::new(&count)).unwrap();
            assert_eq!(bits(&values), bits(&sorted), "ceiling {threads}");
            assert_eq!(
                polls.load(std::sync::atomic::Ordering::SeqCst),
                total,
                "ceiling {threads}"
            );
        }
        for stop in [0, total / 3, total / 2] {
            let calls = std::sync::atomic::AtomicUsize::new(0);
            let cancel = || calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst) >= stop;
            let mut values = raw.clone();
            assert!(matches!(
                sort(&mut values, team, &mut CancelMeter::new(&cancel)),
                Err(CmgError::Cancelled { .. })
            ));
        }
        let mut again = raw;
        sort(&mut again, team, &mut Unmetered).unwrap();
        assert_eq!(bits(&again), bits(&sorted));
    }
}
