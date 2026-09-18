// SPDX-License-Identifier: GPL-3.0-only

//! The deletion-neutral direct FE solve used beneath generic JLA. Statistical
//! probes, physical-copy deletion and target corrections remain in generic_jla.

use super::*;
use crate::full_cmg::{FullCmgDirectSolver, FullCmgPhase, FullCmgPlanOptions, FullCmgReceipt};
use crate::generic_batch::ModelPcgStatus;
use crate::operator::TwoWayOperator;

#[derive(Clone, Debug)]
pub(super) struct SharedDirectSolver<'a> {
    operator: Arc<TwoWayOperator<'a>>,
    solver: Arc<Mutex<FullCmgDirectSolver>>,
}

#[derive(Debug)]
pub(super) struct DirectControlGeometry {
    projection: Vec<ModelCoefficients>,
    inverse: Vec<f64>,
}

impl DirectControlGeometry {
    pub(super) fn from_projections(
        projected: ModelBatchSolve,
        information: &[f64],
        controls: usize,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        let inverse = invert_scaled_spd(
            information,
            controls,
            RANK_NUMERICAL_TOLERANCE_FLOOR,
            interrupt,
            "model_direct_control_schur",
        )?
        .inverse;
        let mut projection = Vec::new();
        reserve_exact(
            &mut projection,
            controls,
            "retained strict FE control projections",
        )?;
        for solution in projected.solution {
            projection.push(solution.coefficients);
        }
        Ok(Self {
            projection,
            inverse,
        })
    }

    fn complete_solution(
        &self,
        coefficients: &mut ModelCoefficients,
        rhs: ModelRhs<'_>,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        let controls = self.projection.len();
        if rhs.control.len() != controls || self.inverse.len() != controls * controls {
            return Err(BackendError::invariant(
                "model_direct_control_schur",
                "control geometry dimensions disagree",
            ));
        }
        let mut reduced = copy_f64_with_interrupt(
            rhs.control,
            "direct control RHS",
            interrupt,
            "model_direct_control_rhs",
        )?;
        // With U = H_FE^-1 A'WX, beta_FE = H_FE^-1 b_FE - U gamma
        // and gamma = S^-1 (b_X - U' b_FE). These contractions use strict
        // cached projections, not an outcome-dependent or approximate rank.
        for (control, projection) in self.projection.iter().enumerate() {
            let mut value = reduced[control];
            let mut correction = 0.0;
            for (index, (left, right)) in projection
                .worker
                .iter()
                .chain(&projection.firm)
                .zip(rhs.worker.iter().chain(rhs.firm))
                .enumerate()
            {
                checkpoint_chunk(interrupt, index, "model_direct_control_rhs")?;
                stable_add_index_value(&mut value, &mut correction, -left * right);
            }
            reduced[control] = value + correction;
        }
        let mut control_solution = zeroed_f64_with_interrupt(
            controls,
            "direct control coefficients",
            interrupt,
            "model_direct_control_solve",
        )?;
        for (row, value) in control_solution.iter_mut().enumerate() {
            let mut correction = 0.0;
            for (column, &rhs) in reduced.iter().enumerate() {
                stable_add_index_value(
                    value,
                    &mut correction,
                    self.inverse[row * controls + column] * rhs,
                );
            }
            *value += correction;
        }
        for (index, value) in coefficients
            .worker
            .iter_mut()
            .chain(&mut coefficients.firm)
            .enumerate()
        {
            checkpoint_chunk(interrupt, index, "model_direct_control_recover")?;
            let mut correction = 0.0;
            for (projection, &gamma) in self.projection.iter().zip(&control_solution) {
                let projected = if index < projection.worker.len() {
                    projection.worker[index]
                } else {
                    projection.firm[index - projection.worker.len()]
                };
                stable_add_index_value(value, &mut correction, -projected * gamma);
            }
            *value += correction;
        }
        coefficients.control = control_solution;
        Ok(())
    }
}

fn stable_add_index_value(sum: &mut f64, correction: &mut f64, value: f64) {
    let next = *sum + value;
    *correction += if sum.abs() >= value.abs() {
        (*sum - next) + value
    } else {
        (value - next) + *sum
    };
    *sum = next;
}

