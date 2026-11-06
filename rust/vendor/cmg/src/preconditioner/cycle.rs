//! The one fixed stationary CMG cycle.
//!
//! Every application (scalar or interleaved, serial or planned, plain,
//! cancellable or profiled) runs [`Cycle::level`]. What varies is resolved
//! statically or per level without changing the algorithm:
//!
//! * [`Width`]: one statically known lane for the scalar workspace, or a
//!   runtime lane count for vertex-major interleaved batches.
//! * [`Store`]: where each level's work vectors live. The scalar workspace
//!   also selects the existing team fixed-chunk centering of a single coarse
//!   component; interleaved lanes keep serial compensated sums.
//! * The level operator: a [`ParallelCmgPlan`] row operator (gather) where the
//!   plan retained one, else the canonical edge list (scatter).
//! * The meter: [`crate::cancel::Unmetered`] compiles every poll away;
//!   a cancellation meter polls every bounded chunk of items in every kernel,
//!   and parallel lanes poll the sticky query before joining.
//! * The recorder: `()` records nothing; a cycle profile records exclusive
//!   phase times.
//!
//! Per-lane arithmetic and order are those of the scalar cycle.

use super::CmgPreconditioner;
#[cfg(feature = "parallel")]
use super::ParallelCmgPlan;
use crate::cancel::Meter;
use crate::cmg_profile::{CmgApplyPhase, CycleRecorder};
#[cfg(feature = "parallel")]
use crate::execution::Team;
use crate::{CmgError, GroundedLdl, TerminalReason};

/// Lanes processed per vertex by one application.
pub(crate) trait Width: Copy + Send + Sync {
    fn get(self) -> usize;

    /// Terminal factor solve in this layout.
    fn terminal<M: Meter>(
        self,
        factor: &GroundedLdl,
        rhs: &[f64],
        output: &mut [f64],
        forward: &mut [f64],
        solution: &mut [f64],
        meter: &mut M,
    ) -> Result<(), CmgError>;
}

/// The scalar workspace: exactly one lane.
#[derive(Clone, Copy)]
pub(crate) struct One;

impl Width for One {
    #[inline(always)]
    fn get(self) -> usize {
        1
    }

    #[inline]
    fn terminal<M: Meter>(
        self,
        factor: &GroundedLdl,
        rhs: &[f64],
        output: &mut [f64],
        forward: &mut [f64],
        solution: &mut [f64],
        meter: &mut M,
    ) -> Result<(), CmgError> {
        factor.solve_into_compatible(rhs, output, forward, solution, meter)
    }
}

impl Width for usize {
    #[inline(always)]
    fn get(self) -> usize {
        self
    }

    #[inline]
    fn terminal<M: Meter>(
        self,
        factor: &GroundedLdl,
        rhs: &[f64],
        output: &mut [f64],
        forward: &mut [f64],
        solution: &mut [f64],
        meter: &mut M,
    ) -> Result<(), CmgError> {
        factor.solve_batch_compatible(rhs, output, self, forward, solution, meter)
    }
}

/// Borrowed work vectors of one level, sized for the call's lanes.
pub(crate) struct Parts<'l> {
    pub(crate) residual: &'l mut [f64],
    pub(crate) coarse_rhs: &'l mut [f64],
    pub(crate) coarse_correction: &'l mut [f64],
    pub(crate) forward: &'l mut [f64],
    pub(crate) solution: &'l mut [f64],
    pub(crate) sums: &'l mut [f64],
    pub(crate) corrections: &'l mut [f64],
    pub(crate) means: &'l mut [f64],
}

/// Owned work vectors of one level.
pub(crate) trait LevelParts {
    /// Borrow the prefixes used by `columns` lanes.
    fn parts(&mut self, columns: usize) -> Parts<'_>;
}

