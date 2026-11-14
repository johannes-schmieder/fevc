// SPDX-License-Identifier: MIT

//! Leading direction of one reported target on the unit geometry.
//!
//! For a target `A_t` the direction is the eigenvector of `A_t v = lambda S v`
//! of largest absolute eigenvalue, where `S` is the normal matrix of the
//! worker-firm design. A thick-restart Lanczos iteration in the `S` inner
//! product finds both ends of the spectrum at once and needs one linear solve
//! per step. Power iteration is far slower exactly where the direction is not
//! dominant, because the leading eigenvalues are then close, and for the
//! covariance target they come with both signs.
//!
//! Nothing here depends on the outcome: the start vector is a fixed function
//! of the coefficient index or the direction of another target.
use super::*;

/// Vectors kept before a restart, and Ritz vectors carried across it.
const BASIS: usize = 6;
const KEPT: usize = 3;

pub(super) struct LeadingDirection {
    /// Rayleigh quotient of the reported direction.
    pub eigenvalue: f64,
    /// Second Ritz value by magnitude: a lower bound in magnitude for the
    /// second eigenvalue, zero when the basis held one vector.
    pub second_eigenvalue: f64,
    /// Residual of the reported pair in the `S` norm over `|eigenvalue|`.
    pub residual: f64,
    /// Linear solves used.
    pub steps: u32,
    pub converged: bool,
    /// Unit-space weights `X v` with unit Euclidean norm.
    pub mode: Vec<f64>,
    pub coefficients: ModelCoefficients,
}

type Pair = (Vec<f64>, Vec<f64>);

fn inner(left: &Pair, right: &Pair) -> f64 {
    let mut value = StableAccumulator::default();
    for (a, b) in left.0.iter().zip(&right.0) {
        value.add(a * b);
    }
    for (a, b) in left.1.iter().zip(&right.1) {
        value.add(a * b);
    }
    value.finish()
}

fn axpy(target: &mut Pair, scale: f64, source: &Pair) {
    for (a, b) in target.0.iter_mut().zip(&source.0) {
        *a += scale * b;
    }
    for (a, b) in target.1.iter_mut().zip(&source.1) {
        *a += scale * b;
    }
}

/// Eigenvalues and eigenvectors of a small symmetric matrix by cyclic Jacobi
/// rotations. `matrix` is row-major of order `n`; column `k` of the returned
/// row-major matrix belongs to eigenvalue `k`.
pub(super) fn symmetric_eigen(n: usize, matrix: &[f64]) -> (Vec<f64>, Vec<f64>) {
    let mut a = matrix.to_vec();
    let mut v = vec![0.0; n * n];
    for index in 0..n {
        v[index * n + index] = 1.0;
    }
    for _ in 0..64 {
        let mut off = 0.0_f64;
        let mut diagonal = 0.0_f64;
        for row in 0..n {
            diagonal += a[row * n + row] * a[row * n + row];
            for column in row + 1..n {
                off += a[row * n + column] * a[row * n + column];
            }
        }
        if off <= 1.0e-30 * diagonal.max(f64::MIN_POSITIVE) {
            break;
        }
        for p in 0..n {
            for q in p + 1..n {
                let apq = a[p * n + q];
                if apq == 0.0 {
                    continue;
                }
                let theta = (a[q * n + q] - a[p * n + p]) / (2.0 * apq);
                let t = theta.signum() / (theta.abs() + (theta * theta + 1.0).sqrt());
                let t = if theta == 0.0 { 1.0 } else { t };
                let c = 1.0 / (t * t + 1.0).sqrt();
                let s = t * c;
                for k in 0..n {
                    let (akp, akq) = (a[k * n + p], a[k * n + q]);
                    a[k * n + p] = c * akp - s * akq;
                    a[k * n + q] = s * akp + c * akq;
                }
                for k in 0..n {
                    let (apk, aqk) = (a[p * n + k], a[q * n + k]);
                    a[p * n + k] = c * apk - s * aqk;
                    a[q * n + k] = s * apk + c * aqk;
                }
                for k in 0..n {
                    let (vkp, vkq) = (v[k * n + p], v[k * n + q]);
                    v[k * n + p] = c * vkp - s * vkq;
                    v[k * n + q] = s * vkp + c * vkq;
                }
            }
        }
    }
    ((0..n).map(|index| a[index * n + index]).collect(), v)
}

/// A fixed standard-normal sequence by coefficient index.
fn start_value(index: u64) -> f64 {
    let mix = |mut z: u64| {
        z = z.wrapping_add(0x9e37_79b9_7f4a_7c15);
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        z ^ (z >> 31)
    };
    let unit = |z: u64| ((z >> 11) as f64 + 0.5) / (1_u64 << 53) as f64;
    let first = unit(mix(2 * index));
    let second = unit(mix(2 * index + 1));
    (-2.0 * first.ln()).sqrt() * (core::f64::consts::TAU * second).cos()
}

