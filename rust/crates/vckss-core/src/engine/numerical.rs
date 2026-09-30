// SPDX-License-Identifier: GPL-3.0-only
use super::*;
use crate::generic_jla::NumericalRhsPhase;
use crate::numerical_mc::{self, Covariance, FiniteDerivative, Primitive, Status, Sum};

#[derive(Clone, Debug)]
pub struct CompressedNumericalReplayRhs {
    pub phase: NumericalRhsPhase,
    pub probe: u32,
    pub solve: JlaRhsReceipt,
}

#[derive(Clone, Debug)]
pub struct CompressedNumericalMcResult {
    pub covariance: Covariance,
    pub leverage_probes: u32,
    pub target_probes: u32,
    pub folds: [u32; 2],
    /// Owned replay work, never appended to statistical receipts.
    pub replay_rhs: Vec<CompressedNumericalReplayRhs>,
    /// Executor-accounted completed work. Failed non-CMG batches return no
    /// completed-batch receipt; attempted directions are counted separately.
    pub replay_executed_rhs_count: usize,
    pub replay_attempted_rhs_count: usize,
    pub replay_generator_word_evaluations: u64,
    pub replay_error: Option<BackendError>,
    pub failed_replay_probe: Option<u32>,
    pub score_means: [[f64; 3]; 2],
    pub allocation_bound_bytes: u64,
    pub minimum_constrained: f64,
    pub minimum_residual_margin: f64,
    pub maximum_sensitivity_ratio: f64,
}

pub(super) fn allocation_bound(groups: usize, probes: u32) -> Result<u64> {
    let group = groups
        .checked_mul(6 * 8 + 6 * 16 + 4 * 8 + 4 * 16)
        .ok_or_else(|| memory_overflow("numerical groups"))?;
    let score = (probes as usize)
        .checked_mul(72 + size_of::<CompressedNumericalReplayRhs>() + 48)
        .ok_or_else(|| memory_overflow("numerical scores"))?;
    u64::try_from(group)
        .ok()
        .and_then(|n| n.checked_add(score as u64))
        .and_then(|n| n.checked_add(4096))
        .ok_or_else(|| memory_overflow("numerical attachment"))
}

pub(super) struct State {
    gradients: Vec<[f64; 5]>,
    offsets: Vec<f64>,
    folds: Vec<[Sum; 6]>,
    r: u32,
    batch: usize,
    status: Status,
    minimum_constrained: f64,
    minimum_residual_margin: f64,
    maximum_sensitivity_ratio: f64,
}