impl<'a> PreparedModelSolver<'a> {
    pub(crate) fn prepare_generic_jla_direct(
        problem: &'a CompressedProblem,
        data: CanonicalModelData<'a>,
        routing: ModelRoutingOptions,
        plan: FullCmgPlanOptions,
        memory_limit: u64,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<PreparedGenericJlaSolvers<'a>> {
        if !data.controls.is_empty() || !problem.controls.is_empty() {
            return Err(BackendError::new(
                ErrorCode::UnsupportedFeature,
                "generic_full_cmg",
                "direct generic CMG requires no controls",
            ));
        }
        let fe = Self::prepare_generic_jla_direct_fe(
            problem,
            data,
            routing,
            plan,
            memory_limit,
            interrupt,
        )?;
        let mut full = Self::prepare_routed_internal(
            data,
            ModelRoutingOptions {
                route: ModelSolverRoute::Diagonal,
                ..routing
            },
            true,
            interrupt,
        )?;
        full.direct = fe.direct.clone();
        full.receipt = fe.receipt.clone();
        Ok(PreparedGenericJlaSolvers {
            full,
            fe,
            fe_hierarchy_reused: true,
        })
    }

    /// Build the shared hierarchy without solve workspaces. The caller must
    /// select widths and admit the complete control-preparation/command peak
    /// before allocating pools or running strict control projections.
    pub(crate) fn prepare_generic_jla_direct_fe(
        problem: &'a CompressedProblem,
        data: CanonicalModelData<'a>,
        routing: ModelRoutingOptions,
        plan: FullCmgPlanOptions,
        memory_limit: u64,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        if routing.route == ModelSolverRoute::Diagonal {
            return Err(BackendError::invalid(
                "generic_full_cmg",
                "explicit diagonal cannot select direct CMG",
            ));
        }
        let mut fe = Self::prepare_with_interrupt(
            CanonicalModelData {
                controls: &[],
                ..data
            },
            routing.solver,
            interrupt,
        )?;
        let direct = SharedDirectSolver {
            operator: Arc::new(TwoWayOperator::fe_part_with_interrupt(problem, interrupt)?),
            solver: Arc::new(Mutex::new(FullCmgDirectSolver::prepare_with_pool_policy(
                problem,
                routing.solver.pcg,
                memory_limit,
                plan,
                interrupt,
                true,
            )?)),
        };
        fe.receipt.requested = routing.route;
        fe.receipt.selected = ModelSolverRoute::Cmg;
        fe.receipt.cmg = Some(direct.lock()?.compatibility_receipt().clone());
        fe.direct = Some(direct);
        Ok(fe)
    }

    pub(crate) fn prepare_generic_jla_direct_controlled(
        data: CanonicalModelData<'a>,
        routing: ModelRoutingOptions,
        fe: &Self,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        let operator = ModelOperator::new_with_interrupt(data, interrupt)?;
        let control_schur =
            certify_control_rank(&operator, routing.solver, true, Some(fe), interrupt)?;
        let diagonal = ModelDiagonalPreconditioner::new_with_interrupt(
            operator.reduced_diagonal(),
            interrupt,
        )?;
        let mut receipt = fe.receipt.clone();
        receipt.dimension = operator.parameter_count();
        Ok(Self {
            operator,
            backend: PreparedModelBackend::Diagonal(diagonal),
            options: routing.solver,
            control_schur,
            receipt,
            direct: fe.direct.clone(),
            diagonal_queue: None,
        })
    }

    pub(crate) fn full_cmg_receipt(&self) -> Result<Option<FullCmgReceipt>> {
        self.direct
            .as_ref()
            .map(|d| d.lock()?.receipt())
            .transpose()
    }

    pub(crate) fn direct_control_retained_bytes(&self) -> Result<u64> {
        let Some(geometry) = &self.control_schur.direct_geometry else {
            return Ok(0);
        };
        let mut bytes = geometry
            .projection
            .capacity()
            .checked_mul(std::mem::size_of::<ModelCoefficients>())
            .ok_or_else(|| rank_resource("control retained headers overflow"))?;
        for values in geometry
            .projection
            .iter()
            .flat_map(|p| [&p.worker, &p.firm, &p.control])
            .chain(std::iter::once(&geometry.inverse))
        {
            bytes = values
                .capacity()
                .checked_mul(std::mem::size_of::<f64>())
                .and_then(|v| bytes.checked_add(v))
                .ok_or_else(|| rank_resource("control retained capacities overflow"))?;
        }
        u64::try_from(bytes).map_err(|_| rank_resource("control retained bytes not representable"))
    }

