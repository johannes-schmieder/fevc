// SPDX-License-Identifier: GPL-3.0-only

use super::*;

struct Fixture {
    worker: Vec<u32>,
    firm: Vec<u32>,
    weights: Vec<f64>,
    controls: Vec<Vec<f64>>,
    worker_rhs: Vec<f64>,
    firm_rhs: Vec<f64>,
    control_rhs: Vec<f64>,
    expected: ModelCoefficients,
}

impl Fixture {
    fn new(controls: usize) -> Self {
        let expected = ModelCoefficients {
            worker: vec![0.2, -0.3, 1.1, -0.7],
            firm: vec![0.5, -0.2, -0.3],
            control: vec![0.4; controls],
        };
        let mut value = Self {
            worker: Vec::new(),
            firm: Vec::new(),
            weights: Vec::new(),
            controls: vec![Vec::new(); controls],
            worker_rhs: vec![0.0; 4],
            firm_rhs: vec![0.0; 3],
            control_rhs: vec![0.0; controls],
            expected,
        };
        // Independent physical-row construction of X'WX beta. Its known
        // solution tests the complete coefficients, not just a solver residual.
        for w in 0..4 {
            for f in 0..3 {
                for copy in 0..3 {
                    let row = value.worker.len();
                    let weight = 1.0 + ((row * 7) % 5) as f64;
                    let x = ((row + 1) as f64 * 0.7).sin() + copy as f64;
                    let y = value.expected.worker[w]
                        + value.expected.firm[f]
                        + if controls == 0 { 0.0 } else { 0.4 * x };
                    value.worker.push(w as u32);
                    value.firm.push(f as u32);
                    value.weights.push(weight);
                    value.worker_rhs[w] += weight * y;
                    value.firm_rhs[f] += weight * y;
                    if controls == 1 {
                        value.controls[0].push(x);
                        value.control_rhs[0] += weight * x * y;
                    }
                }
            }
        }
        value
    }

    fn data(&self) -> CanonicalModelData<'_> {
        CanonicalModelData {
            workers: 4,
            firms: 3,
            row_worker: &self.worker,
            row_firm: &self.firm,
            weight: &self.weights,
            controls: &self.controls,
        }
    }

    fn rhs(&self) -> ModelRhs<'_> {
        ModelRhs {
            worker: &self.worker_rhs,
            firm: &self.firm_rhs,
            control: &self.control_rhs,
        }
    }

    fn check(&self, solved: &ModelSolve) {
        for (actual, expected) in solved
            .coefficients
            .worker
            .iter()
            .chain(&solved.coefficients.firm)
            .chain(&solved.coefficients.control)
            .zip(
                self.expected
                    .worker
                    .iter()
                    .chain(&self.expected.firm)
                    .chain(&self.expected.control),
            )
        {
            assert!((actual - expected).abs() < 1e-9, "{actual} != {expected}");
        }
        let mut residual: Vec<_> = self
            .worker_rhs
            .iter()
            .chain(&self.firm_rhs)
            .chain(&self.control_rhs)
            .copied()
            .collect();
        for row in 0..self.worker.len() {
            let w = self.worker[row] as usize;
            let f = self.firm[row] as usize;
            let prediction = solved.coefficients.worker[w]
                + solved.coefficients.firm[f]
                + self
                    .controls
                    .iter()
                    .zip(&solved.coefficients.control)
                    .map(|(x, b)| x[row] * b)
                    .sum::<f64>();
            residual[w] -= self.weights[row] * prediction;
            residual[4 + f] -= self.weights[row] * prediction;
            for (q, x) in self.controls.iter().enumerate() {
                residual[7 + q] -= self.weights[row] * x[row] * prediction;
            }
        }
        let norm = |x: &[f64]| x.iter().map(|v| v * v).sum::<f64>().sqrt();
        let rhs: Vec<_> = self
            .worker_rhs
            .iter()
            .chain(&self.firm_rhs)
            .chain(&self.control_rhs)
            .copied()
            .collect();
        let relative = norm(&residual) / norm(&rhs);
        assert!(relative <= solved.receipt.full_residual_tolerance);
        assert!((relative - solved.receipt.full_residual).abs() < 1e-14);
    }
}

#[derive(Default)]
struct Refinements {
    attempts: usize,
    cancel: bool,
}
impl InterruptCheck for Refinements {
    fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
        if phase == "model_diagonal_refinement" {
            self.attempts += 1;
            if self.cancel {
                return Err(BackendError::new(ErrorCode::UserBreak, phase, "test break"));
            }
        }
        Ok(())
    }
}

#[test]
fn diagonal_full_residual_refinement_matches_independent_rows_and_preserves_gate() {
    for controls in [0, 1] {
        let fixture = Fixture::new(controls);
        let options = ModelSolverOptions::default();
        let solver = PreparedModelSolver::prepare(fixture.data(), options).unwrap();
        let mut check = Refinements::default();
        let exact = solver
            .solve_with_interrupt(fixture.rhs(), &mut check)
            .unwrap();
        assert_eq!(check.attempts, 0);
        fixture.check(&exact);
        let mut reduced: Vec<_> = exact
            .coefficients
            .firm
            .iter()
            .chain(&exact.coefficients.control)
            .copied()
            .collect();
        // Inject only at the post-PCG boundary to exercise a full-equation
        // failure deterministically on every architecture.
        reduced[0] += 0.003;
        reduced[1] -= 0.003;
        let mut cancelled = Refinements {
            cancel: true,
            ..Default::default()
        };
        assert_eq!(
            solver
                .finish_solution(
                    fixture.rhs(),
                    reduced.clone(),
                    exact.receipt.pcg.clone(),
                    options,
                    &mut cancelled
                )
                .unwrap_err()
                .code,
            ErrorCode::UserBreak
        );
        let corrected = solver
            .finish_solution(
                fixture.rhs(),
                reduced,
                exact.receipt.pcg.clone(),
                options,
                &mut check,
            )
            .unwrap();
        assert!(check.attempts > 0 && check.attempts <= 3);
        assert_eq!(
            corrected.receipt.full_residual_tolerance,
            options.full_residual_tolerance()
        );
        assert!(corrected.receipt.pcg.iterations > exact.receipt.pcg.iterations);
        assert!(
            corrected.receipt.pcg.operator_applications > exact.receipt.pcg.operator_applications
        );
        assert!(
            corrected.receipt.pcg.preconditioner_applications
                > exact.receipt.pcg.preconditioner_applications
        );
        fixture.check(&corrected);
        fixture.check(&solver.solve(fixture.rhs()).unwrap());
    }
}

#[test]
fn diagonal_refinement_is_bounded_and_cannot_hide_incompatible_original_equations() {
    let fixture = Fixture::new(1);
    let options = ModelSolverOptions::default();
    let solver = PreparedModelSolver::prepare(fixture.data(), options).unwrap();
    let exact = solver.solve(fixture.rhs()).unwrap();
    let mut worker = fixture.worker_rhs.clone();
    worker[0] += 1.0;
    let rhs = ModelRhs {
        worker: &worker,
        ..fixture.rhs()
    };
    let reduced = exact
        .coefficients
        .firm
        .iter()
        .chain(&exact.coefficients.control)
        .copied()
        .collect();
    let mut check = Refinements::default();
    let error = solver
        .finish_solution(rhs, reduced, exact.receipt.pcg, options, &mut check)
        .unwrap_err();
    assert_eq!(error.code, ErrorCode::FullResidualFailed);
    assert_eq!(check.attempts, 3);
    fixture.check(&solver.solve(fixture.rhs()).unwrap());
}