/// Per-level work vectors of one validated workspace.
pub(crate) trait Store {
    type Width: Width;
    /// Owned buffers of one level, moved out during its recursive visit.
    type Level: LevelParts;
    /// Whether a single coarse component is centered with the team's
    /// fixed-chunk reduction (the scalar planned arithmetic).
    #[cfg_attr(not(feature = "parallel"), allow(dead_code))]
    const TEAM_CENTERING: bool;

    fn width(&self) -> Self::Width;
    fn take(&mut self, index: usize) -> Self::Level;
    fn put(&mut self, index: usize, level: Self::Level);
}

/// One application: preconditioner, store and execution resources.
pub(crate) struct Cycle<'a, S> {
    pub(crate) cmg: &'a CmgPreconditioner,
    pub(crate) store: S,
    #[cfg(feature = "parallel")]
    pub(crate) plan: Option<&'a ParallelCmgPlan>,
    #[cfg(feature = "parallel")]
    pub(crate) team: Option<Team<'a>>,
}

impl<S: Store> Cycle<'_, S> {
    /// Apply `iterations` stationary sweeps at `index`, recursing to the child.
    pub(crate) fn level<M: Meter, R: CycleRecorder>(
        &mut self,
        index: usize,
        rhs: &[f64],
        output: &mut [f64],
        iterations: usize,
        meter: &mut M,
        recorder: &mut R,
    ) -> Result<(), CmgError> {
        let cmg = self.cmg;
        let level = &cmg.hierarchy.levels()[index];
        let width = self.store.width();
        let values = level.graph().vertex_count() * width.get();
        if rhs.len() != values || output.len() != values {
            return Err(CmgError::InvalidHierarchy {
                context: "recursive vector dimension does not match hierarchy level",
            });
        }

        recorder.enter(index);
        if let Some(reason) = level.terminal_reason() {
            let stamp = R::start();
            let result = if reason == TerminalReason::Direct {
                let factor = cmg
                    .direct_terminal
                    .as_ref()
                    .ok_or(CmgError::InvalidHierarchy {
                        context: "direct terminal is missing its LDL factor",
                    })?;
                let mut local = self.store.take(index);
                let parts = local.parts(width.get());
                let result =
                    width.terminal(factor, rhs, output, parts.forward, parts.solution, meter);
                self.store.put(index, local);
                result
            } else {
                self.scale(false, output, level.inverse_diagonal(), rhs, meter)
            };
            recorder.finish(index, CmgApplyPhase::Terminal, stamp);
            return result;
        }

        if iterations == 0 {
            return Err(CmgError::InvalidHierarchy {
                context: "nonterminal level has zero stationary iterations",
            });
        }
        let child_iterations = cmg.repeat_counts[index];
        if child_iterations == 0 {
            return Err(CmgError::InvalidHierarchy {
                context: "nonterminal level has zero child recursive repeats",
            });
        }
        let mut local = self.store.take(index);
        let result = self.sweeps(
            index,
            rhs,
            output,
            iterations,
            child_iterations,
            &mut local,
            meter,
            recorder,
        );
        self.store.put(index, local);
        result
    }

    #[allow(clippy::too_many_arguments)]
    fn sweeps<M: Meter, R: CycleRecorder>(
        &mut self,
        index: usize,
        rhs: &[f64],
        output: &mut [f64],
        iterations: usize,
        child_iterations: usize,
        local: &mut S::Level,
        meter: &mut M,
        recorder: &mut R,
    ) -> Result<(), CmgError> {
        let level = &self.cmg.hierarchy.levels()[index];
        let width = self.store.width();
        let parallel = self.parallel_level(index);
        let parts = local.parts(width.get());
        // With a zero-dimensional child, restriction and correction have no
        // output. Keep both smoothing sweeps, but avoid forming the unused
        // middle residual and making an empty recursive call.
        #[cfg(feature = "experimental-components")]
        let has_coarse_work = !parts.coarse_rhs.is_empty();
        #[cfg(not(feature = "experimental-components"))]
        let has_coarse_work = true;
        for iteration in 0..iterations {
            recorder.iteration(index);
            let stamp = R::start();
            if iteration == 0 {
                self.scale(parallel, output, level.inverse_diagonal(), rhs, meter)?;
            } else {
                self.matvec(index, output, parts.residual, meter)?;
                self.jacobi(
                    parallel,
                    output,
                    level.inverse_diagonal(),
                    rhs,
                    parts.residual,
                    meter,
                )?;
            }
            recorder.finish(
                index,
                if iteration == 0 {
                    CmgApplyPhase::Initialization
                } else {
                    CmgApplyPhase::Smoothing
                },
                stamp,
            );

            if has_coarse_work {
                let stamp = R::start();
                self.matvec(index, output, parts.residual, meter)?;
                recorder.finish(index, CmgApplyPhase::ResidualMatvec, stamp);
                let stamp = R::start();
                level.restrict_residual_lanes(
                    rhs,
                    parts.residual,
                    parts.coarse_rhs,
                    width,
                    meter,
                )?;
                recorder.finish(index, CmgApplyPhase::Restriction, stamp);
                let stamp = R::start();
                // Restricted residuals are component-compatible in exact
                // arithmetic. Remove only floating-point null-space drift
                // before the recursive solve.
                self.center(
                    index,
                    parts.coarse_rhs,
                    parts.sums,
                    parts.corrections,
                    parts.means,
                    meter,
                )?;
                recorder.finish(index, CmgApplyPhase::Centering, stamp);
                self.level(
                    index + 1,
                    parts.coarse_rhs,
                    parts.coarse_correction,
                    child_iterations,
                    meter,
                    recorder,
                )?;
                let stamp = R::start();
                level.prolong_add_lanes(
                    parts.coarse_correction,
                    output,
                    width,
                    self.parallel_team(parallel),
                    meter,
                )?;
                recorder.finish(index, CmgApplyPhase::Prolongation, stamp);
            }

            let stamp = R::start();
            self.matvec(index, output, parts.residual, meter)?;
            self.jacobi(
                parallel,
                output,
                level.inverse_diagonal(),
                rhs,
                parts.residual,
                meter,
            )?;
            recorder.finish(index, CmgApplyPhase::Smoothing, stamp);
        }
        Ok(())
    }

    /// Whether the plan retained a row operator for this level.
    #[inline]
    fn parallel_level(&self, index: usize) -> bool {
        #[cfg(feature = "parallel")]
        {
            self.plan
                .is_some_and(|plan| plan.level_operator(index).is_some())
        }
        #[cfg(not(feature = "parallel"))]
        {
            let _ = index;
            false
        }
    }

    /// The team for elementwise level kernels, only on planned levels.
    #[cfg(feature = "parallel")]
    #[inline]
    fn parallel_team(&self, parallel: bool) -> Option<Team<'_>> {
        self.team.filter(|_| parallel)
    }

    #[cfg(not(feature = "parallel"))]
    #[inline]
    fn parallel_team(&self, _: bool) -> Option<crate::execution::Team<'_>> {
        None
    }

    /// `output = D^-1 rhs`, row-parallel on planned levels.
    fn scale<M: Meter>(
        &self,
        parallel: bool,
        output: &mut [f64],
        inverse_diagonal: &[f64],
        rhs: &[f64],
        meter: &mut M,
    ) -> Result<(), CmgError> {
        let columns = self.store.width().get();
        // Captured by value: the closure also reaches the out-of-line parallel
        // path, and a captured local's escaped address would make the serial
        // loop reload the width after every polled segment.
        let operation = move |row: usize, first: usize, values: &mut [f64]| {
            let diagonal = inverse_diagonal[row];
            for (lane, value) in (first..).zip(values.iter_mut()) {
                *value = diagonal * rhs[row * columns + lane];
            }
        };
        #[cfg(feature = "parallel")]
        if let Some(team) = self.parallel_team(parallel) {
            if team.should_parallel(output.len()) {
                let rows = rows_parallel(team, meter.query(), output, columns, operation);
                meter.joined("CMG cycle diagonal")?;
                return rows;
            }
        }
        let _ = parallel;
        rows_serial(meter, "CMG cycle diagonal", output, columns, operation)
    }

    /// `output += D^-1 (rhs - product)`, row-parallel on planned levels.
    fn jacobi<M: Meter>(
        &self,
        parallel: bool,
        output: &mut [f64],
        inverse_diagonal: &[f64],
        rhs: &[f64],
        product: &[f64],
        meter: &mut M,
    ) -> Result<(), CmgError> {
        let columns = self.store.width().get();
        // Captured by value: the closure also reaches the out-of-line parallel
        // path, and a captured local's escaped address would make the serial
        // loop reload the width after every polled segment.
        let operation = move |row: usize, first: usize, values: &mut [f64]| {
            let diagonal = inverse_diagonal[row];
            for (lane, value) in (first..).zip(values.iter_mut()) {
                let offset = row * columns + lane;
                *value += diagonal * (rhs[offset] - product[offset]);
            }
        };
        #[cfg(feature = "parallel")]
        if let Some(team) = self.parallel_team(parallel) {
            if team.should_parallel(output.len()) {
                let rows = rows_parallel(team, meter.query(), output, columns, operation);
                meter.joined("CMG cycle smoothing")?;
                return rows;
            }
        }
        let _ = parallel;
        rows_serial(meter, "CMG cycle smoothing", output, columns, operation)
    }

    /// `output = L input` with the plan's row operator or the edge list.
    fn matvec<M: Meter>(
        &self,
        index: usize,
        input: &[f64],
        output: &mut [f64],
        meter: &mut M,
    ) -> Result<(), CmgError> {
        let width = self.store.width();
        #[cfg(feature = "parallel")]
        if let Some(operator) = self.plan.and_then(|plan| plan.level_operator(index)) {
            return operator.matvec_lanes(input, output, width, self.team, meter);
        }
        self.cmg.hierarchy.levels()[index]
            .graph()
            .matvec_lanes(input, output, width, meter)
    }

    /// Remove coarse component means from restricted residuals.
    #[allow(clippy::too_many_arguments)]
    fn center<M: Meter>(
        &self,
        index: usize,
        values: &mut [f64],
        sums: &mut [f64],
        corrections: &mut [f64],
        means: &mut [f64],
        meter: &mut M,
    ) -> Result<(), CmgError> {
        let plan = &self.cmg.coarse_centering[index];
        #[cfg(feature = "parallel")]
        if S::TEAM_CENTERING {
            if let Some(team) = self.team {
                if plan.uses_team_centering(values.len(), team) {
                    return plan.center_single_with_team(values, means, team, meter);
                }
            }
        }
        plan.center_lanes(values, self.store.width(), sums, corrections, means, meter)
    }
}

