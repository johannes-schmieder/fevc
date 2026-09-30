// SPDX-License-Identifier: GPL-3.0-only
use super::*;
#[cfg(test)]
mod cache;
#[cfg(test)]
mod tests;
use crate::numerical_mc::{self, Covariance, FiniteDerivative, Primitive, Status, Sum};

/// Separate attachment receipts; existing statistical RHS layouts are unchanged.
#[derive(Clone, Debug)]
pub struct NumericalMcResult {
    pub covariance: Covariance,
    pub leverage_probes: u32,
    pub target_probes: u32,
    pub folds: [u32; 2],
    pub replay_rhs: Vec<NumericalReplayRhs>,
    /// Executor-accounted completed logical work. Failed serial/queued batches
    /// return no completed-batch receipt. A lower CMG solve may complete before
    /// outer original-model certification rejects that replay direction.
    pub replay_executed_rhs_count: usize,
    /// All directions submitted in the last batch, including uncertified ones.
    pub replay_attempted_rhs_count: usize,
    /// Generator work only: these address existing atoms, not new samples.
    pub replay_generator_word_evaluations: u64,
    pub replay_error: Option<BackendError>,
    /// First uncertified logical direction, distinct from completed RHS work.
    pub failed_replay_probe: Option<u32>,
    pub replay_score_means: [[f64; 3]; 2],
    pub allocation_bound_bytes: u64,
    pub minimum_constrained: f64,
    pub minimum_residual_margin: f64,
    pub maximum_sensitivity_ratio: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NumericalRhsPhase {
    LeverageReplay,
}

#[derive(Clone, Debug)]
pub struct NumericalReplayRhs {
    pub phase: NumericalRhsPhase,
    pub probe: u32,
    pub pcg: ModelPcgReceipt,
    pub complete_residual: f64,
}

#[derive(Clone, Copy, Default)]
struct Block {
    beta: [f64; 5],
    offset: f64,
    t: f64,
}

pub(super) struct State {
    #[cfg(test)]
    pub(super) cache: cache::Cache,
    r: u32,
    t: u32,
    copy: Vec<[f64; 2]>,
    row: Vec<[f64; 3]>,
    offsets: Vec<usize>,
    classes: Vec<ObservationClass>,
    observation_fold: Vec<[Sum; 6]>,
    blocks: Vec<Block>,
    w: Vec<f64>,
    predictions: Vec<[f64; 2]>,
    block_fold: Vec<[Sum; 6]>,
    scores: [Vec<Primitive>; 2],
    conditional: numerical_mc::Matrix3,
    status: Status,
    bound: u64,
    minimum_constrained: f64,
    minimum_residual_margin: f64,
    maximum_sensitivity_ratio: f64,
}

fn checked_bytes(count: usize, width: usize, name: &str) -> Result<u64> {
    count
        .checked_mul(width)
        .and_then(|n| u64::try_from(n).ok())
        .ok_or_else(|| resource(format!("{name} size overflow")))
}

pub(super) fn allocation_bound(problem: &CompressedProblem, probes: u32) -> Result<u64> {
    // Conservative overlap: both derivative families, compensation, copied
    // addresses/offsets, scores, separate replay receipts, transition and export.
    // Replay is capped at four columns and reuses the baseline solver workspace.
    checked_sum(&[
        problem
            .physical_total
            .checked_mul(16)
            .ok_or_else(|| resource("numerical copy overflow"))?,
        checked_bytes(problem.outcome.len(), 640, "numerical rows")?,
        checked_bytes(
            probes as usize,
            72 + std::mem::size_of::<NumericalReplayRhs>(),
            "numerical scores, conditional transition and receipts",
        )?,
        4096,
    ])
}

pub(super) fn export_bound(probes: u32) -> Result<u64> {
    // The prepared context is gone at export, but the attachment's owned
    // Rust receipts, the C V1 export buffer and Stata's destination coexist.
    checked_sum(&[
        std::mem::size_of::<NumericalMcResult>() as u64,
        checked_bytes(
            probes as usize,
            std::mem::size_of::<NumericalReplayRhs>() + 48,
            "numerical owned and caller replay receipts",
        )?,
        4096,
    ])
}

impl State {
    pub(super) fn new(
        problem: &CompressedProblem,
        probes: u32,
        target_probes: u32,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        let rows = problem.outcome.len();
        let bound = allocation_bound(problem, probes)?;
        Ok(Self {
            #[cfg(test)]
            cache: cache::Cache::default(),
            r: probes,
            t: target_probes,
            copy: Vec::new(),
            row: Vec::new(),
            offsets: Vec::new(),
            classes: Vec::new(),
            observation_fold: Vec::new(),
            blocks: Vec::new(),
            w: Vec::new(),
            predictions: repeated(
                rows,
                [0.0; 2],
                "numerical target predictions",
                interrupt,
                "numerical_mc_allocate",
            )?,
            block_fold: Vec::new(),
            scores: [
                repeated(
                    probes as usize,
                    Primitive::default(),
                    "numerical fold scores",
                    interrupt,
                    "numerical_mc_allocate",
                )?,
                repeated(
                    probes as usize,
                    Primitive::default(),
                    "numerical fold scores",
                    interrupt,
                    "numerical_mc_allocate",
                )?,
            ],
            conditional: [[0.0; 3]; 3],
            status: Status::OkLocal,
            bound,
            minimum_constrained: f64::INFINITY,
            minimum_residual_margin: f64::INFINITY,
            maximum_sensitivity_ratio: 0.0,
        })
    }