#[allow(clippy::too_many_arguments, clippy::too_many_lines)]
pub(super) fn leading_direction(
    problem: &CompressedProblem,
    solver: &PreparedModelSolver<'_>,
    inference_rows: ComponentInferenceRows<'_>,
    target: usize,
    start: Option<&ModelCoefficients>,
    cap: u32,
    tolerance: f64,
    receipts: &mut Vec<ComponentInferenceSolveReceipt>,
    interrupt: &mut dyn InterruptCheck,
) -> Result<LeadingDirection> {
    let rows = inference_rows.len(problem);
    if cap < 2 || !tolerance.is_finite() || tolerance <= 0.0 || target >= REPORTED_TARGETS {
        return Err(invalid("leading-direction inputs are invalid"));
    }
    let (workers, firms) = (problem.workers(), problem.firms());
    let coefficients = |vector: &Pair| ModelCoefficients {
        worker: vector.0.clone(),
        firm: vector.1.clone(),
        control: Vec::new(),
    };
    let mut prediction = vec![0.0; rows];
    // `S`-normalise a coefficient vector through its unit-space image.
    let normalise = |vector: &mut Pair,
                     prediction: &mut [f64],
                     interrupt: &mut dyn InterruptCheck|
     -> Result<f64> {
        component_predict(
            problem,
            inference_rows,
            solver,
            &coefficients(vector),
            prediction,
            interrupt,
        )?;
        let norm = component_prediction_inner(prediction, prediction)?.sqrt();
        if !norm.is_finite() {
            return Err(nonfinite("a leading-direction vector is nonfinite"));
        }
        if norm > 0.0 {
            for value in vector.0.iter_mut().chain(vector.1.iter_mut()) {
                *value /= norm;
            }
        }
        Ok(norm)
    };
    let apply = |vector: &Pair, interrupt: &mut dyn InterruptCheck| -> Result<Pair> {
        let rhs = reported_target_rhs(problem, &coefficients(vector), target, interrupt)?;
        Ok((rhs.0, rhs.1))
    };

    let mut first: Pair = match start {
        Some(value) if value.worker.len() == workers && value.firm.len() == firms => {
            (value.worker.clone(), value.firm.clone())
        }
        Some(_) => return Err(invalid("leading-direction start has the wrong dimension")),
        None => (
            (0..workers as u64).map(start_value).collect(),
            (0..firms as u64)
                .map(|index| start_value(workers as u64 + index))
                .collect(),
        ),
    };
    if normalise(&mut first, &mut prediction, interrupt)? <= 0.0 {
        return Err(invalid("leading-direction start vanishes on the units"));
    }
    let mut image = apply(&first, interrupt)?;
    let mut basis = vec![first];
    let mut projected = vec![inner(&basis[0], &image)];
    let mut steps = 0_u32;
    let no_controls: [f64; 0] = [];
    loop {
        interrupt.checkpoint("leading_direction_step")?;
        let solved =
            solver.solve_batch_with_interrupt(&image.0, &image.1, &no_controls, 1, 1, interrupt)?;
        steps += 1;
        let solution = &solved.solution[0];
        receipts.push(component_solve_receipt(
            ComponentInferenceSolvePhase::SpectrumIteration,
            target as u32,
            solution,
        ));
        let mut next: Pair = (
            solution.coefficients.worker.clone(),
            solution.coefficients.firm.clone(),
        );
        drop(solved);
        // `S`-orthogonalise against the basis. For u = S^-1 g the inner
        // product <u, v>_S is g'v.
        let size = basis.len();
        for vector in &basis {
            let weight = inner(&image, vector);
            axpy(&mut next, -weight, vector);
        }
        // The solve is inexact, and near convergence what is left of the new
        // vector is small beside what was removed. A second pass with the
        // normal matrix applied to the vector itself restores orthogonality
        // to rounding; it costs a prediction and a transpose, not a solve.
        component_predict(
            problem,
            inference_rows,
            solver,
            &coefficients(&next),
            &mut prediction,
            interrupt,
        )?;
        let normal = component_transpose_rhs(problem, inference_rows, &prediction, interrupt)?;
        let normal: Pair = (normal.0, normal.1);
        for vector in &basis {
            let weight = inner(&normal, vector);
            axpy(&mut next, -weight, vector);
        }
        drop(normal);
        let norm = normalise(&mut next, &mut prediction, interrupt)?;
        let (values, vectors) = symmetric_eigen(size, &projected);
        let mut order = (0..size).collect::<Vec<_>>();
        order.sort_by(|&a, &b| values[b].abs().total_cmp(&values[a].abs()));
        let lead = order[0];
        let scale = values[lead].abs();
        // A vanishing new vector closes an invariant subspace, and a target
        // without spectrum on the subspace has nothing left to find.
        let invariant = scale == 0.0 || norm <= 1.0e-12 * scale;
        let (border, next_image, residual) = if invariant {
            (Vec::new(), (Vec::new(), Vec::new()), 0.0)
        } else {
            let next_image = apply(&next, interrupt)?;
            let border = basis
                .iter()
                .map(|vector| inner(vector, &next_image))
                .collect::<Vec<_>>();
            let mut residual = StableAccumulator::default();
            for index in 0..size {
                residual.add(border[index] * vectors[index * size + lead]);
            }
            let residual = if scale > 0.0 {
                residual.finish().abs() / scale
            } else {
                f64::INFINITY
            };
            (border, next_image, residual)
        };
        let converged = invariant || (residual <= tolerance && steps >= 2);
        if converged || steps >= cap {
            let mut vector: Pair = (vec![0.0; workers], vec![0.0; firms]);
            for (index, member) in basis.iter().enumerate() {
                axpy(&mut vector, vectors[index * size + lead], member);
            }
            let length = normalise(&mut vector, &mut prediction, interrupt)?;
            if length <= 0.0 {
                return Err(nonfinite("the leading direction vanishes on the units"));
            }
            for value in &mut prediction {
                *value /= length;
            }
            let eigenvalue = inner(&vector, &apply(&vector, interrupt)?);
            if !eigenvalue.is_finite() || !residual.is_finite() {
                return Err(nonfinite("the leading eigenvalue is nonfinite"));
            }
            return Ok(LeadingDirection {
                eigenvalue,
                second_eigenvalue: if size > 1 { values[order[1]] } else { 0.0 },
                residual,
                steps,
                converged,
                mode: prediction,
                coefficients: coefficients(&vector),
            });
        }
        if size >= BASIS {
            // Thick restart: the Ritz vectors of largest magnitude and the
            // new vector span the next basis; their residuals are all
            // multiples of the new vector.
            let kept = &order[..KEPT.min(size)];
            let mut reduced = Vec::with_capacity(kept.len() + 1);
            for &ritz in kept {
                let mut vector: Pair = (vec![0.0; workers], vec![0.0; firms]);
                for (index, member) in basis.iter().enumerate() {
                    axpy(&mut vector, vectors[index * size + ritz], member);
                }
                reduced.push(vector);
            }
            let order_new = kept.len() + 1;
            let mut matrix = vec![0.0; order_new * order_new];
            for (position, &ritz) in kept.iter().enumerate() {
                matrix[position * order_new + position] = values[ritz];
                let mut coupling = StableAccumulator::default();
                for index in 0..size {
                    coupling.add(border[index] * vectors[index * size + ritz]);
                }
                let coupling = coupling.finish();
                matrix[position * order_new + kept.len()] = coupling;
                matrix[kept.len() * order_new + position] = coupling;
            }
            matrix[order_new * order_new - 1] = inner(&next, &next_image);
            basis = reduced;
            projected = matrix;
        } else {
            let order_new = size + 1;
            let mut matrix = vec![0.0; order_new * order_new];
            for row in 0..size {
                for column in 0..size {
                    matrix[row * order_new + column] = projected[row * size + column];
                }
                matrix[row * order_new + size] = border[row];
                matrix[size * order_new + row] = border[row];
            }
            matrix[order_new * order_new - 1] = inner(&next, &next_image);
            projected = matrix;
        }
        basis.push(next);
        image = next_image;
    }
}