impl State {
    pub(super) fn new(
        plan: &JlaPlan,
        options: JlaEngineOptions,
        moments: &[FiveMoments],
        clipped: bool,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        let n = plan.deletion_units();
        let r = f64::from(options.probes);
        let mut state = Self {
            gradients: statistical_buffer(n, [0.0; 5], interrupt)?,
            offsets: statistical_buffer(n, 0.0, interrupt)?,
            folds: statistical_buffer(n, [Sum::default(); 6], interrupt)?,
            r: options.probes,
            batch: options.leverage_batch_width.clamp(1, 4),
            status: if clipped {
                Status::NonsmoothAdjustment
            } else {
                Status::OkLocal
            },
            minimum_constrained: f64::INFINITY,
            minimum_residual_margin: f64::INFINITY,
            maximum_sensitivity_ratio: 0.0,
        };
        for (g, m) in moments.iter().enumerate() {
            checkpoint_chunk(interrupt, g, "compressed_numerical_derivative")?;
            let u = [
                m.projection.finish() / r,
                m.residual.finish() / r,
                m.projection_fourth.finish() / r,
                m.residual_fourth.finish() / r,
                m.mixed.finish() / r,
            ];
            let Some(finite) = FiniteDerivative::new(u, options.probes) else {
                state.status = Status::NonfiniteDerivative;
                continue;
            };
            state.minimum_constrained = state.minimum_constrained.min(u[0] + u[1]);
            state.minimum_residual_margin = state.minimum_residual_margin.min(finite.m);
            state.maximum_sensitivity_ratio = state
                .maximum_sensitivity_ratio
                .max(finite.variance.max(0.0).sqrt() / finite.m);
            if finite.nonsmooth {
                state.status = Status::NonsmoothAdjustment;
                continue;
            }
            let Some(gradient) = finite.observation(0.0) else {
                state.status = Status::NonfiniteDerivative;
                continue;
            };
            state.gradients[g] = gradient;
            let mut offset = Sum::default();
            for (gradient, u) in gradient.into_iter().zip(u) {
                offset.add(gradient * u)
            }
            state.offsets[g] = offset.get();
        }
        Ok(state)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn target(
        &mut self,
        problem: &CompressedProblem,
        plan: &JlaPlan,
        adjustment: &LeverageAdjustment,
        worker_side: (&[f64], &[f64]),
        firm_side: (&[f64], &[f64]),
        probe: usize,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        let fold = probe % 2;
        let count = if fold == 0 {
            self.r.div_ceil(2)
        } else {
            self.r / 2
        } as f64;
        for g in 0..plan.deletion_units() {
            checkpoint_chunk(interrupt, g, "compressed_numerical_target")?;
            let cell = plan.deletion.cell[g] as usize;
            let w = problem.cell_worker[cell] as usize;
            let f = problem.cell_firm[cell] as usize;
            let sw = worker_side.0[w] + worker_side.1[f];
            let sf = firm_side.0[w] + firm_side.1[f];
            let mass = plan.deletion.outcome_sum[g] * adjustment.residual_mass[g] / count;
            for (j, kappa) in [sw * sw, sf * sf, sw * sf].into_iter().enumerate() {
                self.folds[g][fold * 3 + j].add(mass * kappa)
            }
        }
        Ok(())
    }

    pub(super) fn replay(
        self,
        problem: &CompressedProblem,
        plan: &JlaPlan,
        solver: &PreparedTwoWaySolver<'_>,
        rng: CounterRng,
        draws: &[VarianceComponents],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<CompressedNumericalMcResult> {
        let draws: Vec<_> = draws
            .iter()
            .map(|v| Primitive {
                worker: v.worker,
                firm: v.firm,
                covariance: v.covariance,
            })
            .collect();
        let conditional = numerical_mc::conditional_covariance(&draws)?;
        drop(draws);
        let mut status = self.status;
        let mut scores = [
            statistical_buffer(self.r as usize, Primitive::default(), interrupt)?,
            statistical_buffer(self.r as usize, Primitive::default(), interrupt)?,
        ];
        let mut receipts = Vec::new();
        reserve_exact(
            &mut receipts,
            self.r as usize,
            "compressed numerical replay receipts",
        )?;
        let mut evaluations = 0_u64;
        let mut replay_error = None;
        let mut failed_replay_probe = None;
        let direct_before = solver.full_cmg_receipt()?;
        let mut attempted = 0;
        let groups = plan.deletion_units();
        let mut atoms = statistical_buffer(groups * self.batch, 0_i64, interrupt)?;
        let mut worker = statistical_buffer(problem.workers() * self.batch, 0.0, interrupt)?;
        let mut firm = statistical_buffer(problem.firms() * self.batch, 0.0, interrupt)?;
        if status == Status::OkLocal {
            for first in (0..self.r as usize).step_by(self.batch) {
                interrupt.checkpoint("compressed_numerical_replay")?;
                let width = self.batch.min(self.r as usize - first);
                rng.fill_rademacher_sums_with_interrupt(
                    ProbeDomain::Leverage,
                    first as u64,
                    width,
                    &plan.deletion.semantic_rank,
                    &plan.deletion.physical_count,
                    &mut atoms[..groups * width],
                    interrupt,
                )?;
                for count in &plan.deletion.physical_count {
                    evaluations = evaluations
                        .checked_add(count.div_ceil(64) * width as u64)
                        .ok_or_else(|| memory_overflow("compressed replay work"))?;
                }
                worker.fill(0.0);
                firm.fill(0.0);
                for column in 0..width {
                    for group in 0..groups {
                        checkpoint_chunk(interrupt, group, "compressed_numerical_replay_rhs")?;
                        let cell = plan.deletion.cell[group] as usize;
                        let atom = atoms[column * groups + group] as f64;
                        worker[column * problem.workers() + problem.cell_worker[cell] as usize] +=
                            atom;
                        firm[column * problem.firms() + problem.cell_firm[cell] as usize] += atom;
                    }
                }
                attempted += width;
                let solved = match solver.solve_batch_with_interrupt(
                    &worker[..problem.workers() * width],
                    &firm[..problem.firms() * width],
                    width,
                    interrupt,
                ) {
                    Ok(s) => s,
                    Err(e)
                        if matches!(
                            e.code,
                            ErrorCode::PcgMaxIterations
                                | ErrorCode::PcgStagnation
                                | ErrorCode::PcgCurvatureBreakdown
                                | ErrorCode::PcgPreconditionerBreakdown
                                | ErrorCode::FullResidualFailed
                        ) =>
                    {
                        status = Status::ReplayFailed;
                        failed_replay_probe = Some(first as u32);
                        replay_error = Some(e);
                        break;
                    }
                    Err(e) => return Err(e),
                };
                for column in 0..width {
                    let probe = first + column;
                    let solution = &solved.solution[column];
                    receipts.push(CompressedNumericalReplayRhs {
                        phase: NumericalRhsPhase::LeverageReplay,
                        probe: probe as u32,
                        solve: rhs_receipt(
                            &solved.receipt[column],
                            solution.residual.relative_norm,
                            solution.residual.rhs_norm,
                            JlaSolvePhase::Leverage,
                            Some(probe as u64),
                            JlaRhsSide::Joint,
                        )?,
                    });
                    let mut score = [[Sum::default(); 3]; 2];
                    for g in 0..plan.deletion_units() {
                        checkpoint_chunk(interrupt, g, "compressed_numerical_replay_score")?;
                        let cell = plan.deletion.cell[g] as usize;
                        let w = problem.cell_worker[cell] as usize;
                        let f = problem.cell_firm[cell] as usize;
                        let root = (plan.deletion.physical_count[g] as f64).sqrt();
                        let p = root * (solution.worker[w] + solution.firm[f]);
                        let m = atoms[column * groups + g] as f64 / root - p;
                        let u = [p * p, m * m, p.powi(4), m.powi(4), p * p * m * m];
                        let mut alpha = Sum::default();
                        alpha.add(-self.offsets[g]);
                        for (gradient, u) in self.gradients[g].into_iter().zip(u) {
                            alpha.add(gradient * u)
                        }
                        for (fold, score) in score.iter_mut().enumerate() {
                            for (j, s) in score.iter_mut().enumerate() {
                                s.add(-self.folds[g][fold * 3 + j].get() * alpha.get())
                            }
                        }
                    }
                    for fold in 0..2 {
                        scores[fold][probe] = Primitive::from_values(score[fold].map(Sum::get))
                    }
                }
            }
        }
        let mut means = [[0.0; 3]; 2];
        for fold in 0..2 {
            let mut sum = [Sum::default(); 3];
            for score in &scores[fold] {
                for (j, v) in score.values().into_iter().enumerate() {
                    sum[j].add(v / f64::from(self.r))
                }
            }
            means[fold] = sum.map(Sum::get);
        }
        let mut covariance = if status == Status::OkLocal {
            match numerical_mc::cross_covariance(&scores[0], &scores[1]) {
                Ok(leverage) => numerical_mc::finalize(conditional, leverage),
                Err(_) => {
                    status = Status::NonfiniteDerivative;
                    numerical_mc::finalize(conditional, [[f64::NAN; 3]; 3])
                }
            }
        } else {
            numerical_mc::finalize(conditional, [[f64::NAN; 3]; 3])
        };
        if status != Status::OkLocal {
            covariance.status = status;
        }
        let replay_executed_rhs_count = match (direct_before, solver.full_cmg_receipt()?) {
            (Some(before), Some(after)) => {
                let count = after
                    .rhs_count
                    .checked_sub(before.rhs_count)
                    .and_then(|n| usize::try_from(n).ok())
                    .ok_or_else(|| {
                        BackendError::invariant(
                            "compressed_numerical",
                            "replay work count overflow",
                        )
                    })?;
                if count < receipts.len() || count > attempted {
                    return Err(BackendError::invariant(
                        "compressed_numerical",
                        "replay work does not reconcile",
                    ));
                }
                count
            }
            (None, None) => receipts.len(),
            _ => {
                return Err(BackendError::invariant(
                    "compressed_numerical",
                    "replay executor identity changed",
                ))
            }
        };
        Ok(CompressedNumericalMcResult {
            covariance,
            leverage_probes: self.r,
            target_probes: self.r,
            folds: [self.r.div_ceil(2), self.r / 2],
            replay_rhs: receipts,
            replay_executed_rhs_count,
            replay_attempted_rhs_count: attempted,
            replay_generator_word_evaluations: evaluations,
            replay_error,
            failed_replay_probe,
            score_means: means,
            allocation_bound_bytes: allocation_bound(plan.deletion_units(), self.r)?,
            minimum_constrained: self.minimum_constrained,
            minimum_residual_margin: self.minimum_residual_margin,
            maximum_sensitivity_ratio: self.maximum_sensitivity_ratio,
        })
    }
}