/// Serial elementwise rows in bounded counted chunks; `operation(row, first,
/// values)` updates lanes `first..` of `row`: every lane of each row of a
/// counted segment, or one lane tile of a row wider than one segment.
#[inline(always)]
pub(crate) fn rows_serial<M: Meter>(
    meter: &mut M,
    phase: &'static str,
    output: &mut [f64],
    columns: usize,
    operation: impl Fn(usize, usize, &mut [f64]),
) -> Result<(), CmgError> {
    crate::cancel::row_lanes(
        meter,
        phase,
        output.len() / columns,
        columns,
        1,
        #[inline(always)]
        |rows, lanes| {
            let block = &mut output[rows.start * columns..rows.end * columns];
            for (row, values) in rows.zip(block.chunks_exact_mut(columns)) {
                operation(row, lanes.start, &mut values[lanes.clone()]);
            }
            Ok(())
        },
    )
}

/// Rows on the team in fixed blocks of the team's work-chunk length, claimed by
/// at most `worker_ceiling` lanes. `kernel(row, values, poll)` counts its own
/// work through the lane's poll, inside long rows too, and its error stops the
/// lane. Callers poll their meter after the join, so cancellation dominates.
#[cfg(feature = "parallel")]
pub(crate) fn rows_parallel_with(
    team: Team<'_>,
    query: Option<crate::Cancellation<'_>>,
    output: &mut [f64],
    columns: usize,
    kernel: impl Fn(usize, &mut [f64], &mut crate::cancel::LanePoll<'_>) -> Result<(), CmgError> + Sync,
) -> Result<(), CmgError> {
    let rows = team.work_chunk_len(output.len() / columns);
    team.chunks_mut(output, rows * columns, query, |block, values, poll| {
        let first = block * rows;
        for (offset, row) in values.chunks_exact_mut(columns).enumerate() {
            kernel(first + offset, row, poll)?;
        }
        Ok(())
    })
}

/// Elementwise rows of `columns` units each on the team; as in
/// [`rows_serial`], a row wider than one counted segment is updated and
/// counted in lane tiles.
#[cfg(feature = "parallel")]
pub(crate) fn rows_parallel(
    team: Team<'_>,
    query: Option<crate::Cancellation<'_>>,
    output: &mut [f64],
    columns: usize,
    operation: impl Fn(usize, usize, &mut [f64]) + Sync,
) -> Result<(), CmgError> {
    type Lane<'p> = crate::cancel::LanePoll<'p>;
    rows_parallel_with(team, query, output, columns, |row, values, poll| {
        for lanes in crate::cancel::lane_tiles::<Lane<'_>>(columns, 1) {
            operation(row, lanes.start, &mut values[lanes.clone()]);
            crate::cancel::Progress::advance(poll, "parallel rows", lanes.len())?;
        }
        Ok(())
    })
}