#[cfg(test)]
mod tests {
    use super::symmetric_eigen;

    #[test]
    fn jacobi_reproduces_a_known_spectrum() {
        // Eigenvalues 1, 2 and 4 with a rotated basis.
        let q: [[f64; 3]; 3] = [
            [2.0 / 3.0, -2.0 / 3.0, 1.0 / 3.0],
            [1.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0],
            [2.0 / 3.0, 1.0 / 3.0, -2.0 / 3.0],
        ];
        let d = [1.0, 2.0, 4.0];
        let mut matrix = [0.0; 9];
        for row in 0..3 {
            for column in 0..3 {
                matrix[row * 3 + column] = (0..3).map(|k| q[row][k] * d[k] * q[column][k]).sum();
            }
        }
        let (values, vectors) = symmetric_eigen(3, &matrix);
        let mut sorted = values.clone();
        sorted.sort_by(f64::total_cmp);
        for (value, expected) in sorted.iter().zip(d) {
            assert!((value - expected).abs() <= 1.0e-13);
        }
        for k in 0..3 {
            for row in 0..3 {
                let image: f64 = (0..3)
                    .map(|column| matrix[row * 3 + column] * vectors[column * 3 + k])
                    .sum();
                assert!((image - values[k] * vectors[row * 3 + k]).abs() <= 1.0e-13);
            }
        }
    }
}
