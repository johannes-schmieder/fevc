// SPDX-License-Identifier: GPL-3.0-only

//! Statistical sweeps partition output rows/copies, never floating-point
//! reductions. Each accumulator sees probes in the original logical order.

use super::*;

#[derive(Clone, Copy, Default)]
pub(super) struct ObservationAddress {
    entity: u64,
    copy_offset: u64,
    active: bool,
}

pub(super) fn observation_addresses(
    problem: &CompressedProblem,
    classes: &[ObservationClass],
    interrupt: &mut dyn InterruptCheck,
) -> Result<Vec<ObservationAddress>> {
    let mut addresses = repeated(
        problem.outcome.len(),
        ObservationAddress::default(),
        "observation statistical addresses",
        interrupt,
        "generic_jla_observation_addresses",
    )?;
    for class in classes {
        let mut offset = 0_u64;
        for &row in &class.rows {
            checkpoint_chunk(interrupt, row, "generic_jla_observation_addresses")?;
            if addresses[row].active {
                return Err(BackendError::invariant(
                    "generic_jla_observation_addresses",
                    "duplicate observation address",
                ));
            }
            addresses[row] = ObservationAddress {
                entity: class.entity,
                copy_offset: offset,
                active: true,
            };
            offset = offset
                .checked_add(problem.frequency[row])
                .ok_or_else(|| resource("observation address overflow"))?;
        }
        if offset != class.physical_count {
            return Err(BackendError::invariant(
                "generic_jla_observation_addresses",
                "class frequency mismatch",
            ));
        }
    }
    Ok(addresses)
}

fn chunk_width(length: usize, solver: &PreparedModelSolver<'_>, batch: usize) -> Result<usize> {
    let jobs = solver.owned_batch_workers()?.min(batch).max(1);
    Ok(length.div_ceil(jobs).max(1))
}