#[cfg(test)]
// Context literals keep `..Default::default()` for their feature-gated fields.
#[allow(clippy::needless_update)]
mod cancellation_tests {
    use crate::cancel::{CancelMeter, StopAt, Unmetered};
    use crate::{ApplyContext, CmgError, CmgOptions, CmgPreconditioner, Laplacian};

    /// Path with chords: several nonterminal levels and a direct terminal.
    fn fixture(n: usize, direct_threshold: usize) -> (Laplacian, CmgPreconditioner, Vec<f64>) {
        let edges = (1..n)
            .map(|v| (v - 1, v, 1.0 + (v % 7) as f64 / 4.0))
            .chain((5..n).step_by(5).map(|v| (v - 5, v, 0.25)));
        let graph = Laplacian::from_edges(n, edges).unwrap();
        let preconditioner = CmgPreconditioner::build(
            &graph,
            CmgOptions {
                direct_threshold,
                ..CmgOptions::default()
            },
        )
        .unwrap();
        let known: Vec<_> = (0..n).map(|v| ((v * 13) % 31) as f64 - 15.0).collect();
        let rhs = graph.matvec(&known).unwrap();
        (graph, preconditioner, rhs)
    }

    fn bits(values: &[f64]) -> Vec<u64> {
        values.iter().map(|value| value.to_bits()).collect()
    }