    pub(crate) fn configure_full_cmg_capacity(&self, columns: usize) -> Result<()> {
        if let Some(direct) = &self.direct {
            direct.lock()?.configure_capacity(columns)?;
        }
        Ok(())
    }

    pub(crate) fn forecast_full_cmg_capacity(
        &self,
        columns: usize,
    ) -> Result<crate::full_cmg::FullCmgSetupReceipt> {
        self.direct
            .as_ref()
            .ok_or_else(|| BackendError::invariant("cmg_capacity", "missing direct solver"))?
            .lock()?
            .forecast_capacity(columns)
    }

    pub(crate) fn allocate_full_cmg_pools(&mut self) -> Result<()> {
        if let Some(direct) = &self.direct {
            let mut solver = direct.lock()?;
            solver.allocate_deferred_pools()?;
            self.receipt.cmg = Some(solver.compatibility_receipt().clone());
        }
        Ok(())
    }

    pub(crate) fn reconcile_full_cmg_memory(&self, peak: u64) -> Result<()> {
        if let Some(direct) = &self.direct {
            direct.lock()?.reconcile_memory(peak)?;
        }
        Ok(())
    }

    pub(crate) fn refresh_full_cmg_receipt(&mut self) -> Result<()> {
        if let Some(direct) = &self.direct {
            self.receipt.cmg = Some(direct.lock()?.compatibility_receipt().clone());
        }
        Ok(())
    }
}