    pub(super) fn point_clipping(&mut self, clipped: bool) {
        if clipped {
            self.status = Status::NonsmoothAdjustment;
        }
    }

    pub(super) fn observation(
        &mut self,
        problem: &CompressedProblem,
        classes: &[ObservationClass],
        moments: &ObservationMoments,
        correlations: &ObservationCorrelations,
        control: &[f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        #[cfg(test)]
        self.cache.observation_geometry(correlations, control);
        // Match-only deletion never needs physical-copy pullbacks.
        self.copy = repeated(
            problem.physical_total as usize,
            [0.0; 2],
            "numerical copy pullbacks",
            interrupt,
            "numerical_mc_allocate",
        )?;
        self.row = repeated(
            problem.outcome.len(),
            [0.0; 3],
            "numerical row pullbacks",
            interrupt,
            "numerical_mc_allocate",
        )?;
        self.observation_fold = repeated(
            problem.outcome.len(),
            [Sum::default(); 6],
            "numerical observation folds",
            interrupt,
            "numerical_mc_allocate",
        )?;
        self.classes = classes.to_vec();
        self.offsets = correlations.row_offset.clone();
        let r = f64::from(self.r);
        for class in classes {
            for &row in &class.rows {
                checkpoint_chunk(interrupt, row, "numerical_mc_derivative")?;
                let a2 = moments.projection_square_sum[row] / r;
                let a4 = moments.projection_fourth_sum[row] / r;
                let mut g2 = Sum::default();
                let mut g4 = Sum::default();
                let mut offset = Sum::default();
                for physical in self.offsets[row]..self.offsets[row + 1] {
                    checkpoint_chunk(interrupt, physical, "numerical_mc_copy_derivative")?;
                    let c1 = correlations.first[physical] / r;
                    let c3 = correlations.third[physical] / r;
                    let u = [
                        a2,
                        1.0 + a2 - 2.0 * c1,
                        a4,
                        1.0 + 6.0 * a2 + a4 - 4.0 * c1 - 4.0 * c3,
                        a2 + a4 - 2.0 * c3,
                    ];
                    self.minimum_constrained = self.minimum_constrained.min(u[0] + u[1]);
                    let Some(finite) = FiniteDerivative::new(u, self.r) else {
                        self.status = Status::NonfiniteDerivative;
                        continue;
                    };
                    let ell = finite.m - control[row];
                    self.minimum_residual_margin = self.minimum_residual_margin.min(ell);
                    self.maximum_sensitivity_ratio = self
                        .maximum_sensitivity_ratio
                        .max(finite.variance.max(0.0).sqrt() / ell);
                    if finite.nonsmooth {
                        self.status = Status::NonsmoothAdjustment;
                        continue;
                    }
                    let Some(gradient) = finite.observation(control[row]) else {
                        self.status = Status::NonfiniteDerivative;
                        continue;
                    };
                    let [p2, p4, q1, q3] = numerical_mc::copy_pullback(gradient);
                    g2.add(p2);
                    g4.add(p4);
                    offset.add(q1 * c1);
                    offset.add(q3 * c3);
                    self.copy[physical] = [q1, q3];
                }
                offset.add(g2.get() * a2);
                offset.add(g4.get() * a4);
                self.row[row] = [g2.get(), g4.get(), offset.get()];
            }
        }
        if self.offsets.last().copied() != Some(problem.physical_total as usize) {
            return Err(BackendError::invariant(
                "numerical_mc",
                "copy offsets disagree",
            ));
        }
        Ok(())
    }

    pub(super) fn prepare_blocks(
        &mut self,
        count: usize,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        self.w = zeroed_f64_with_interrupt(
            self.predictions.len(),
            "numerical block direction",
            interrupt,
            "numerical_mc_allocate",
        )?;
        self.blocks = repeated(
            count,
            Block::default(),
            "numerical match derivative",
            interrupt,
            "numerical_mc_allocate",
        )?;
        self.block_fold = repeated(
            count,
            [Sum::default(); 6],
            "numerical match folds",
            interrupt,
            "numerical_mc_allocate",
        )?;
        Ok(())
    }

    pub(super) fn block(
        &mut self,
        group: usize,
        rows: &[usize],
        raw: FiveMoments,
        inverse_common: &[f64],
        t: f64,
        k: f64,
        margin: f64,
    ) {
        let r = f64::from(self.r);
        let u = [
            raw.projection.finish() / r,
            raw.residual.finish() / r,
            raw.projection_fourth.finish() / r,
            raw.residual_fourth.finish() / r,
            raw.mixed.finish() / r,
        ];
        self.minimum_constrained = self.minimum_constrained.min(u[0] + u[1]);
        self.minimum_residual_margin = self.minimum_residual_margin.min(margin);
        let Some(finite) = FiniteDerivative::new(u, self.r) else {
            self.status = Status::NonfiniteDerivative;
            return;
        };
        self.maximum_sensitivity_ratio = self
            .maximum_sensitivity_ratio
            .max(finite.variance.max(0.0).sqrt() * k);
        if finite.nonsmooth {
            self.status = Status::NonsmoothAdjustment;
            return;
        }
        let Some(beta) = finite.block(k) else {
            self.status = Status::NonfiniteDerivative;
            return;
        };
        let mut offset = Sum::default();
        for (g, u) in beta.into_iter().zip(u) {
            offset.add(g * u);
        }
        self.blocks[group] = Block {
            beta,
            offset: offset.get(),
            t,
        };
        for (&row, &w) in rows.iter().zip(inverse_common) {
            self.w[row] = w;
        }
    }

    pub(super) fn target(
        &mut self,
        problem: &CompressedProblem,
        solver: &PreparedModelSolver<'_>,
        pair: &[ModelSolve],
        y: &[f64],
        residual: &[f64],
        plan: Option<&MatchPlan>,
        probe: usize,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        #[cfg(test)]
        self.cache
            .target(problem, &self.classes, solver, pair, y, residual, plan)?;
        let fold = probe % 2;
        let denominator = if fold == 0 {
            self.t.div_ceil(2)
        } else {
            self.t / 2
        } as f64;
        for (row, prediction) in self.predictions.iter_mut().enumerate() {
            checkpoint_chunk(interrupt, row, "numerical_mc_target")?;
            for side in 0..2 {
                let c = &pair[side].coefficients;
                prediction[side] = solver
                    .operator()
                    .prediction_at(&c.worker, &c.firm, &c.control, row)?;
            }
        }
        for class in &self.classes {
            for &row in &class.rows {
                checkpoint_chunk(interrupt, row, "numerical_mc_target")?;
                let sw = self.predictions[row][0];
                let sf = self.predictions[row][1];
                for (j, kappa) in [sw * sw, sf * sf, sw * sf].into_iter().enumerate() {
                    self.observation_fold[row][fold * 3 + j]
                        .add(y[row] * residual[row] * kappa / denominator);
                }
            }
        }
        if let Some(plan) = plan {
            for (group, rows) in plan.rows.iter().enumerate() {
                let mut aw = Sum::default();
                let mut af = Sum::default();
                for &row in rows {
                    checkpoint_chunk(interrupt, row, "numerical_mc_target_block")?;
                    let f = problem.frequency[row] as f64;
                    aw.add(f * y[row] * self.predictions[row][0]);
                    af.add(f * y[row] * self.predictions[row][1]);
                }
                let aw = aw.get();
                let af = af.get();
                let mut lw = Sum::default();
                let mut lf = Sum::default();
                let mut lc = Sum::default();
                for &row in rows {
                    checkpoint_chunk(interrupt, row, "numerical_mc_target_block")?;
                    let sw = self.predictions[row][0];
                    let sf = self.predictions[row][1];
                    let factor =
                        (problem.frequency[row] as f64).sqrt() * self.w[row] * self.blocks[group].t
                            / denominator;
                    lw.add(factor * aw * sw);
                    lf.add(factor * af * sf);
                    lc.add(factor * (aw * sf + af * sw) * 0.5);
                }
                for (j, v) in [lw.get(), lf.get(), lc.get()].into_iter().enumerate() {
                    self.block_fold[group][fold * 3 + j].add(v);
                }
            }
        }
        Ok(())
    }

    pub(super) fn conditional(&mut self, draws: &[VarianceComponents]) -> Result<()> {
        let draws: Vec<_> = draws
            .iter()
            .map(|v| Primitive {
                worker: v.worker,
                firm: v.firm,
                covariance: v.covariance,
            })
            .collect();
        self.conditional = numerical_mc::conditional_covariance(&draws)?;
        Ok(())
    }

    pub(super) fn replay(
        mut self,
        problem: &CompressedProblem,
        solver: &PreparedModelSolver<'_>,
        plan: Option<&MatchPlan>,
        rng: CounterRng,
        options: GenericJlaOptions,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<NumericalMcResult> {
        let direct_before = solver.full_cmg_receipt()?;
        let mut receipts = Vec::new();
        reserve_exact(&mut receipts, self.r as usize, "numerical replay receipts")?;
        let mut replay_error = None;
        let mut failed_replay_probe = None;
        let mut evaluations = 0_u64;
        let mut attempted = 0;
        let batch = options.leverage_batch_width.clamp(1, 4);
        // Fixed-width scratch independent of R; one packed word supplies up to
        // four existing Philox lanes. Physical signs feed RHS and pullbacks once.
        let mut worker = zeroed_f64_with_interrupt(
            problem.workers() * batch,
            "replay worker RHS",
            interrupt,
            "numerical_mc_replay_allocate",
        )?;
        let mut firm = zeroed_f64_with_interrupt(
            problem.firms() * batch,
            "replay firm RHS",
            interrupt,
            "numerical_mc_replay_allocate",
        )?;
        let mut pullback = repeated(
            if self.classes.is_empty() {
                0
            } else {
                problem.outcome.len() * batch
            },
            [0.0; 2],
            "replay row pullbacks",
            interrupt,
            "numerical_mc_replay_allocate",
        )?;
        let groups = plan.map_or(0, |p| p.rows.len());
        let mut atoms = repeated(
            groups * batch,
            0_i64,
            "replay match atoms",
            interrupt,
            "numerical_mc_replay_allocate",
        )?;
        if self.status == Status::OkLocal {
            for first in (0..self.r as usize).step_by(batch) {
                interrupt.checkpoint("numerical_mc_replay")?;
                let width = batch.min(self.r as usize - first);
                worker.fill(0.0);
                firm.fill(0.0);
                for class in &self.classes {
                    let mut offset = 0_u64;
                    let mut word_index = u64::MAX;
                    let mut words = [[0_u64; 4]; 2];
                    for &row in &class.rows {
                        let mut count = [0_i64; 4];
                        let mut weighted = [[Sum::default(); 2]; 4];
                        for copy in 0..problem.frequency[row] {
                            checkpoint_chunk(
                                interrupt,
                                copy as usize,
                                "numerical_mc_replay_atoms",
                            )?;
                            let physical = offset + copy;
                            if physical / 64 != word_index {
                                word_index = physical / 64;
                                let first_block = first as u64 / 4;
                                let last_block = (first + width - 1) as u64 / 4;
                                for block in first_block..=last_block {
                                    words[(block - first_block) as usize] = rng.raw_block(
                                        ProbeDomain::Leverage,
                                        block,
                                        class.entity,
                                        word_index,
                                    );
                                }
                                evaluations = evaluations
                                    .checked_add(width as u64)
                                    .ok_or_else(|| resource("replay evaluation overflow"))?;
                            }
                            let [g1, g3] = self.copy[self.offsets[row] + copy as usize];
                            for column in 0..width {
                                let probe = first + column;
                                let word = words[probe / 4 - first / 4][probe % 4];
                                let q = if (word >> (physical % 64)) & 1 == 0 {
                                    -1
                                } else {
                                    1
                                };
                                count[column] += q;
                                weighted[column][0].add(g1 * q as f64);
                                weighted[column][1].add(g3 * q as f64);
                            }
                        }
                        offset += problem.frequency[row];
                        for column in 0..width {
                            worker
                                [column * problem.workers() + problem.row_worker[row] as usize] +=
                                count[column] as f64;
                            firm[column * problem.firms() + problem.row_firm[row] as usize] +=
                                count[column] as f64;
                            pullback[column * problem.outcome.len() + row] =
                                weighted[column].map(Sum::get);
                        }
                    }
                }
                if let Some(plan) = plan {
                    fill_probe_atoms(
                        solver,
                        rng,
                        ProbeDomain::Leverage,
                        first as u64,
                        width,
                        &plan.entity,
                        &plan.physical_count,
                        &mut atoms[..groups * width],
                        interrupt,
                    )?;
                    for &count in &plan.physical_count {
                        evaluations = evaluations
                            .checked_add(count.div_ceil(64) * width as u64)
                            .ok_or_else(|| resource("replay evaluation overflow"))?;
                    }
                    for column in 0..width {
                        for group in 0..groups {
                            let cell = plan.cell[group] as usize;
                            let atom = atoms[column * groups + group] as f64;
                            worker[column * problem.workers()
                                + problem.cell_worker[cell] as usize] += atom;
                            firm[column * problem.firms() + problem.cell_firm[cell] as usize] +=
                                atom;
                        }
                    }
                }
                attempted += width;
                let solved = match solver.solve_batch_with_interrupt(
                    &worker[..problem.workers() * width],
                    &firm[..problem.firms() * width],
                    &[],
                    width,
                    width,
                    interrupt,
                ) {
                    Ok(solved) => solved,
                    Err(error)
                        if matches!(
                            error.code,
                            ErrorCode::PcgMaxIterations
                                | ErrorCode::FullResidualFailed
                                | ErrorCode::PcgCurvatureBreakdown
                                | ErrorCode::PcgPreconditionerBreakdown
                                | ErrorCode::PcgStagnation
                        ) =>
                    {
                        self.status = Status::ReplayFailed;
                        failed_replay_probe = Some(first as u32);
                        replay_error = Some(error);
                        break;
                    }
                    Err(error) => return Err(error),
                };
                for column in 0..width {
                    let probe = first + column;
                    let solution = &solved.solution[column];
                    receipts.push(NumericalReplayRhs {
                        phase: NumericalRhsPhase::LeverageReplay,
                        probe: probe as u32,
                        pcg: solution.receipt.pcg.clone(),
                        complete_residual: solution.receipt.full_residual,
                    });
                    let c = &solution.coefficients;
                    let predict = |row| {
                        solver
                            .operator()
                            .prediction_at(&c.worker, &c.firm, &c.control, row)
                    };
                    let mut scores = [[Sum::default(); 3]; 2];
                    for class in &self.classes {
                        for &row in &class.rows {
                            checkpoint_chunk(interrupt, row, "numerical_mc_replay_copies")?;
                            let p = predict(row)?;
                            let [g2, g4, o] = self.row[row];
                            let [g1q, g3q] = pullback[column * problem.outcome.len() + row];
                            let mut alpha = Sum::default();
                            alpha.add(g2 * p * p);
                            alpha.add(g4 * p.powi(4));
                            alpha.add(-o);
                            alpha.add(g1q * p);
                            alpha.add(g3q * p.powi(3));
                            for (fold, score) in scores.iter_mut().enumerate() {
                                for (j, s) in score.iter_mut().enumerate() {
                                    s.add(
                                        -self.observation_fold[row][fold * 3 + j].get()
                                            * alpha.get(),
                                    );
                                }
                            }
                        }
                    }
                    if let Some(plan) = plan {
                        for group in 0..groups {
                            checkpoint_chunk(interrupt, group, "numerical_mc_replay_blocks")?;
                            let root = (plan.physical_count[group] as f64).sqrt();
                            let p = root * predict(plan.rows[group][0])?;
                            let m = atoms[column * groups + group] as f64 / root - p;
                            let u = [p * p, m * m, p.powi(4), m.powi(4), p * p * m * m];
                            let block = self.blocks[group];
                            let mut xi = Sum::default();
                            xi.add(-block.offset);
                            for (b, u) in block.beta.into_iter().zip(u) {
                                xi.add(b * u);
                            }
                            for (fold, score) in scores.iter_mut().enumerate() {
                                for (j, s) in score.iter_mut().enumerate() {
                                    s.add(-self.block_fold[group][fold * 3 + j].get() * xi.get());
                                }
                            }
                        }
                    }
                    for fold in 0..2 {
                        self.scores[fold][probe] =
                            Primitive::from_values(scores[fold].map(Sum::get));
                    }
                }
            }
        }
        #[cfg(test)]
        if self.status == Status::OkLocal {
            self.cache.validate(
                self.r,
                self.t,
                &self.scores,
                numerical_mc::cross_covariance(&self.scores[0], &self.scores[1])?,
            );
        }
        let mut means = [[0.0; 3]; 2];
        for (fold, mean) in means.iter_mut().enumerate() {
            let mut sums = [Sum::default(); 3];
            for score in &self.scores[fold] {
                for (j, v) in score.values().into_iter().enumerate() {
                    sums[j].add(v / f64::from(self.r));
                }
            }
            *mean = sums.map(Sum::get);
        }
        let mut covariance = if self.status == Status::OkLocal {
            match numerical_mc::cross_covariance(&self.scores[0], &self.scores[1]) {
                Ok(leverage) => numerical_mc::finalize(self.conditional, leverage),
                Err(_) => {
                    self.status = Status::NonfiniteDerivative;
                    numerical_mc::finalize(self.conditional, [[f64::NAN; 3]; 3])
                }
            }
        } else {
            numerical_mc::finalize(self.conditional, [[f64::NAN; 3]; 3])
        };
        if self.status != Status::OkLocal {
            covariance.status = self.status;
        }
        let replay_executed_rhs_count = match (direct_before, solver.full_cmg_receipt()?) {
            (Some(before), Some(after)) => {
                let logical = |receipt: &crate::full_cmg::FullCmgReceipt| {
                    receipt
                        .rhs_count
                        .checked_sub(receipt.model_diagnostics.control_refinement_rhs_count)
                };
                let count = logical(&after)
                    .and_then(|after| logical(&before).and_then(|before| after.checked_sub(before)))
                    .and_then(|count| usize::try_from(count).ok())
                    .ok_or_else(|| {
                        BackendError::invariant(
                            "numerical_mc_replay",
                            "direct replay work count does not reconcile",
                        )
                    })?;
                if count < receipts.len() || count > attempted {
                    return Err(BackendError::invariant(
                        "numerical_mc_replay",
                        "direct replay work differs from its certification inventory",
                    ));
                }
                count
            }
            (None, None) => receipts.len(),
            _ => {
                return Err(BackendError::invariant(
                    "numerical_mc_replay",
                    "direct replay executor identity changed",
                ))
            }
        };
        Ok(NumericalMcResult {
            covariance,
            leverage_probes: self.r,
            target_probes: self.t,
            folds: [self.t.div_ceil(2), self.t / 2],
            replay_rhs: receipts,
            replay_executed_rhs_count,
            replay_attempted_rhs_count: attempted,
            replay_error,
            failed_replay_probe,
            replay_generator_word_evaluations: evaluations,
            replay_score_means: means,
            allocation_bound_bytes: self.bound,
            minimum_constrained: self.minimum_constrained,
            minimum_residual_margin: self.minimum_residual_margin,
            maximum_sensitivity_ratio: self.maximum_sensitivity_ratio,
        })
    }
}