    #[test]
    fn scalar_cycle_cancels_at_every_bounded_poll_and_workspace_stays_reusable() {
        let (graph, preconditioner, rhs) = fixture(600, 32);
        assert!(preconditioner.hierarchy().levels().len() > 2);
        assert!(preconditioner.terminal_factor().is_some());
        let mut workspace = preconditioner.try_workspace().unwrap();
        let mut expected = vec![0.0; graph.vertex_count()];
        preconditioner
            .apply_compatible_into(&rhs, &mut expected, &mut workspace)
            .unwrap();

        let count = StopAt::never();
        let query = || count.query();
        let mut output = vec![f64::NAN; graph.vertex_count()];
        preconditioner
            .apply_compatible_metered(
                &rhs,
                &mut output,
                &mut workspace,
                ApplyContext::default(),
                &mut CancelMeter::every_operation(&query),
                &mut (),
            )
            .unwrap();
        assert_eq!(bits(&output), bits(&expected));
        let polls = count.polls();
        assert!(polls > 50, "{polls}");

        let mut phases = std::collections::BTreeSet::new();
        for stop in 0..polls {
            let cancel = StopAt::at(Some(stop));
            let query = || cancel.query();
            match preconditioner.apply_compatible_metered(
                &rhs,
                &mut output,
                &mut workspace,
                ApplyContext::default(),
                &mut CancelMeter::every_operation(&query),
                &mut (),
            ) {
                Err(CmgError::Cancelled { phase }) => {
                    phases.insert(phase);
                }
                other => panic!("poll {stop} did not cancel: {other:?}"),
            }
            assert_eq!(cancel.polls(), stop + 1);
            // Every level buffer was returned: the next ordinary application
            // reproduces the reference bits.
            preconditioner
                .apply_compatible_into(&rhs, &mut output, &mut workspace)
                .unwrap();
            assert_eq!(bits(&output), bits(&expected));
        }
        for phase in [
            "CMG cycle diagonal",
            "Laplacian matvec",
            "CMG cycle smoothing",
            "CMG restriction",
            "CMG centering sums",
            "CMG centering",
            "CMG prolongation",
            "terminal forward solve",
            "terminal backward solve",
            "terminal scatter",
        ] {
            assert!(
                phases.contains(phase),
                "{phase} not interruptible: {phases:?}"
            );
        }
    }

