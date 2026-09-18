// SPDX-License-Identifier: GPL-3.0-only

use super::*;

type Pair = [ComponentSpectrumVector; 2];
type Targets = [Option<Pair>; REPORTED_TARGETS];

pub(super) fn width(requested: usize, capacity: Option<usize>, workers: usize) -> Option<usize> {
    // A scalar pair already fills one or two workers. Interleaving more
    // targets there adds live-vector/cache cost without exposing parallelism.
    if workers <= 2 {
        return None;
    }
    capacity
        .map(|cap| cap.min(requested).min(2 * REPORTED_TARGETS))
        .filter(|&n| n >= 4)
}

// The extra bases, intermediate actions and next iterates coexist with the
// original scalar spectrum lifetimes. Keep this additive, including headers;
// it is admitted before the owned pool and before estimator RNG. Independent
// target packing additionally owns up to eight RHS tuples; a total-target
// worker temporarily owns three primitive tuples. Charge a conservative four
// parameter vectors per lane separately from the original spectrum envelope.
pub(super) fn extra_bytes(rows: u64, parameters: u64, enabled: bool) -> Result<u64> {
    if !enabled {
        return Ok(0);
    }
    checked_sum(&[
        checked_product(
            &[checked_sum(&[rows, parameters])?, 32, 8],
            "cross-target spectrum vectors",
        )?,
        checked_product(&[parameters, 32, 8], "parallel spectrum RHS scratch")?,
        crate::ordered_work::metadata_bytes(2 * REPORTED_TARGETS)?,
        8_192,
    ])
}

fn clone_vector(value: &ComponentSpectrumVector) -> Result<ComponentSpectrumVector> {
    let copy = |input: &[f64]| -> Result<Vec<f64>> {
        let mut result = Vec::new();
        reserve_exact(&mut result, input.len(), "cross-target spectrum copy")?;
        result.extend_from_slice(input);
        Ok(result)
    };
    Ok(ComponentSpectrumVector {
        coefficients: ModelCoefficients {
            worker: copy(&value.coefficients.worker)?,
            firm: copy(&value.coefficients.firm)?,
            control: copy(&value.coefficients.control)?,
        },
        prediction: copy(&value.prediction)?,
    })
}

