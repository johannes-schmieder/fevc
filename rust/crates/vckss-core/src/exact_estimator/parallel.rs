// SPDX-License-Identifier: GPL-3.0-only

//! Isolated exact-execution prototype. No legacy entrypoint or ABI selects it.

use super::*;
use crate::model_operator::{checked_matrix_length, reserve_exact, zeroed_f64_with_interrupt};
use std::sync::atomic::{AtomicUsize, Ordering};

#[derive(Clone, Debug)]
pub struct ParallelExactResult {
    pub estimator: ExactEstimatorResult,
    pub correction_sources: Option<[VarianceComponents; 2]>,
    pub requested_threads: usize,
    pub worker_limit: usize,
    pub parallel_regions: usize,
    pub incremental_forecast_bytes: u64,
}

/// Experimental core entrypoint only. The installed/native exact interfaces
/// remain serial until a separate capability, transport and receipt are tested.
pub fn run_with_interrupt(
    problem: &CompressedProblem,
    options: ExactEstimatorOptions,
    hybrid: Option<&ExactStayerHybridPlan>,
    threads: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<ParallelExactResult> {
    let (options, workers, incremental) =
        admitted_plan(problem, options, hybrid, threads, interrupt)?;
    interrupt.checkpoint("exact_parallel_admitted")?;
    let runtime = Runtime::new(workers)?;
    let (estimator, correction_sources) =
        run_exact_estimator_internal(problem, options, hybrid, Some(&runtime), interrupt)?;
    Ok(ParallelExactResult {
        estimator,
        correction_sources,
        requested_threads: threads,
        worker_limit: workers,
        parallel_regions: runtime.regions.load(Ordering::Relaxed),
        incremental_forecast_bytes: incremental,
    })
}

/// Allocation-free command preflight for callers with more than one exact
/// pass. The same deterministic admission is repeated at execution entry.
pub fn preflight(
    problem: &CompressedProblem,
    options: ExactEstimatorOptions,
    hybrid: Option<&ExactStayerHybridPlan>,
    threads: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    admitted_plan(problem, options, hybrid, threads, interrupt).map(|_| ())
}

pub fn wall_work(problem: &CompressedProblem, options: ExactEstimatorOptions) -> Result<WallWork> {
    exact_wall_work(problem, options.validate()?)
}

fn admitted_plan(
    problem: &CompressedProblem,
    options: ExactEstimatorOptions,
    hybrid: Option<&ExactStayerHybridPlan>,
    threads: usize,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(ExactEstimatorOptions, usize, u64)> {
    let mut options = options.validate()?;
    if threads == 0 || problem.outcome.is_empty() || problem.firms() < 2 {
        return Err(BackendError::invalid(
            "exact_parallel",
            "positive threads and valid model dimensions are required",
        ));
    }
    if let Some(hybrid) = hybrid {
        validate_stayer_hybrid_plan(problem, hybrid, options, interrupt)?;
    }
    let full = problem
        .workers()
        .checked_add(problem.firms())
        .and_then(|n| n.checked_add(problem.controls.len()))
        .ok_or_else(|| resource_error("exact parallel dimension overflow"))?;
    if full - 1 > options.exact_limit {
        return Err(resource_error(
            "identified coefficient dimension exceeds exact_limit()",
        ));
    }
    let working = if options.nuisance == NuisanceMode::FixedOffset {
        full - problem.controls.len()
    } else {
        full
    };
    let workers = threads.min(problem.outcome.len().max(full)).max(1);
    let block = if options.deletion == DeletionMode::Match {
        (0..problem.deletion_units())
            .map(|g| problem.deletion_index.range(g).len())
            .max()
            .unwrap_or(0)
    } else {
        1
    };
    if block > options.blocksize_limit {
        return Err(resource_error("a deletion block exceeds blocksize_limit()"));
    }
    let p = working as u64;
    let b = block as u64;
    // Additional simultaneous deletion scratch, not copies of the N-by-P
    // design or shared information/inverse. Includes the direct rank fallback.
    let scratch = checked_add_many(&[
        checked_scale(
            p.checked_mul(p)
                .ok_or_else(|| resource_error("exact scratch overflow"))?,
            7 * 8,
            "exact square scratch overflow",
        )?,
        checked_scale(
            b.checked_mul(p)
                .ok_or_else(|| resource_error("exact block scratch overflow"))?,
            3 * 8,
            "exact block scratch overflow",
        )?,
        checked_scale(
            b.min(p)
                .checked_mul(b.min(p))
                .ok_or_else(|| resource_error("exact maker scratch overflow"))?,
            3 * 8,
            "exact maker scratch overflow",
        )?,
        checked_scale(b, 9 * 8, "exact block vector scratch overflow")?,
        checked_scale(p, 4 * 8, "exact parameter scratch overflow")?,
        64 * 1024,
    ])?;
    let incremental = checked_add_many(&[
        checked_scale(
            scratch,
            workers.saturating_sub(1) as u64,
            "exact concurrent scratch overflow",
        )?,
        checked_scale(
            problem.outcome.len().max(problem.deletion_units()) as u64,
            size_of::<Contribution>() as u64,
            "exact ordered contribution storage overflow",
        )?,
        crate::ordered_work::metadata_bytes(workers)?,
        if workers == 1 {
            0
        } else {
            checked_scale(
                workers as u64,
                2 * 1024 * 1024 + 128 * 1024,
                "exact runtime storage overflow",
            )?
        },
    ])?;
    options.prepared_persistent_bytes = options
        .prepared_persistent_bytes
        .checked_add(incremental)
        .ok_or_else(|| resource_error("exact parallel command peak overflow"))?;
    let memory = exact_peak_forecast(problem, full, working, options)?;
    options
        .memory_budget
        .admit(memory.peak, options.memory_limit_bytes, "exact_parallel")?;
    Ok((options, workers, incremental))
}

pub(super) struct Runtime {
    pool: Option<rayon::ThreadPool>,
    workers: usize,
    regions: AtomicUsize,
}

impl Runtime {
    pub(super) fn workers(&self) -> usize {
        self.workers
    }
    fn new(workers: usize) -> Result<Self> {
        let pool = if workers == 1 {
            None
        } else {
            Some(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(workers)
                    .stack_size(2 * 1024 * 1024)
                    .thread_name(|i| format!("fevc-exact-{i}"))
                    .build()
                    .map_err(|e| {
                        BackendError::new(
                            ErrorCode::AllocationFailed,
                            "exact_parallel",
                            e.to_string(),
                        )
                    })?,
            )
        };
        Ok(Self {
            pool,
            workers,
            regions: AtomicUsize::new(0),
        })
    }

    fn run<
        I: Send,
        Iter: ExactSizeIterator<Item = I>,
        F: Fn(usize, I, &mut dyn InterruptCheck) -> Result<()> + Sync,
    >(
        &self,
        input: Iter,
        phase: &'static str,
        operation: F,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        if input.len() > 1 && self.pool.is_some() {
            self.regions.fetch_add(1, Ordering::Relaxed);
        }
        crate::ordered_work::run(
            self.pool.as_ref().map(crate::ordered_work::Pool::Rayon),
            self.workers,
            input,
            phase,
            operation,
            interrupt,
        )
    }

    pub(super) fn crossproduct(
        &self,
        design: &[f64],
        rows: usize,
        parameters: usize,
        frequency: &[u64],
        interrupt: &mut dyn InterruptCheck,
        phase: &'static str,
    ) -> Result<Vec<f64>> {
        let mut output = zeroed_f64_with_interrupt(
            checked_matrix_length(parameters, parameters, phase)?,
            phase,
            interrupt,
            phase,
        )?;
        let chunk_rows = parameters.div_ceil(self.workers).max(1);
        let chunk = checked_matrix_length(chunk_rows, parameters, phase)?;
        self.run(
            output.chunks_mut(chunk),
            phase,
            |job, output, interrupt| {
                for (local, output) in output.chunks_mut(parameters).enumerate() {
                    let left = job * chunk_rows + local;
                    let mut work = 0_usize;
                    for row in 0..rows {
                        let scaled = u64_to_f64(frequency[row])? * design[row * parameters + left];
                        if scaled == 0.0 {
                            continue;
                        }
                        for right in 0..=left {
                            checkpoint_chunk(interrupt, work, phase)?;
                            work = work.saturating_add(1);
                            output[right] += scaled * design[row * parameters + right];
                        }
                    }
                }
                Ok(())
            },
            interrupt,
        )?;
        for left in 0..parameters {
            for right in 0..left {
                checkpoint_chunk(interrupt, left * parameters + right, phase)?;
                output[right * parameters + left] = output[left * parameters + right];
            }
        }
        Ok(output)
    }

    pub(super) fn multiply(
        &self,
        left: &[f64],
        rows: usize,
        inner: usize,
        right: &[f64],
        columns: usize,
        interrupt: &mut dyn InterruptCheck,
        phase: &'static str,
    ) -> Result<Vec<f64>> {
        if left.len() != checked_matrix_length(rows, inner, phase)?
            || right.len() != checked_matrix_length(inner, columns, phase)?
        {
            return Err(BackendError::invalid(
                phase,
                "dense multiplication dimensions disagree",
            ));
        }
        let mut output = zeroed_f64_with_interrupt(
            checked_matrix_length(rows, columns, phase)?,
            phase,
            interrupt,
            phase,
        )?;
        let chunk_rows = rows.div_ceil(self.workers).max(1);
        let chunk = checked_matrix_length(chunk_rows, columns, phase)?;
        self.run(
            output.chunks_mut(chunk),
            phase,
            |job, output, interrupt| {
                let mut work = 0_usize;
                for (local, output) in output.chunks_mut(columns).enumerate() {
                    let row = job * chunk_rows + local;
                    for (column, value) in output.iter_mut().enumerate() {
                        for index in 0..inner {
                            checkpoint_chunk(interrupt, work, phase)?;
                            work = work.saturating_add(1);
                            *value += left[row * inner + index] * right[index * columns + column];
                        }
                    }
                }
                Ok(())
            },
            interrupt,
        )?;
        Ok(output)
    }
}

#[derive(Clone, Copy)]
struct Contribution {
    value: VarianceComponents,
    scale: f64,
    leverage: f64,
    maker_relres: f64,
    gap: f64,
    units: u64,
}

impl Default for Contribution {
    fn default() -> Self {
        Self {
            value: VarianceComponents::default(),
            scale: 0.0,
            leverage: 0.0,
            maker_relres: 0.0,
            gap: f64::INFINITY,
            units: 0,
        }
    }
}

pub(super) struct CorrectionContext<'a, 'p> {
    pub problem: &'a CompressedProblem,
    pub design: &'a [f64],
    pub design_inverse: &'a [f64],
    pub inverse: &'a DenseInverse,
    pub inverse_factor: Option<&'a [f64]>,
    pub information: &'a [f64],
    pub outcome: &'a [f64],
    pub residual: &'a [f64],
    pub target: &'a TargetMoments<'p>,
    pub embedding: usize,
    pub options: ExactEstimatorOptions,
    pub rank_margin: f64,
    pub controlled_joint: bool,
    pub control_forward: f64,
}

pub(super) struct CorrectionResult {
    pub correction: VarianceComponents,
    pub sources: Option<([VarianceComponents; 2], u64)>,
    pub units: u64,
    pub leverage: f64,
    pub maker_relres: f64,
    pub gap: f64,
}

impl CorrectionContext<'_, '_> {
    fn observation(&self, row: usize, interrupt: &mut dyn InterruptCheck) -> Result<Contribution> {
        let z = &self.design_inverse[row * self.embedding..(row + 1) * self.embedding];
        let leverage = row_dot(
            self.design,
            row,
            self.embedding,
            z,
            interrupt,
            "exact_observation_leverage_matvec",
        )?;
        if !leverage.is_finite() || leverage < -100.0 * self.options.rank_tolerance {
            return Err(nonestimable("physical observation leverage is invalid"));
        }
        let maker = 1.0 - leverage;
        if maker <= self.options.block_tolerance {
            return Err(nonestimable(
                "physical observation deletion has leverage at or above one",
            ));
        }
        if self.controlled_joint {
            enforce_downstream_bound(
                self.control_forward,
                maker,
                "observation-deletion conditioning cannot certify control-basis invariance",
            )?;
        }
        if maker <= self.rank_margin || self.controlled_joint {
            certify_deleted_information(
                self.information,
                &self.design[row * self.embedding..(row + 1) * self.embedding],
                1,
                self.embedding,
                self.problem.workers()..self.problem.workers() + self.problem.firms(),
                self.options.rank_tolerance,
                interrupt,
                "exact_observation_deleted_information",
            )?;
        }
        Ok(Contribution {
            value: self.target.bilinear(
                z,
                z,
                interrupt,
                "exact_observation_target_bilinear",
                "exact_observation_target_bilinear_cells",
            )?,
            scale: u64_to_f64(self.problem.frequency[row])?
                * self.outcome[row]
                * self.residual[row]
                / maker,
            leverage,
            gap: maker,
            units: self.problem.frequency[row],
            maker_relres: 0.0,
        })
    }

    fn matched(&self, group: usize, interrupt: &mut dyn InterruptCheck) -> Result<Contribution> {
        let range = self.problem.deletion_index.range(group);
        let mut indices = Vec::new();
        reserve_exact(&mut indices, range.len(), "exact parallel block rows")?;
        for &row in &self.problem.deletion_index.items[range] {
            indices.push(row as usize);
        }
        let block = match_block_action(
            self.design,
            self.inverse,
            self.inverse_factor.ok_or_else(|| {
                BackendError::invariant("exact_parallel", "missing inverse factor")
            })?,
            self.information,
            self.outcome,
            self.residual,
            &self.problem.frequency,
            self.embedding,
            self.problem.workers()..self.problem.workers() + self.problem.firms(),
            &indices,
            self.options,
            self.rank_margin,
            self.controlled_joint,
            self.control_forward,
            interrupt,
        )?;
        let left = transpose_matvec(
            &block.block_inverse,
            indices.len(),
            self.embedding,
            &block.transformed_outcome,
            interrupt,
            "exact_match_target_left_transpose",
        )?;
        let right = transpose_matvec(
            &block.block_inverse,
            indices.len(),
            self.embedding,
            &block.deleted_residual,
            interrupt,
            "exact_match_target_right_transpose",
        )?;
        Ok(Contribution {
            value: self.target.bilinear(
                &left,
                &right,
                interrupt,
                "exact_match_target_bilinear",
                "exact_match_target_bilinear_cells",
            )?,
            scale: 1.0,
            leverage: block.max_leverage,
            maker_relres: block.relres,
            gap: 1.0 - block.max_leverage,
            units: 1,
        })
    }

    pub(super) fn run(
        &self,
        runtime: &Runtime,
        hybrid: Option<&ExactStayerHybridPlan>,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<CorrectionResult> {
        let mut result = CorrectionResult {
            correction: VarianceComponents::default(),
            sources: None,
            units: 0,
            leverage: 0.0,
            maker_relres: 0.0,
            gap: f64::INFINITY,
        };
        let matched = self.options.deletion == DeletionMode::Match;
        let count = if matched {
            hybrid.map_or(self.problem.deletion_units(), |h| h.mover_deletion_units)
        } else {
            self.problem.outcome.len()
        };
        let passes = if hybrid.is_some() { 2 } else { 1 };
        for pass in 0..passes {
            let count = if pass == 0 {
                count
            } else {
                self.problem.outcome.len()
            };
            let mut values = Vec::new();
            reserve_exact(&mut values, count, "exact ordered contributions")?;
            values.resize(count, Contribution::default());
            let chunk = count.div_ceil(runtime.workers).max(1);
            runtime.run(
                values.chunks_mut(chunk),
                "exact_parallel_correction",
                |job, values, interrupt| {
                    for (local, value) in values.iter_mut().enumerate() {
                        interrupt.checkpoint("exact_parallel_deletion")?;
                        let index = job * chunk + local;
                        if pass == 1 && !hybrid.expect("hybrid pass").stayer_rows[index] {
                            continue;
                        }
                        *value = if matched && pass == 0 {
                            self.matched(index, interrupt)?
                        } else {
                            self.observation(index, interrupt)?
                        };
                    }
                    Ok(())
                },
                interrupt,
            )?;
            let mut correction = VarianceComponents::default();
            for (index, value) in values.into_iter().enumerate() {
                checkpoint_chunk(interrupt, index, "exact_parallel_collect")?;
                add_scaled(&mut correction, value.value, value.scale);
                result.units = result
                    .units
                    .checked_add(value.units)
                    .ok_or_else(|| resource_error("exact deletion count overflow"))?;
                result.leverage = result.leverage.max(value.leverage);
                result.maker_relres = result.maker_relres.max(value.maker_relres);
                result.gap = result.gap.min(value.gap);
            }
            correction.total = correction.worker + correction.firm + 2.0 * correction.covariance;
            correction.verify_accounting(1e-9)?;
            if pass == 0 {
                result.correction = correction;
            } else {
                result.sources = Some(([result.correction, correction], result.units));
                result.correction = add_components(result.correction, correction)?;
            }
        }
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::{MemoryBudget, MemoryCheck};

    fn equivalent(left: &ExactEstimatorResult, right: &ExactEstimatorResult) {
        let values = |c: VarianceComponents| [c.worker, c.firm, c.covariance, c.total];
        for (left, right) in values(left.corrected)
            .into_iter()
            .zip(values(right.corrected))
        {
            assert!(
                (left - right).abs() <= 1e-8 * left.abs().max(right.abs()).max(1.0),
                "{left} != {right}"
            );
        }
        assert_eq!(left.receipt.deletion_units, right.receipt.deletion_units);
        assert_eq!(left.receipt.full_fit_relres, right.receipt.full_fit_relres);
        assert_eq!(
            left.receipt.working_fit_relres,
            right.receipt.working_fit_relres
        );
        assert_eq!(left.receipt.max_leverage, right.receipt.max_leverage);
        assert_eq!(left.receipt.maker_relres, right.receipt.maker_relres);
    }

    #[test]
    fn both_deletions_controls_weights_maker_shapes_and_all_thread_caps() {
        let controls = vec![vec![1.0, -1.0, 0.0, 2.0, 1.0, 3.0, -2.0, 0.5]];
        let fixtures = [
            super::super::tests::fixture(controls, vec![1, 2, 1, 3, 1, 2, 1, 1]),
            super::super::tests::block_fixture(2),
            super::super::tests::block_fixture(9),
        ];
        for problem in fixtures {
            for deletion in [DeletionMode::Match, DeletionMode::Observation] {
                for nuisance in [NuisanceMode::Joint, NuisanceMode::FixedOffset] {
                    let options = ExactEstimatorOptions {
                        deletion,
                        nuisance,
                        memory_budget: MemoryBudget::Unspecified,
                        memory_limit_bytes: 0,
                        ..Default::default()
                    };
                    let reference = run_exact_estimator(&problem, options).unwrap();
                    for threads in [1, 2, 3, 4, 7, 14, 28, 64] {
                        let result = run_with_interrupt(
                            &problem,
                            options,
                            None,
                            threads,
                            &mut NeverInterrupt,
                        )
                        .unwrap();
                        equivalent(&reference, &result.estimator);
                        assert_eq!(result.requested_threads, threads);
                        assert!(result.worker_limit <= threads);
                        assert_eq!(result.parallel_regions > 0, threads > 1);
                        assert!(result.correction_sources.is_none());
                    }
                    let legacy = run_exact_estimator_planned(
                        &problem,
                        PlannedExactEstimatorOptions {
                            estimator: options,
                            wallseconds: None,
                        },
                    )
                    .unwrap();
                    assert_eq!(legacy.execution.threads.used, 1);
                    assert_eq!(legacy.execution.threads.parallel_regions, 0);
                }
            }
        }
    }

    #[test]
    fn hybrid_sources_budget_boundaries_cancellation_and_reuse() {
        let (problem, plan) = super::super::tests::hybrid_fixture();
        let options = ExactEstimatorOptions {
            memory_budget: MemoryBudget::Unspecified,
            memory_limit_bytes: 0,
            ..Default::default()
        };
        let reference = run_exact_stayer_hybrid(&problem, &plan, options).unwrap();
        for threads in [1, 2, 3, 4, 7, 14, 28, 64] {
            let result =
                run_with_interrupt(&problem, options, Some(&plan), threads, &mut NeverInterrupt)
                    .unwrap();
            equivalent(&reference.estimator, &result.estimator);
            let [mover, stayer] = result.correction_sources.unwrap();
            assert_eq!(mover.worker, reference.mover_correction.worker);
            assert_eq!(stayer.worker, reference.stayer_correction.worker);
            let bound = result.estimator.receipt.peak_forecast_bytes;
            for (bytes, passes) in [(bound, true), (bound - 1, false)] {
                let options = ExactEstimatorOptions {
                    memory_budget: MemoryBudget::Explicit {
                        bytes,
                        check: MemoryCheck::Error,
                    },
                    ..options
                };
                let result = run_with_interrupt(
                    &problem,
                    options,
                    Some(&plan),
                    threads,
                    &mut NeverInterrupt,
                );
                assert_eq!(result.is_ok(), passes, "{result:?}");
                if !passes {
                    assert_eq!(result.unwrap_err().code, ErrorCode::ResourceLimit);
                }
            }
        }
        struct BreakAt {
            phase: &'static str,
            caller: std::thread::ThreadId,
        }
        impl InterruptCheck for BreakAt {
            fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
                assert_eq!(self.caller, std::thread::current().id());
                if phase == self.phase {
                    Err(BackendError::new(ErrorCode::UserBreak, phase, "test break"))
                } else {
                    Ok(())
                }
            }
        }
        for phase in [
            "exact_parallel_admitted",
            "exact_full_information",
            "exact_design_inverse",
            "exact_parallel_correction",
        ] {
            let result = run_with_interrupt(
                &problem,
                options,
                Some(&plan),
                7,
                &mut BreakAt {
                    phase,
                    caller: std::thread::current().id(),
                },
            );
            assert_eq!(result.unwrap_err().code, ErrorCode::UserBreak);
            equivalent(
                &reference.estimator,
                &run_with_interrupt(&problem, options, Some(&plan), 7, &mut NeverInterrupt)
                    .unwrap()
                    .estimator,
            );
        }
        assert!(
            run_with_interrupt(&problem, options, Some(&plan), 0, &mut NeverInterrupt).is_err()
        );
        let options = ExactEstimatorOptions {
            prepared_persistent_bytes: u64::MAX,
            ..options
        };
        assert_eq!(
            run_with_interrupt(&problem, options, Some(&plan), 7, &mut NeverInterrupt)
                .unwrap_err()
                .code,
            ErrorCode::ResourceLimit
        );
    }
}