    #[test]
    fn public_cancellable_application_polls_inside_one_cycle() {
        let (graph, preconditioner, rhs) = fixture(20_000, 64);
        let mut workspace = preconditioner.try_workspace().unwrap();
        let mut expected = vec![0.0; graph.vertex_count()];
        preconditioner
            .apply_compatible_into(&rhs, &mut expected, &mut workspace)
            .unwrap();
        let count = StopAt::never();
        let query = || count.query();
        let context = ApplyContext {
            cancel: Some(&query),
            ..ApplyContext::default()
        };
        let mut output = vec![0.0; graph.vertex_count()];
        preconditioner
            .apply_compatible_into_with(&rhs, &mut output, &mut workspace, context)
            .unwrap();
        assert_eq!(bits(&output), bits(&expected));
        // Polls at the fixed cadence inside the cycle plus one at completion.
        let polls = count.polls();
        assert!(polls >= 3, "{polls}");
        for stop in [0, polls / 2, polls - 1] {
            let cancel = StopAt::at(Some(stop));
            let query = || cancel.query();
            let context = ApplyContext {
                cancel: Some(&query),
                ..ApplyContext::default()
            };
            assert!(matches!(
                preconditioner.apply_compatible_into_with(
                    &rhs,
                    &mut output,
                    &mut workspace,
                    context
                ),
                Err(CmgError::Cancelled { .. })
            ));
            assert_eq!(cancel.polls(), stop + 1);
        }
        // A cancellation query that never fires does not change a fatal error.
        let never = || false;
        let mut bad = rhs.clone();
        bad[7] = f64::NAN;
        let projected = preconditioner.apply_projected_into(
            &bad,
            &mut output,
            &mut workspace,
            crate::ValidationOptions::default(),
            ApplyContext {
                cancel: Some(&never),
                ..ApplyContext::default()
            },
        );
        assert!(matches!(
            projected,
            Err(CmgError::NonFiniteMatrixValue { .. })
        ));
        preconditioner
            .apply_compatible_into(&rhs, &mut output, &mut workspace)
            .unwrap();
        assert_eq!(bits(&output), bits(&expected));
        let _ = Unmetered;
    }