pub(super) fn validate_predictions(
    solver: &PreparedModelSolver<'_>,
    solved: &[ModelSolve],
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    for solution in solved {
        let coefficients = &solution.coefficients;
        solver.operator().validate_prediction_coefficients(
            &coefficients.worker,
            &coefficients.firm,
            &coefficients.control,
            interrupt,
        )?;
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
pub(super) fn observation_moments(
    solver: &PreparedModelSolver<'_>,
    addresses: &[ObservationAddress],
    solved: &[ModelSolve],
    square: &mut [f64],
    fourth: &mut [f64],
    square_correction: &mut [f64],
    fourth_correction: &mut [f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    let chunk = chunk_width(addresses.len(), solver, solved.len())?;
    solver.statistical_work(
        square
            .chunks_mut(chunk)
            .zip(fourth.chunks_mut(chunk))
            .zip(square_correction.chunks_mut(chunk))
            .zip(fourth_correction.chunks_mut(chunk)),
        "generic_jla_observation_moments",
        |job, (((square, fourth), square_correction), fourth_correction), interrupt| {
            for local in 0..square.len() {
                let row = job * chunk + local;
                if !addresses[row].active {
                    continue;
                }
                for (column, solution) in solved.iter().enumerate() {
                    checkpoint_chunk(
                        interrupt,
                        local.saturating_mul(solved.len()).saturating_add(column),
                        "generic_jla_observation_moments",
                    )?;
                    let coefficients = &solution.coefficients;
                    let prediction = solver.operator().prediction_at(
                        &coefficients.worker,
                        &coefficients.firm,
                        &coefficients.control,
                        row,
                    )?;
                    stable_add_index(square, square_correction, local, prediction * prediction);
                    stable_add_index(fourth, fourth_correction, local, prediction.powi(4));
                }
            }
            Ok(())
        },
        interrupt,
    )
}

#[allow(clippy::too_many_arguments)]
pub(super) fn observation_correlations(
    solver: &PreparedModelSolver<'_>,
    addresses: &[ObservationAddress],
    row_offset: &[usize],
    solved: &[ModelSolve],
    rng: CounterRng,
    first_probe: usize,
    first: &mut [f64],
    third: &mut [f64],
    first_correction: &mut [f64],
    third_correction: &mut [f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    let chunk = chunk_width(first.len(), solver, solved.len())?;
    solver.statistical_work(
        first
            .chunks_mut(chunk)
            .zip(third.chunks_mut(chunk))
            .zip(first_correction.chunks_mut(chunk))
            .zip(third_correction.chunks_mut(chunk)),
        "generic_jla_observation_correlations",
        |job, (((first, third), first_correction), third_correction), interrupt| {
            let start = job * chunk;
            let mut row = row_offset
                .partition_point(|&offset| offset <= start)
                .saturating_sub(1);
            for local in 0..first.len() {
                let physical = start + local;
                while row_offset[row + 1] <= physical {
                    row += 1;
                }
                let address = addresses[row];
                if !address.active {
                    continue;
                }
                let copy = address.copy_offset + (physical - row_offset[row]) as u64;
                for (column, solution) in solved.iter().enumerate() {
                    checkpoint_chunk(
                        interrupt,
                        local.saturating_mul(solved.len()).saturating_add(column),
                        "generic_jla_observation_correlations",
                    )?;
                    let coefficients = &solution.coefficients;
                    let prediction = solver.operator().prediction_at(
                        &coefficients.worker,
                        &coefficients.firm,
                        &coefficients.control,
                        row,
                    )?;
                    let sign = f64::from(rademacher_copy(
                        rng,
                        ProbeDomain::Leverage,
                        (first_probe + column) as u64,
                        address.entity,
                        copy,
                    ));
                    stable_add_index(first, first_correction, local, sign * prediction);
                    stable_add_index(third, third_correction, local, sign * prediction.powi(3));
                }
            }
            Ok(())
        },
        interrupt,
    )
}

pub(super) fn match_moments(
    solver: &PreparedModelSolver<'_>,
    plan: &MatchPlan,
    solved: &[ModelSolve],
    atoms: &[i64],
    moments: &mut [FiveMoments],
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    let chunk = chunk_width(moments.len(), solver, solved.len())?;
    solver.statistical_work(
        moments.chunks_mut(chunk),
        "generic_jla_match_leverage_moments",
        |job, moments, interrupt| {
            for (local, moment) in moments.iter_mut().enumerate() {
                let group = job * chunk + local;
                let frequency = (plan.physical_count[group] as f64).sqrt();
                for (column, solution) in solved.iter().enumerate() {
                    checkpoint_chunk(
                        interrupt,
                        local.saturating_mul(solved.len()).saturating_add(column),
                        "generic_jla_match_leverage_moments",
                    )?;
                    let coefficients = &solution.coefficients;
                    let prediction = solver.operator().prediction_at(
                        &coefficients.worker,
                        &coefficients.firm,
                        &coefficients.control,
                        plan.rows[group][0],
                    )?;
                    let projection = frequency * prediction;
                    let residual =
                        atoms[column * plan.rows.len() + group] as f64 / frequency - projection;
                    if !projection.is_finite() || !residual.is_finite() {
                        return Err(BackendError::new(
                            ErrorCode::JlaMomentFailed,
                            "generic_jla_match_leverage_moments",
                            "match leverage projection is nonfinite",
                        ));
                    }
                    moment.add(projection, residual);
                }
            }
            Ok(())
        },
        interrupt,
    )
}

/// Contract all three target directions in one row/group traversal. Each
/// compensated sum receives precisely the same terms in the same order as
/// the separate scalar contractions, without three N-length predictions.
#[allow(clippy::too_many_arguments)]
pub(super) fn target_draw(
    problem: &CompressedProblem,
    solver: &PreparedModelSolver<'_>,
    pair: &[ModelSolve],
    working_y: &[f64],
    deleted_adjusted: &[f64],
    deletion: DeletionMode,
    row_order: &[usize],
    match_rows: Option<&[Vec<usize>]>,
    stayer_rows: Option<&[bool]>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<VarianceComponents> {
    let rows = problem.outcome.len();
    if pair.len() != 2
        || working_y.len() != rows
        || deleted_adjusted.len() != rows
        || stayer_rows.is_some_and(|mask| mask.len() != rows)
    {
        return Err(BackendError::invariant(
            "generic_jla_target_contraction",
            "target contraction dimensions disagree",
        ));
    }
    let prediction = |row| -> Result<[f64; 3]> {
        let mut value = [0.0; 3];
        for (side, solution) in pair.iter().enumerate() {
            let c = &solution.coefficients;
            value[side] = solver
                .operator()
                .prediction_at(&c.worker, &c.firm, &c.control, row)?;
        }
        value[2] = value[0] + value[1];
        Ok(value)
    };
    let mut output = [StableAccumulator::default(); 3];
    if deletion == DeletionMode::Match {
        let groups = match_rows.ok_or_else(|| {
            BackendError::invariant(
                "generic_jla_target_contraction",
                "match target contraction has no canonical block rows",
            )
        })?;
        for (group, rows) in groups.iter().enumerate() {
            checkpoint_chunk(interrupt, group, "generic_jla_target_match")?;
            let mut first = [StableAccumulator::default(); 3];
            let mut second = [StableAccumulator::default(); 3];
            for (local, &row) in rows.iter().enumerate() {
                checkpoint_chunk(interrupt, local, "generic_jla_target_match_rows")?;
                let frequency = problem.frequency[row] as f64;
                for (target, projection) in prediction(row)?.into_iter().enumerate() {
                    first[target].add(frequency * working_y[row] * projection);
                    second[target].add(frequency.sqrt() * deleted_adjusted[row] * projection);
                }
            }
            for target in 0..3 {
                output[target].add(first[target].finish() * second[target].finish());
            }
        }
    }
    if deletion == DeletionMode::Observation || stayer_rows.is_some() {
        for (position, &row) in row_order.iter().enumerate() {
            checkpoint_chunk(interrupt, position, "generic_jla_target_observation")?;
            if deletion == DeletionMode::Match && !stayer_rows.expect("stayer mask")[row] {
                continue;
            }
            for (target, projection) in prediction(row)?.into_iter().enumerate() {
                output[target].add(
                    problem.frequency[row] as f64
                        * working_y[row]
                        * deleted_adjusted[row]
                        * projection
                        * projection,
                );
            }
        }
    }
    let [worker, firm, total] = output.map(StableAccumulator::finish);
    if !worker.is_finite() || !firm.is_finite() || !total.is_finite() {
        return Err(nonfinite("target-probe contraction is nonfinite"));
    }
    let result = VarianceComponents {
        worker,
        firm,
        covariance: 0.5 * (total - worker - firm),
        total,
    };
    result.verify_accounting(1e-11)?;
    Ok(result)
}

pub(super) fn target_diagonal(
    solver: &PreparedModelSolver<'_>,
    solved: &[ModelSolve],
    diagonal: &mut [Vec<StableAccumulator>; 3],
    interrupt: &mut dyn InterruptCheck,
) -> Result<()> {
    let [worker, firm, covariance] = diagonal;
    let chunk = chunk_width(worker.len(), solver, solved.len() / 2)?;
    solver.statistical_work(
        worker
            .chunks_mut(chunk)
            .zip(firm.chunks_mut(chunk))
            .zip(covariance.chunks_mut(chunk)),
        "generic_jla_target_diagonal",
        |job, ((worker, firm), covariance), interrupt| {
            for row in 0..worker.len() {
                for (probe, pair) in solved.chunks_exact(2).enumerate() {
                    checkpoint_chunk(
                        interrupt,
                        row.saturating_mul(solved.len()).saturating_add(probe),
                        "generic_jla_target_diagonal",
                    )?;
                    let left = &pair[0].coefficients;
                    let right = &pair[1].coefficients;
                    let w = solver.operator().prediction_at(
                        &left.worker,
                        &left.firm,
                        &left.control,
                        job * chunk + row,
                    )?;
                    let f = solver.operator().prediction_at(
                        &right.worker,
                        &right.firm,
                        &right.control,
                        job * chunk + row,
                    )?;
                    worker[row].add(w * w);
                    firm[row].add(f * f);
                    covariance[row].add(w * f);
                }
            }
            Ok(())
        },
        interrupt,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn packed_statistical_rng_preserves_unaligned_partial_lanes_and_rejects_invalid_shapes() {
        let problem = super::super::direct_tests::fixture(false, false);
        let weight = vec![1.0; problem.outcome.len()];
        let solver = PreparedModelSolver::prepare(
            CanonicalModelData {
                workers: problem.workers(),
                firms: problem.firms(),
                row_worker: &problem.row_worker,
                row_firm: &problem.row_firm,
                weight: &weight,
                controls: &[],
            },
            ModelSolverOptions::default(),
        )
        .unwrap();
        let rng = CounterRng::new(104729);
        let entities = [1, 17, 91];
        let copies = [1, 65, 129];
        for domain in [ProbeDomain::Leverage, ProbeDomain::Target] {
            for first in 0..8 {
                for width in 1..10 {
                    let mut expected = vec![0; width * 3];
                    let mut actual = expected.clone();
                    rng.fill_rademacher_sums(
                        domain,
                        first,
                        width,
                        &entities,
                        &copies,
                        &mut expected,
                    )
                    .unwrap();
                    fill_probe_atoms(
                        &solver,
                        rng,
                        domain,
                        first,
                        width,
                        &entities,
                        &copies,
                        &mut actual,
                        &mut NeverInterrupt,
                    )
                    .unwrap();
                    assert_eq!(expected, actual);
                }
            }
        }
        assert!(fill_probe_atoms(
            &solver,
            rng,
            ProbeDomain::Target,
            0,
            0,
            &entities,
            &copies,
            &mut [],
            &mut NeverInterrupt
        )
        .is_err());
        assert!(fill_probe_atoms(
            &solver,
            rng,
            ProbeDomain::Target,
            u64::MAX,
            2,
            &entities,
            &copies,
            &mut [0; 6],
            &mut NeverInterrupt
        )
        .is_err());
        assert!(fill_probe_atoms(
            &solver,
            rng,
            ProbeDomain::Target,
            0,
            1,
            &entities,
            &[1, 2],
            &mut [0; 3],
            &mut NeverInterrupt
        )
        .is_err());
    }

    #[test]
    fn combined_targets_match_materialized_scalar_bits_with_controls_weights_and_stayers() {
        let mut problem = super::super::direct_tests::controlled_fixture();
        for controls in [2, 0] {
            problem.controls.truncate(controls);
            let weight: Vec<_> = problem.frequency.iter().map(|&n| n as f64).collect();
            let data = CanonicalModelData {
                workers: problem.workers(),
                firms: problem.firms(),
                row_worker: &problem.row_worker,
                row_firm: &problem.row_firm,
                weight: &weight,
                controls: &problem.controls,
            };
            let solver = PreparedModelSolver::prepare(data, ModelSolverOptions::default()).unwrap();
            let zero = solver
                .solve(ModelRhs {
                    worker: &vec![0.0; problem.workers()],
                    firm: &vec![0.0; problem.firms()],
                    control: &vec![0.0; controls],
                })
                .unwrap();
            let mut pair = [zero.clone(), zero];
            for (side, solve) in pair.iter_mut().enumerate() {
                for (index, value) in solve
                    .coefficients
                    .worker
                    .iter_mut()
                    .chain(&mut solve.coefficients.firm)
                    .chain(&mut solve.coefficients.control)
                    .enumerate()
                {
                    *value = ((index * 13 + side * 17) as f64 * 0.21).sin();
                }
            }
            let rows = problem.outcome.len();
            let y: Vec<_> = (0..rows).map(|row| [1e4, -1e4, 1e-4][row % 3]).collect();
            let adjustment: Vec<_> = (0..rows).map(|row| (row as f64 * 0.37).cos()).collect();
            let order: Vec<_> = (0..rows).rev().collect();
            let mut prediction = [vec![0.0; rows], vec![0.0; rows], vec![0.0; rows]];
            for side in 0..2 {
                let c = &pair[side].coefficients;
                solver
                    .operator()
                    .predict_into(&c.worker, &c.firm, &c.control, &mut prediction[side])
                    .unwrap();
            }
            for row in 0..rows {
                prediction[2][row] = prediction[0][row] + prediction[1][row];
            }
            for deletion in [DeletionMode::Observation, DeletionMode::Match] {
                for hybrid in [false, true] {
                    let mask: Vec<_> = (0..rows).map(|row| hybrid && row % 7 == 0).collect();
                    let mover: Vec<_> = order.iter().copied().filter(|&row| !mask[row]).collect();
                    let groups: Vec<_> = mover.chunks(5).map(<[usize]>::to_vec).collect();
                    let mask = hybrid.then_some(mask.as_slice());
                    let candidate = target_draw(
                        &problem,
                        &solver,
                        &pair,
                        &y,
                        &adjustment,
                        deletion,
                        &order,
                        Some(&groups),
                        mask,
                        &mut NeverInterrupt,
                    )
                    .unwrap();
                    let expected = prediction.each_ref().map(|projection| {
                        target_contraction(
                            &problem,
                            &y,
                            &adjustment,
                            projection,
                            deletion,
                            &order,
                            Some(&groups),
                            mask,
                            &mut NeverInterrupt,
                        )
                        .unwrap()
                    });
                    assert_eq!(
                        [candidate.worker, candidate.firm, candidate.total].map(f64::to_bits),
                        expected.map(f64::to_bits)
                    );
                }
            }
        }
    }
}