#[allow(clippy::too_many_arguments)]
fn apply(
    problem: &CompressedProblem,
    inference_rows: ComponentInferenceRows<'_>,
    solver: &PreparedModelSolver<'_>,
    input: &Targets,
    iteration: u32,
    stage: u32,
    iterations: u32,
    batch_width: usize,
    receipts: &mut Vec<ComponentInferenceSolveReceipt>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<Targets> {
    let columns = 2 * input.iter().filter(|pair| pair.is_some()).count();
    let mut output: Targets = core::array::from_fn(|_| None);
    if columns == 0 {
        return Ok(output);
    }
    let controls = inference_rows.controls();
    let zeros = |rows, interrupt: &mut dyn InterruptCheck| {
        zeroed_f64_with_interrupt(
            checked_matrix_length(rows, columns, "cross-target spectrum RHS")?,
            "cross-target spectrum RHS",
            interrupt,
            "generic_jla_component_spectrum_pack",
        )
    };
    let mut worker_rhs = zeros(problem.workers(), interrupt)?;
    let mut firm_rhs = zeros(problem.firms(), interrupt)?;
    let mut control_rhs = zeros(controls, interrupt)?;
    let capacity = solver.owned_batch_capacity()?.unwrap_or(1).min(columns);
    let mut active = Vec::new();
    reserve_exact(&mut active, columns, "spectrum active target lanes")?;
    for (target, pair) in input.iter().enumerate() {
        if let Some(pair) = pair {
            for vector in pair {
                active.push((target, &vector.coefficients));
            }
        }
    }
    let mut packed = repeated(
        columns,
        None,
        "spectrum RHS slots",
        interrupt,
        "generic_jla_component_spectrum_pack",
    )?;
    // A four-lane solver may interleave all eight logical target lanes. Split
    // statistical jobs by its admitted capacity too, not just the solve call.
    for (slots, input) in packed.chunks_mut(capacity).zip(active.chunks(capacity)) {
        solver.statistical_work(
            slots.iter_mut().zip(input.iter().copied()),
            "generic_jla_component_spectrum_rhs",
            |_, (slot, (target, coefficients)), check| {
                *slot = Some(reported_target_rhs(problem, coefficients, target, check)?);
                Ok(())
            },
            interrupt,
        )?;
    }
    for (column, rhs) in packed.into_iter().enumerate() {
        let rhs = rhs.ok_or_else(|| {
            BackendError::invariant("component_inference_spectrum", "missing packed target RHS")
        })?;
        let range = |n| column * n..(column + 1) * n;
        worker_rhs[range(problem.workers())].copy_from_slice(&rhs.0);
        firm_rhs[range(problem.firms())].copy_from_slice(&rhs.1);
        // Fixed-offset match inference uses the FE-only solver; the target
        // action still describes the original controlled model.
        if controls > 0 {
            control_rhs[range(controls)].copy_from_slice(&rhs.2);
        }
    }
    let solved = solver.solve_batch_with_interrupt(
        &worker_rhs,
        &firm_rhs,
        &control_rhs,
        columns,
        batch_width.min(columns),
        interrupt,
    )?;
    if solved.solution.len() != columns {
        return Err(BackendError::invariant(
            "component_inference_spectrum",
            "cross-target solve width",
        ));
    }
    let mut predictions = Vec::new();
    reserve_exact(&mut predictions, columns, "spectrum prediction slots")?;
    for _ in 0..columns {
        predictions.push(zeroed_f64_with_interrupt(
            inference_rows.len(problem),
            "cross-target spectrum prediction",
            interrupt,
            "generic_jla_component_spectrum_unpack",
        )?);
    }
    for (prediction, solutions) in predictions
        .chunks_mut(capacity)
        .zip(solved.solution.chunks(capacity))
    {
        solver.statistical_work(
            prediction.iter_mut().zip(solutions),
            "generic_jla_component_spectrum_predict",
            |_, (output, solution), check| {
                component_predict(
                    problem,
                    inference_rows,
                    solver,
                    &solution.coefficients,
                    output,
                    check,
                )
            },
            interrupt,
        )?;
    }
    let mut solutions = solved.solution.into_iter().zip(predictions);
    for (target, pair) in input.iter().enumerate() {
        if pair.is_none() {
            continue;
        }
        let mut next = Vec::new();
        reserve_exact(&mut next, 2, "cross-target spectrum pair")?;
        for lane in 0..2 {
            let (solution, prediction) = solutions.next().ok_or_else(|| {
                BackendError::invariant(
                    "component_inference_spectrum",
                    "missing cross-target solution",
                )
            })?;
            let index = (target as u32)
                .checked_mul(iterations)
                .and_then(|n| n.checked_add(iteration))
                .and_then(|n| n.checked_mul(4))
                .and_then(|n| n.checked_add(stage * 2 + lane))
                .ok_or_else(|| resource("cross-target spectrum receipt index overflow"))?;
            receipts.push(component_solve_receipt(
                ComponentInferenceSolvePhase::SpectrumIteration,
                index,
                &solution,
            ));
            next.push(ComponentSpectrumVector {
                coefficients: solution.coefficients,
                prediction,
            });
        }
        output[target] = Some(next.try_into().map_err(|_| {
            BackendError::invariant("component_inference_spectrum", "cross-target pair width")
        })?);
    }
    Ok(output)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn iterate(
    problem: &CompressedProblem,
    inference_rows: ComponentInferenceRows<'_>,
    solver: &PreparedModelSolver<'_>,
    start: &Pair,
    iterations: u32,
    batch_width: usize,
    receipts: &mut Vec<ComponentInferenceSolveReceipt>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<[Option<Result<Pair>>; REPORTED_TARGETS]> {
    let mut basis: Targets = core::array::from_fn(|_| None);
    let mut result = core::array::from_fn(|_| None);
    for target in &mut basis {
        *target = Some([clone_vector(&start[0])?, clone_vector(&start[1])?]);
    }
    for iteration in 0..iterations {
        interrupt.checkpoint("generic_jla_component_spectrum_iteration")?;
        let once = apply(
            problem,
            inference_rows,
            solver,
            &basis,
            iteration,
            0,
            iterations,
            batch_width,
            receipts,
            interrupt,
        )?;
        let twice = apply(
            problem,
            inference_rows,
            solver,
            &once,
            iteration,
            1,
            iterations,
            batch_width,
            receipts,
            interrupt,
        )?;
        for (target, pair) in twice.into_iter().enumerate() {
            if let Some(pair) = pair {
                match component_orthonormalize_pair(pair) {
                    Ok(pair) => basis[target] = Some(pair),
                    Err(error)
                        if error.code == ErrorCode::JlaConstraintFailed
                            && error.phase == "component_inference_spectrum" =>
                    {
                        // Preserve target-local withholding. Never continue a
                        // failed target or change its estimator/starting draws.
                        basis[target] = None;
                        result[target] = Some(Err(error));
                    }
                    Err(error) => return Err(error),
                }
            }
        }
    }
    for (target, pair) in basis.into_iter().enumerate() {
        if let Some(pair) = pair {
            result[target] = Some(Ok(pair));
        }
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn spectrum_width_and_memory_are_capacity_bound() {
        for requested in [1, 2, 3, 4, 7, 8, 14, 28, 64, usize::MAX] {
            assert_eq!(width(requested, None, 64), None);
            for capacity in [1, 2, 3, 4, 7, 8, 14, 28, 64] {
                let selected = requested.min(capacity).min(8);
                assert_eq!(width(requested, Some(capacity), 1), None);
                assert_eq!(width(requested, Some(capacity), 2), None);
                assert_eq!(
                    width(requested, Some(capacity), 3),
                    (selected >= 4).then_some(selected)
                );
            }
        }
        assert_eq!(extra_bytes(u64::MAX, u64::MAX, false).unwrap(), 0);
        assert!(extra_bytes(u64::MAX, 1, true).is_err());
        assert!(extra_bytes(u64::MAX / 128, 0, true).is_err());
        assert_eq!(
            extra_bytes(100, 20, true).unwrap(),
            140 * 32 * 8 + 8192 + crate::ordered_work::metadata_bytes(8).unwrap()
        );
    }

    #[test]
    fn small_residual_and_trace_consistency_do_not_certify_dominance() {
        // diag(10, 2, 1): the last two coordinate vectors have exact zero
        // residual and pass the existing final gate, yet miss the leading
        // mode completely. A residual-only early exit would not be safe.
        let diagnostic =
            finish_spectrum_diagnostics(2.0, 1.0, 105.0, 0.0, 1.0, 0.0, 0.0, 128, 2, 0.002)
                .unwrap();
        assert!(diagnostic.certified);
        assert!(diagnostic.leading_eigenvalue < 10.0);
    }
}