    #[cfg(feature = "parallel")]
    #[test]
    fn planned_and_interleaved_cycles_join_lanes_before_cancelling() {
        use crate::{CmgBatchWorkspace, ParallelCmgPlan, ParallelExecutor, ParallelOptions};
        use std::sync::atomic::{AtomicUsize, Ordering};

        let (graph, preconditioner, rhs) = fixture(60_000, 64);
        let executor = ParallelExecutor::new(ParallelOptions {
            threads: 4,
            min_parallel_len: 64,
            reduction_chunk_size: 256,
            ..ParallelOptions::default()
        })
        .unwrap();
        let team = executor.team();
        let plan = ParallelCmgPlan::build(&preconditioner, team).unwrap();
        assert!(plan.operator_count() > 0);
        let planned = ApplyContext {
            plan: Some(&plan),
            team: Some(team),
            cancel: None,
        };
        let n = graph.vertex_count();
        let mut workspace = preconditioner.try_workspace().unwrap();
        let mut expected = vec![0.0; n];
        preconditioner
            .apply_compatible_into_with(&rhs, &mut expected, &mut workspace, planned)
            .unwrap();
        let columns = 3;
        let interleaved: Vec<f64> = rhs.iter().flat_map(|&value| [value; 3]).collect();
        let mut batch = CmgBatchWorkspace::try_new(&preconditioner, columns, usize::MAX).unwrap();
        let mut batch_expected = vec![0.0; n * columns];
        preconditioner
            .apply_compatible_batch_into(
                &interleaved,
                &mut batch_expected,
                columns,
                &mut batch,
                planned,
            )
            .unwrap();

        // Queries of one uncancelled planned application (coordinator and lanes).
        let total = {
            let calls = AtomicUsize::new(0);
            let query = || {
                calls.fetch_add(1, Ordering::SeqCst);
                false
            };
            let mut output = vec![0.0; n];
            preconditioner
                .apply_compatible_into_with(
                    &rhs,
                    &mut output,
                    &mut workspace,
                    ApplyContext {
                        cancel: Some(&query),
                        ..planned
                    },
                )
                .unwrap();
            assert_eq!(bits(&output), bits(&expected));
            calls.load(Ordering::SeqCst)
        };
        assert!(total > 8, "{total} queries");
        for threshold in [1, total / 4, total / 2, total - 1] {
            // Sticky after `threshold` queries from the coordinator or any lane.
            let calls = AtomicUsize::new(0);
            let query = || calls.fetch_add(1, Ordering::SeqCst) >= threshold;
            let context = ApplyContext {
                cancel: Some(&query),
                ..planned
            };
            let mut output = vec![0.0; n];
            assert!(matches!(
                preconditioner.apply_compatible_into_with(
                    &rhs,
                    &mut output,
                    &mut workspace,
                    context
                ),
                Err(CmgError::Cancelled { .. })
            ));
            // Returned only after every lane joined: no lane still queries.
            let settled = calls.load(Ordering::SeqCst);
            std::thread::sleep(std::time::Duration::from_millis(5));
            assert_eq!(calls.load(Ordering::SeqCst), settled);
            preconditioner
                .apply_compatible_into_with(&rhs, &mut output, &mut workspace, planned)
                .unwrap();
            assert_eq!(bits(&output), bits(&expected));

            let calls = AtomicUsize::new(0);
            let query = || calls.fetch_add(1, Ordering::SeqCst) >= threshold;
            let context = ApplyContext {
                cancel: Some(&query),
                ..planned
            };
            let mut output = vec![0.0; n * columns];
            assert!(matches!(
                preconditioner.apply_compatible_batch_into(
                    &interleaved,
                    &mut output,
                    columns,
                    &mut batch,
                    context
                ),
                Err(CmgError::Cancelled { .. })
            ));
            preconditioner
                .apply_compatible_batch_into(
                    &interleaved,
                    &mut output,
                    columns,
                    &mut batch,
                    planned,
                )
                .unwrap();
            assert_eq!(bits(&output), bits(&batch_expected));
        }
    }
}