impl SharedDirectSolver<'_> {
    fn lock(&self) -> Result<std::sync::MutexGuard<'_, FullCmgDirectSolver>> {
        self.solver.lock().map_err(|_| {
            BackendError::new(
                ErrorCode::ContextPoisoned,
                "generic_full_cmg",
                "shared direct solver lock poisoned",
            )
        })
    }

    pub(super) fn solve(
        &self,
        model: &PreparedModelSolver<'_>,
        rhs: ModelRhs<'_>,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<ModelSolve> {
        let batch = self.solve_phase(
            model,
            rhs.worker,
            rhs.firm,
            rhs.control,
            1,
            1,
            FullCmgPhase::Fit,
            None,
            interrupt,
        )?;
        batch.solution.into_iter().next().ok_or_else(|| {
            BackendError::invariant("generic_full_cmg", "fit solve returned no column")
        })
    }

    pub(super) fn statistical_work<I, Iter, F>(
        &self,
        input: Iter,
        phase: &'static str,
        operation: F,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()>
    where
        I: Send,
        Iter: ExactSizeIterator<Item = I>,
        F: Fn(usize, I, &mut dyn InterruptCheck) -> Result<()> + Sync,
    {
        let solver = self.lock()?;
        crate::ordered_work::run(
            Some(crate::ordered_work::Pool::Cmg(&solver)),
            input.len(),
            input,
            phase,
            operation,
            interrupt,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn solve_batch(
        &self,
        model: &PreparedModelSolver<'_>,
        worker: &[f64],
        firm: &[f64],
        control: &[f64],
        columns: usize,
        width: usize,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<ModelBatchSolve> {
        self.solve_phase(
            model,
            worker,
            firm,
            control,
            columns,
            width,
            FullCmgPhase::Probe,
            None,
            interrupt,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn solve_batch_with_options(
        &self,
        model: &PreparedModelSolver<'_>,
        worker: &[f64],
        firm: &[f64],
        control: &[f64],
        columns: usize,
        width: usize,
        options: ModelSolverOptions,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<ModelBatchSolve> {
        options.validate()?;
        self.solve_phase(
            model,
            worker,
            firm,
            control,
            columns,
            width,
            FullCmgPhase::Fit,
            Some(options),
            interrupt,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn solve_phase(
        &self,
        model: &PreparedModelSolver<'_>,
        worker: &[f64],
        firm: &[f64],
        control: &[f64],
        columns: usize,
        width: usize,
        phase: FullCmgPhase,
        override_options: Option<ModelSolverOptions>,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<ModelBatchSolve> {
        let _profile =
            crate::pipeline_profile::Scope::new(crate::pipeline_profile::Phase::ModelSolve);
        validate_batch_rhs(
            &model.operator,
            worker,
            firm,
            control,
            columns,
            width,
            interrupt,
        )?;
        let solver = self.lock()?;
        let setup = solver.receipt()?.setup;
        let (pcg_options, gate) = override_options.map_or_else(
            || solver.phase_options(phase),
            |options| (options.pcg, options.full_residual_tolerance()),
        );
        let width = width.min(setup.maximum_batch_rhs);
        let mut solution = Vec::new();
        reserve_exact(&mut solution, columns, "generic direct results")?;
        let workers = model.operator.workers();
        let firms = model.operator.firms();
        for first in (0..columns).step_by(width) {
            let count = width.min(columns - first);
            let batch = solver.solve_batch_with_options_and_interrupt(
                &self.operator,
                &worker[first * workers..(first + count) * workers],
                &firm[first * firms..(first + count) * firms],
                count,
                pcg_options,
                gate,
                interrupt,
            )?;
            if batch.solution.len() != count || batch.pcg.len() != count {
                return Err(BackendError::invariant(
                    "generic_full_cmg",
                    "direct column count mismatch",
                ));
            }
            let mut certified = Vec::new();
            reserve_exact(&mut certified, count, "generic certified result slots")?;
            certified.resize_with(count, || None);
            let certification_profile = crate::pipeline_profile::Scope::new(
                crate::pipeline_profile::Phase::GenericCertification,
            );
            crate::ordered_work::run(
                Some(crate::ordered_work::Pool::Cmg(&solver)),
                count,
                batch
                    .solution
                    .into_iter()
                    .map(|solved| (solved.worker, solved.firm))
                    .zip(batch.pcg)
                    .zip(&mut certified),
                "generic_full_cmg_certification",
                |local, (((solved_worker, solved_firm), pcg), slot), interrupt| {
                    let column = first + local;
                    let rhs = ModelRhs {
                        worker: rhs_column(worker, column, workers),
                        firm: rhs_column(firm, column, firms),
                        control: rhs_column(control, column, model.operator.controls()),
                    };
                    // Keep scientific failures in their logical slots so an
                    // earlier column's refinement/error still takes precedence.
                    *slot = Some((|| {
                        let mut coefficients = ModelCoefficients {
                            worker: solved_worker,
                            firm: solved_firm,
                            control: Vec::new(),
                        };
                        if let Some(geometry) = &model.control_schur.direct_geometry {
                            geometry.complete_solution(&mut coefficients, rhs, interrupt)?;
                        }
                        // Certify through the generic original operator as well as the
                        // direct solver's independent complete-system/refinement gate.
                        let residual = model.operator.full_residual_with_interrupt(
                            &coefficients.worker,
                            &coefficients.firm,
                            &coefficients.control,
                            rhs,
                            interrupt,
                        )?;
                        let pcg = ModelPcgReceipt {
                            status: if pcg.zero_rhs && rhs.control.iter().all(|&v| v == 0.0) {
                                ModelPcgStatus::ZeroRhs
                            } else {
                                ModelPcgStatus::Converged
                            },
                            iterations: pcg.iterations,
                            relative_residual: pcg.relative_residual,
                            residual_replacements: pcg.residual_replacements,
                            operator_applications: pcg.operator_applications,
                            preconditioner_applications: pcg.preconditioner_applications,
                        };
                        Ok(ModelSolve {
                            coefficients,
                            receipt: ModelSolveReceipt {
                                pcg,
                                full_residual_tolerance: gate,
                                full_residual: residual.relative_norm,
                            },
                            residual,
                        })
                    })());
                    Ok(())
                },
                interrupt,
            )?;
            drop(certification_profile);
            for (local, certified) in certified.into_iter().enumerate() {
                let column = first + local;
                let rhs = ModelRhs {
                    worker: rhs_column(worker, column, workers),
                    firm: rhs_column(firm, column, firms),
                    control: rhs_column(control, column, model.operator.controls()),
                };
                let mut certified = certified.ok_or_else(|| {
                    BackendError::invariant("generic_full_cmg", "missing certified column")
                })??;
                let ModelSolve {
                    coefficients,
                    residual,
                    receipt,
                } = &mut certified;
                if model.control_schur.direct_geometry.is_some() && residual.relative_norm > gate {
                    self.refine_controlled(
                        model,
                        &solver,
                        coefficients,
                        residual,
                        &mut receipt.pcg,
                        rhs,
                        pcg_options,
                        gate,
                        interrupt,
                    )?;
                }
                if residual.relative_norm > gate {
                    return Err(BackendError::new(
                        ErrorCode::FullResidualFailed,
                        "generic_full_cmg",
                        format!(
                            "column {column}: original residual {} exceeds {gate}",
                            residual.relative_norm
                        ),
                    ));
                }
                receipt.full_residual = residual.relative_norm;
                solution.push(certified);
            }
        }
        solver.record_model_work(
            if override_options.is_some() {
                columns
            } else {
                0
            },
            if model.control_schur.direct_geometry.is_some() {
                columns
            } else {
                0
            },
            0,
        )?;
        Ok(ModelBatchSolve { columns, solution })
    }

    /// Fixed full-system iterative correction. Only a failing logical column
    /// enters this ladder; every correction uses the same cached Schur block,
    /// hierarchy and owned scalar queue, never a fallback or new estimator.
    #[allow(clippy::too_many_arguments)]
    fn refine_controlled(
        &self,
        model: &PreparedModelSolver<'_>,
        solver: &FullCmgDirectSolver,
        coefficients: &mut ModelCoefficients,
        residual: &mut ModelResidual,
        receipt: &mut ModelPcgReceipt,
        rhs: ModelRhs<'_>,
        pcg: PcgOptions,
        gate: f64,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        let geometry = model
            .control_schur
            .direct_geometry
            .as_ref()
            .ok_or_else(|| {
                BackendError::invariant("model_direct_refine", "missing control Schur geometry")
            })?;
        for factor in [0.1, 0.01, 0.001] {
            if residual.relative_norm <= gate {
                break;
            }
            interrupt.checkpoint("model_direct_control_refinement")?;
            let (correction_worker, correction_firm) =
                correction_quotient_rhs(residual, interrupt)?;
            let correction_rhs = ModelRhs {
                worker: &correction_worker,
                firm: &correction_firm,
                control: &residual.control,
            };
            let corrected = solver.solve_batch_with_options_and_interrupt(
                &self.operator,
                correction_rhs.worker,
                correction_rhs.firm,
                1,
                PcgOptions {
                    tolerance: pcg.tolerance * factor,
                    ..pcg
                },
                gate,
                interrupt,
            )?;
            solver.record_model_work(0, 0, 1)?;
            let solved = corrected.solution.into_iter().next().ok_or_else(|| {
                BackendError::invariant("model_direct_refine", "missing correction solution")
            })?;
            let mut delta = ModelCoefficients {
                worker: solved.worker,
                firm: solved.firm,
                control: Vec::new(),
            };
            geometry.complete_solution(&mut delta, correction_rhs, interrupt)?;
            for (index, (value, change)) in coefficients
                .worker
                .iter_mut()
                .chain(&mut coefficients.firm)
                .chain(&mut coefficients.control)
                .zip(delta.worker.iter().chain(&delta.firm).chain(&delta.control))
                .enumerate()
            {
                checkpoint_chunk(interrupt, index, "model_direct_control_refinement_update")?;
                *value += change;
            }
            let pcg = &corrected.pcg[0];
            receipt.iterations = receipt
                .iterations
                .checked_add(pcg.iterations)
                .ok_or_else(|| rank_resource("direct refinement iteration count overflow"))?;
            receipt.residual_replacements = receipt
                .residual_replacements
                .checked_add(pcg.residual_replacements)
                .ok_or_else(|| rank_resource("direct refinement replacement count overflow"))?;
            receipt.operator_applications = receipt
                .operator_applications
                .checked_add(pcg.operator_applications)
                .ok_or_else(|| rank_resource("direct refinement action count overflow"))?;
            receipt.preconditioner_applications = receipt
                .preconditioner_applications
                .checked_add(pcg.preconditioner_applications)
                .ok_or_else(|| rank_resource("direct refinement preconditioner count overflow"))?;
            receipt.relative_residual = pcg.relative_residual;
            *residual = model.operator.full_residual_with_interrupt(
                &coefficients.worker,
                &coefficients.firm,
                &coefficients.control,
                rhs,
                interrupt,
            )?;
        }
        Ok(())
    }
}

fn correction_quotient_rhs(
    residual: &ModelResidual,
    interrupt: &mut dyn InterruptCheck,
) -> Result<(Vec<f64>, Vec<f64>)> {
    // Roundoff in recomputed score residuals can leave a tiny null-coordinate
    // component even for an exactly compatible original RHS. Project only the
    // *correction* onto the FE quotient. Its direct gate is unchanged, and the
    // accumulated answer is still certified against every original equation.
    let mut worker = copy_f64_with_interrupt(
        &residual.worker,
        "correction worker quotient",
        interrupt,
        "model_direct_refinement_quotient",
    )?;
    let mut firm = copy_f64_with_interrupt(
        &residual.firm,
        "correction firm quotient",
        interrupt,
        "model_direct_refinement_quotient",
    )?;
    let dimension = worker
        .len()
        .checked_add(firm.len())
        .ok_or_else(|| rank_resource("correction quotient dimension overflow"))?;
    let mut sum = 0.0;
    let mut correction = 0.0;
    for (index, value) in worker
        .iter()
        .copied()
        .chain(firm.iter().map(|&v| -v))
        .enumerate()
    {
        checkpoint_chunk(interrupt, index, "model_direct_refinement_quotient")?;
        stable_add_index_value(&mut sum, &mut correction, value);
    }
    let shift = (sum + correction) / dimension as f64;
    for (index, value) in worker.iter_mut().enumerate() {
        checkpoint_chunk(interrupt, index, "model_direct_refinement_quotient")?;
        *value -= shift;
    }
    for (index, value) in firm.iter_mut().enumerate() {
        checkpoint_chunk(interrupt, index, "model_direct_refinement_quotient")?;
        *value += shift;
    }
    Ok((worker, firm))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryBudget;
    use crate::problem::CanonicalInput;
    use crate::types::InputColumns;

    fn problem() -> CompressedProblem {
        let mut input = InputColumns {
            worker: vec![],
            firm: vec![],
            deletion: vec![],
            outcome: vec![],
            frequency: vec![],
            target_weight: vec![],
            controls: vec![vec![], vec![]],
        };
        for worker in 0..8 {
            for firm in 0..4 {
                for copy in 0..3 {
                    let row = input.worker.len();
                    input.worker.push(worker + 1);
                    input.firm.push(firm + 1);
                    input.deletion.push(worker * 4 + firm + 1);
                    input.outcome.push((row as f64 * 0.31).cos());
                    input.frequency.push(1 + copy);
                    input.target_weight.push(1.0);
                    input.controls[0].push((row as f64 * 0.73).sin());
                    input.controls[1].push((row as f64 * 1.29).cos());
                }
            }
        }
        let n = input.worker.len();
        CanonicalInput::from_validated(input.validate().unwrap())
            .unwrap()
            .compress(&vec![true; n])
            .unwrap()
    }

    // Independent test-only grounded normal matrix and pivoted elimination.
    // Neither production operator, Schur algebra nor dense inverse is used.
    fn dense_oracle(problem: &CompressedProblem, rhs: ModelRhs<'_>) -> Vec<f64> {
        let w = problem.workers();
        let f = problem.firms();
        let q = problem.controls.len();
        let n = w + f - 1 + q;
        let mut a = vec![vec![0.0; n + 1]; n];
        for row in 0..problem.outcome.len() {
            let mut x = vec![0.0; n];
            x[problem.row_worker[row] as usize] = 1.0;
            let firm = problem.row_firm[row] as usize;
            if firm + 1 < f {
                x[w + firm] = 1.0;
            }
            for control in 0..q {
                x[w + f - 1 + control] = problem.controls[control][row];
            }
            for i in 0..n {
                for j in 0..n {
                    a[i][j] += problem.frequency[row] as f64 * x[i] * x[j];
                }
            }
        }
        for (i, value) in rhs
            .worker
            .iter()
            .chain(&rhs.firm[..f - 1])
            .chain(rhs.control)
            .enumerate()
        {
            a[i][n] = *value;
        }
        for col in 0..n {
            let pivot = (col..n)
                .max_by(|&i, &j| a[i][col].abs().total_cmp(&a[j][col].abs()))
                .unwrap();
            a.swap(col, pivot);
            assert!(a[col][col].abs() > 1e-10);
            let diagonal = a[col][col];
            for j in col..=n {
                a[col][j] /= diagonal;
            }
            for i in 0..n {
                if i != col {
                    let factor = a[i][col];
                    for j in col..=n {
                        a[i][j] -= factor * a[col][j];
                    }
                }
            }
        }
        a.iter().map(|row| row[n]).collect()
    }

    fn compare_oracle(solution: &ModelSolve, expected: &[f64]) {
        let c = &solution.coefficients;
        let last = c.firm[c.firm.len() - 1];
        let actual = c
            .worker
            .iter()
            .map(|&v| v + last)
            .chain(c.firm[..c.firm.len() - 1].iter().map(|&v| v - last))
            .chain(c.control.iter().copied());
        for (actual, &expected) in actual.zip(expected) {
            assert!(
                (actual - expected).abs() <= 1e-9 * expected.abs().max(1.0),
                "{actual} != {expected}"
            );
        }
        assert!(solution.residual.relative_norm <= solution.receipt.full_residual_tolerance);
    }

    #[test]
    fn controlled_direct_dense_oracle_zero_control_only_partial_batches_and_reuse() {
        let problem = problem();
        assert_eq!(
            TwoWayOperator::new(&problem).unwrap_err().code,
            ErrorCode::UnsupportedFeature
        );
        let weights: Vec<_> = problem.frequency.iter().map(|&v| v as f64).collect();
        let data = CanonicalModelData {
            workers: problem.workers(),
            firms: problem.firms(),
            row_worker: &problem.row_worker,
            row_firm: &problem.row_firm,
            weight: &weights,
            controls: &problem.controls,
        };
        let mut worker = vec![0.0; 5 * data.workers];
        let mut firm = vec![0.0; 5 * data.firms];
        let mut control = vec![0.0; 5 * data.controls.len()];
        control[2] = 1.0; // Column one has only a control RHS, not a zero model RHS.
        for col in 2..5 {
            for row in 0..weights.len() {
                let value = weights[row] * ((row + col * 11) as f64 * 0.17).cos();
                worker[col * data.workers + data.row_worker[row] as usize] += value;
                firm[col * data.firms + data.row_firm[row] as usize] += value;
                for j in 0..2 {
                    control[col * 2 + j] += value * data.controls[j][row];
                }
            }
        }
        let expected: Vec<_> = (0..5)
            .map(|col| {
                dense_oracle(
                    &problem,
                    ModelRhs {
                        worker: rhs_column(&worker, col, data.workers),
                        firm: rhs_column(&firm, col, data.firms),
                        control: rhs_column(&control, col, 2),
                    },
                )
            })
            .collect();
        for threads in [1, 2, 3, 4, 7, 14, 28, 64] {
            let mut plan = FullCmgPlanOptions::production(threads, 1e-10, Some(1e-10));
            plan.memory_budget = MemoryBudget::Unspecified;
            plan.maximum_batch_rhs = 3;
            let routing = ModelRoutingOptions {
                route: ModelSolverRoute::Cmg,
                ..Default::default()
            };
            let mut fe = PreparedModelSolver::prepare_generic_jla_direct_fe(
                &problem,
                data,
                routing,
                plan,
                0,
                &mut NeverInterrupt,
            )
            .unwrap();
            fe.configure_full_cmg_capacity(3).unwrap();
            fe.allocate_full_cmg_pools().unwrap();
            let model = PreparedModelSolver::prepare_generic_jla_direct_controlled(
                data,
                routing,
                &fe,
                &mut NeverInterrupt,
            )
            .unwrap();
            assert_eq!(
                model.solve_batch(&[], &[], &[], 0, 1).unwrap_err().code,
                ErrorCode::InvalidInput
            );
            for width in [1, 3, 5] {
                let solved = model
                    .solve_batch(&worker, &firm, &control, 5, width)
                    .unwrap();
                assert_eq!(
                    solved.solution[0].receipt.pcg.status,
                    ModelPcgStatus::ZeroRhs
                );
                assert_eq!(
                    solved.solution[1].receipt.pcg.status,
                    ModelPcgStatus::Converged
                );
                for (solution, expected) in solved.solution.iter().zip(&expected) {
                    compare_oracle(solution, expected);
                }
            }
            let mut malformed = control.clone();
            malformed[4] = f64::NAN;
            assert!(model.solve_batch(&worker, &firm, &malformed, 5, 3).is_err());
            assert!(model.solve_batch(&worker, &firm, &control, 5, 3).is_ok());
            let diagnostics = model.full_cmg_receipt().unwrap().unwrap().model_diagnostics;
            assert_eq!(diagnostics.explicit_options_rhs_count, 2);
            assert_eq!(diagnostics.controlled_rhs_count, 20);
        }
    }

    #[test]
    fn controlled_full_residual_refines_fails_closed_and_recovers() {
        struct BreakRefinement;
        impl InterruptCheck for BreakRefinement {
            fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
                if phase == "model_direct_control_refinement" {
                    Err(BackendError::new(ErrorCode::UserBreak, phase, "test break"))
                } else {
                    Ok(())
                }
            }
        }
        let problem = problem();
        let weights: Vec<_> = problem.frequency.iter().map(|&v| v as f64).collect();
        let data = CanonicalModelData {
            workers: problem.workers(),
            firms: problem.firms(),
            row_worker: &problem.row_worker,
            row_firm: &problem.row_firm,
            weight: &weights,
            controls: &problem.controls,
        };
        let mut plan = FullCmgPlanOptions::production(4, 1e-10, Some(1e-10));
        plan.memory_budget = MemoryBudget::Unspecified;
        let routing = ModelRoutingOptions {
            route: ModelSolverRoute::Cmg,
            ..Default::default()
        };
        let mut fe = PreparedModelSolver::prepare_generic_jla_direct_fe(
            &problem,
            data,
            routing,
            plan,
            0,
            &mut NeverInterrupt,
        )
        .unwrap();
        fe.allocate_full_cmg_pools().unwrap();
        let mut model = PreparedModelSolver::prepare_generic_jla_direct_controlled(
            data,
            routing,
            &fe,
            &mut NeverInterrupt,
        )
        .unwrap();
        let worker = vec![0.0; data.workers];
        let firm = vec![0.0; data.firms];
        let rhs = ModelRhs {
            worker: &worker,
            firm: &firm,
            control: &[1.0, -0.5],
        };
        let expected = dense_oracle(&problem, rhs);
        let original = model
            .control_schur
            .direct_geometry
            .as_ref()
            .unwrap()
            .inverse
            .clone();
        // Test-only perturbation forces the complete W+F+Q gate and ladder;
        // ordinary preparation never modifies a certified inverse this way.
        for value in &mut model
            .control_schur
            .direct_geometry
            .as_mut()
            .unwrap()
            .inverse
        {
            *value *= 1.001;
        }
        assert_eq!(
            model
                .solve_with_interrupt(rhs, &mut BreakRefinement)
                .unwrap_err()
                .code,
            ErrorCode::UserBreak
        );
        compare_oracle(&model.solve(rhs).unwrap(), &expected);
        let receipt = model.full_cmg_receipt().unwrap().unwrap();
        assert!(receipt.model_diagnostics.control_refinement_rhs_count >= 2);
        for (value, &exact) in model
            .control_schur
            .direct_geometry
            .as_mut()
            .unwrap()
            .inverse
            .iter_mut()
            .zip(&original)
        {
            *value = exact * 2.0;
        }
        assert_eq!(
            model.solve(rhs).unwrap_err().code,
            ErrorCode::FullResidualFailed
        );
        model
            .control_schur
            .direct_geometry
            .as_mut()
            .unwrap()
            .inverse = original;
        compare_oracle(&model.solve(rhs).unwrap(), &expected);
    }
}
