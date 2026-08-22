// SPDX-License-Identifier: GPL-3.0-only

use vckss_core::cmg::CmgOptions;
use vckss_core::error::{BackendError, ErrorCode, Result};
use vckss_core::generic_batch::{
    apply_model_batch, ModelBatchWorkspace, ModelBatchWorkspaceLayout, ModelPcgStatus,
};
use vckss_core::interrupt::InterruptCheck;
use vckss_core::krylov::PcgOptions;
use vckss_core::model_operator::{CanonicalModelData, ModelOperator, ModelRhs, ModelWorkspace};
use vckss_core::model_solver::{
    solve_model_diagonal_pcg, ControlRankReceipt, ModelRoutingOptions, ModelSolverOptions,
    ModelSolverRoute, PreparedModelSolver,
};
use vckss_core::operator::{stable_dot, SymmetricOperator};
use vckss_core::problem::CanonicalInput;
use vckss_core::solver::{LinearSolverOptions, LinearSolverRoute, PreparedTwoWaySolver};
use vckss_core::types::InputColumns;

#[derive(Clone, Debug)]
struct Fixture {
    workers: usize,
    firms: usize,
    worker: Vec<u32>,
    firm: Vec<u32>,
    weight: Vec<f64>,
    controls: Vec<Vec<f64>>,
}

impl Fixture {
    fn data(&self) -> CanonicalModelData<'_> {
        CanonicalModelData {
            workers: self.workers,
            firms: self.firms,
            row_worker: &self.worker,
            row_firm: &self.firm,
            weight: &self.weight,
            controls: &self.controls,
        }
    }
}

#[derive(Clone, Debug)]
struct OwnedRhs {
    worker: Vec<f64>,
    firm: Vec<f64>,
    control: Vec<f64>,
}

impl OwnedRhs {
    fn view(&self) -> ModelRhs<'_> {
        ModelRhs {
            worker: &self.worker,
            firm: &self.firm,
            control: &self.control,
        }
    }
}

fn options() -> ModelSolverOptions {
    ModelSolverOptions {
        pcg: PcgOptions {
            tolerance: 1.0e-12,
            maximum_iterations: 2_000,
            residual_replacement_interval: 11,
        },
        rank_tolerance: 1.0e-11,
    }
}

fn routed_options(route: ModelSolverRoute) -> ModelRoutingOptions {
    ModelRoutingOptions {
        route,
        cmg_minimum_dimension: 1,
        allow_automatic_cmg_setup_fallback: true,
        solver: options(),
        cmg: CmgOptions::default(),
    }
}

fn fixed_fixture(controls: usize) -> Fixture {
    let workers = 4;
    let firms = 3;
    let mut worker = Vec::new();
    let mut firm = Vec::new();
    let mut weight = Vec::new();
    for worker_index in 0..workers {
        for firm_index in 0..firms {
            worker.push(worker_index as u32);
            firm.push(firm_index as u32);
            weight.push(0.75 + ((worker_index * 5 + firm_index * 3) % 7) as f64 / 4.0);
        }
    }
    let candidates = [
        vec![
            -1.2, 0.3, 0.8, 0.4, -0.7, 1.1, 1.5, -0.2, -0.9, -0.5, 1.3, 0.6,
        ],
        vec![
            0.2, -1.0, 1.4, -0.3, 0.9, 0.5, -1.1, 0.7, 0.1, 1.2, -0.6, -0.4,
        ],
        vec![
            1.1, 0.1, -0.8, -1.3, 0.6, 0.9, 0.2, -1.0, 1.4, -0.4, -0.7, 0.5,
        ],
    ];
    Fixture {
        workers,
        firms,
        worker,
        firm,
        weight,
        controls: candidates[..controls].to_vec(),
    }
}

fn random_fixture(seed: u64, controls: usize) -> Fixture {
    let workers = 5;
    let firms = 4;
    let rows = workers * firms;
    let mut state = seed;
    let mut draw = || {
        state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let unit = ((state >> 11) as f64) / ((1_u64 << 53) as f64);
        2.0 * unit - 1.0
    };
    let mut worker = Vec::with_capacity(rows);
    let mut firm = Vec::with_capacity(rows);
    let mut weight = Vec::with_capacity(rows);
    for worker_index in 0..workers {
        for firm_index in 0..firms {
            worker.push(worker_index as u32);
            firm.push(firm_index as u32);
            weight.push(1.25 + 0.5 * draw());
        }
    }
    let mut columns = Vec::with_capacity(controls);
    for _ in 0..controls {
        columns.push((0..rows).map(|_| draw()).collect());
    }
    Fixture {
        workers,
        firms,
        worker,
        firm,
        weight,
        controls: columns,
    }
}

fn wide_control_fixture(controls: usize) -> Fixture {
    let workers = 9;
    let firms = 9;
    let rows = workers * firms;
    let mut worker = Vec::with_capacity(rows);
    let mut firm = Vec::with_capacity(rows);
    let mut weight = Vec::with_capacity(rows);
    for worker_index in 0..workers {
        for firm_index in 0..firms {
            worker.push(worker_index as u32);
            firm.push(firm_index as u32);
            weight.push(1.0 + ((worker_index * 11 + firm_index * 7) % 13) as f64 / 17.0);
        }
    }
    let mut state = 0x4d59_5df4_d0f3_3173_u64;
    let mut columns = Vec::with_capacity(controls);
    for control in 0..controls {
        let mut column = Vec::with_capacity(rows);
        for row in 0..rows {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let random = ((state >> 11) as f64) / ((1_u64 << 53) as f64) - 0.5;
            column.push(random + 0.01 * ((row * (control + 3)) % 17) as f64);
        }
        columns.push(column);
    }
    Fixture {
        workers,
        firms,
        worker,
        firm,
        weight,
        controls: columns,
    }
}

fn coefficients(fixture: &Fixture, seed: u64) -> (Vec<f64>, Vec<f64>, Vec<f64>) {
    let mut worker = (0..fixture.workers)
        .map(|index| ((index * 7 + seed as usize) % 13) as f64 / 7.0 - 0.8)
        .collect::<Vec<_>>();
    let mut firm = (0..fixture.firms)
        .map(|index| ((index * 11 + seed as usize) % 17) as f64 / 9.0 - 0.7)
        .collect::<Vec<_>>();
    let firm_mean = firm.iter().sum::<f64>() / firm.len() as f64;
    for value in &mut firm {
        *value -= firm_mean;
    }
    let control = (0..fixture.controls.len())
        .map(|index| 0.35 * (index + 1) as f64 - 0.2)
        .collect::<Vec<_>>();
    worker[0] += 0.125;
    (worker, firm, control)
}

fn normal_rhs(
    fixture: &Fixture,
    worker_coef: &[f64],
    firm_coef: &[f64],
    control_coef: &[f64],
) -> OwnedRhs {
    let mut rhs = OwnedRhs {
        worker: vec![0.0; fixture.workers],
        firm: vec![0.0; fixture.firms],
        control: vec![0.0; fixture.controls.len()],
    };
    for row in 0..fixture.weight.len() {
        let worker = fixture.worker[row] as usize;
        let firm = fixture.firm[row] as usize;
        let prediction = worker_coef[worker]
            + firm_coef[firm]
            + fixture
                .controls
                .iter()
                .zip(control_coef)
                .map(|(column, coefficient)| column[row] * coefficient)
                .sum::<f64>();
        let value = fixture.weight[row] * prediction;
        rhs.worker[worker] += value;
        rhs.firm[firm] += value;
        for (control, column) in rhs.control.iter_mut().zip(&fixture.controls) {
            *control += column[row] * value;
        }
    }
    rhs
}

/// Independent dense KKT oracle for min ||sqrt(omega)(y-X beta)|| with the
/// explicit sum(phi)=0 constraint.
fn dense_kkt(fixture: &Fixture, rhs: &OwnedRhs) -> Vec<f64> {
    let parameters = fixture.workers + fixture.firms + fixture.controls.len();
    let dimension = parameters + 1;
    let mut matrix = vec![0.0; dimension * dimension];
    for row in 0..fixture.weight.len() {
        let mut nonzero = vec![
            (fixture.worker[row] as usize, 1.0),
            (fixture.workers + fixture.firm[row] as usize, 1.0),
        ];
        nonzero.extend(
            fixture
                .controls
                .iter()
                .enumerate()
                .map(|(control, column)| (fixture.workers + fixture.firms + control, column[row])),
        );
        for &(left, left_value) in &nonzero {
            for &(right, right_value) in &nonzero {
                matrix[left * dimension + right] += fixture.weight[row] * left_value * right_value;
            }
        }
    }
    for firm in 0..fixture.firms {
        let index = fixture.workers + firm;
        matrix[index * dimension + parameters] = 1.0;
        matrix[parameters * dimension + index] = 1.0;
    }
    let mut right_hand_side = Vec::new();
    right_hand_side.extend_from_slice(&rhs.worker);
    right_hand_side.extend_from_slice(&rhs.firm);
    right_hand_side.extend_from_slice(&rhs.control);
    right_hand_side.push(0.0);
    gaussian_solve(matrix, right_hand_side, dimension)[..parameters].to_vec()
}

fn gaussian_solve(mut matrix: Vec<f64>, mut rhs: Vec<f64>, dimension: usize) -> Vec<f64> {
    for pivot in 0..dimension {
        let selected = (pivot..dimension)
            .max_by(|&left, &right| {
                matrix[left * dimension + pivot]
                    .abs()
                    .total_cmp(&matrix[right * dimension + pivot].abs())
            })
            .expect("pivot");
        assert!(matrix[selected * dimension + pivot].abs() > 1.0e-12);
        if selected != pivot {
            for column in 0..dimension {
                matrix.swap(pivot * dimension + column, selected * dimension + column);
            }
            rhs.swap(pivot, selected);
        }
        let diagonal = matrix[pivot * dimension + pivot];
        for column in pivot..dimension {
            matrix[pivot * dimension + column] /= diagonal;
        }
        rhs[pivot] /= diagonal;
        for row in 0..dimension {
            if row == pivot {
                continue;
            }
            let factor = matrix[row * dimension + pivot];
            for column in pivot..dimension {
                matrix[row * dimension + column] -= factor * matrix[pivot * dimension + column];
            }
            rhs[row] -= factor * rhs[pivot];
        }
    }
    rhs
}

fn assert_close(left: &[f64], right: &[f64], tolerance: f64) {
    assert_eq!(left.len(), right.len());
    for (index, (&left, &right)) in left.iter().zip(right).enumerate() {
        assert!(
            (left - right).abs() <= tolerance * (1.0 + left.abs().max(right.abs())),
            "index {index}: {left} != {right}"
        );
    }
}

fn assert_rank_receipt_close(
    left: &ControlRankReceipt,
    right: &ControlRankReceipt,
    tolerance: f64,
) {
    assert_eq!(left.controls, right.controls);
    assert_close(&[left.rcond], &[right.rcond], tolerance);
    assert_close(
        &[left.smallest_generalized_eigenvalue_lower],
        &[right.smallest_generalized_eigenvalue_lower],
        tolerance,
    );
    assert_close(
        &[left.largest_generalized_eigenvalue_upper],
        &[right.largest_generalized_eigenvalue_upper],
        tolerance,
    );
    assert_close(
        &[left.projection_error_bound],
        &[right.projection_error_bound],
        tolerance,
    );
    assert_close(
        &[left.normalization_error_bound],
        &[right.normalization_error_bound],
        tolerance,
    );
    assert_close(
        &[left.fe_information_eigenvalue_lower_bound],
        &[right.fe_information_eigenvalue_lower_bound],
        tolerance,
    );
    assert_close(
        &[left.maximum_projection_residual],
        &[right.maximum_projection_residual],
        tolerance,
    );
}

#[test]
fn model_action_is_symmetric_centered_and_has_the_exact_diagonal() {
    let fixture = fixed_fixture(3);
    let operator = ModelOperator::new(fixture.data()).expect("operator");
    let dimension = operator.parameter_count();
    let mut left = vec![0.2, -0.7, 0.5, 0.4, -0.3, 0.8];
    let mut right = vec![-0.6, 0.1, 0.5, 0.7, 0.2, -0.4];
    operator
        .project_parameters(&mut left)
        .expect("left projection");
    operator
        .project_parameters(&mut right)
        .expect("right projection");
    let mut action_left = vec![0.0; dimension];
    let mut action_right = vec![0.0; dimension];
    operator
        .apply(&left, &mut action_left)
        .expect("left action");
    operator
        .apply(&right, &mut action_right)
        .expect("right action");
    assert!((action_left[..fixture.firms].iter().sum::<f64>()).abs() < 1.0e-13);
    assert!((stable_dot(&left, &action_right) - stable_dot(&right, &action_left)).abs() < 1.0e-12);

    for coordinate in 0..dimension {
        let mut basis = vec![0.0; dimension];
        basis[coordinate] = 1.0;
        let mut action = vec![0.0; dimension];
        operator.apply(&basis, &mut action).expect("basis action");
        assert!(
            (action[coordinate] - operator.reduced_diagonal()[coordinate]).abs() < 1.0e-12,
            "diagonal coordinate {coordinate}"
        );
    }
}

#[test]
fn forced_cmg_block_is_symmetric_positive_and_matches_diagonal_dense_solution() {
    let fixture = fixed_fixture(2);
    let cmg =
        PreparedModelSolver::prepare_routed(fixture.data(), routed_options(ModelSolverRoute::Cmg))
            .expect("forced generic CMG prepare");
    assert_eq!(cmg.receipt().requested, ModelSolverRoute::Cmg);
    assert_eq!(cmg.receipt().selected, ModelSolverRoute::Cmg);
    assert!(cmg.receipt().cmg.is_some());
    assert!(cmg.receipt().fallback.is_none());

    let left = vec![0.4, -0.7, 0.3, 0.2, -0.5];
    let right = vec![-0.6, 0.1, 0.5, 0.8, -0.2];
    let mut preconditioned_left = vec![0.0; left.len()];
    let mut preconditioned_right = vec![0.0; right.len()];
    cmg.apply_preconditioner(&left, &mut preconditioned_left)
        .expect("left CMG action");
    cmg.apply_preconditioner(&right, &mut preconditioned_right)
        .expect("right CMG action");
    let left_cross = stable_dot(&left, &preconditioned_right);
    let right_cross = stable_dot(&preconditioned_left, &right);
    assert!((left_cross - right_cross).abs() < 2.0e-11 * (1.0 + left_cross.abs()));
    assert!(stable_dot(&left, &preconditioned_left) > 0.0);

    let truth = coefficients(&fixture, 23);
    let rhs = normal_rhs(&fixture, &truth.0, &truth.1, &truth.2);
    let cmg_solution = cmg.solve(rhs.view()).expect("forced CMG solve");
    let diagonal = PreparedModelSolver::prepare(fixture.data(), options())
        .expect("diagonal prepare")
        .solve(rhs.view())
        .expect("diagonal solve");
    assert_close(
        &cmg_solution.coefficients.worker,
        &diagonal.coefficients.worker,
        2.0e-10,
    );
    assert_close(
        &cmg_solution.coefficients.firm,
        &diagonal.coefficients.firm,
        2.0e-10,
    );
    assert_close(
        &cmg_solution.coefficients.control,
        &diagonal.coefficients.control,
        2.0e-10,
    );
    assert!(cmg_solution.residual.relative_norm <= options().full_residual_tolerance());

    let mut worker_batch = rhs.worker.clone();
    worker_batch.extend(rhs.worker.iter().map(|value| -value));
    let mut firm_batch = rhs.firm.clone();
    firm_batch.extend(rhs.firm.iter().map(|value| -value));
    let mut control_batch = rhs.control.clone();
    control_batch.extend(rhs.control.iter().map(|value| -value));
    let narrow = cmg
        .solve_batch(&worker_batch, &firm_batch, &control_batch, 2, 1)
        .expect("narrow CMG batch");
    let wide = cmg
        .solve_batch(&worker_batch, &firm_batch, &control_batch, 2, 2)
        .expect("wide CMG batch");
    for column in 0..2 {
        assert_eq!(
            narrow.solution[column].coefficients.worker,
            wide.solution[column].coefficients.worker
        );
        assert_eq!(
            narrow.solution[column].coefficients.firm,
            wide.solution[column].coefficients.firm
        );
        assert_eq!(
            narrow.solution[column].coefficients.control,
            wide.solution[column].coefficients.control
        );
        assert_eq!(
            narrow.solution[column].receipt.pcg.iterations,
            wide.solution[column].receipt.pcg.iterations
        );
    }
}

#[test]
fn generic_cmg_routing_falls_back_only_during_eligible_auto_setup() {
    let fixture = fixed_fixture(1);
    let mut failing = routed_options(ModelSolverRoute::Cmg);
    failing.cmg.memory_limit_bytes = 1;
    let forced_error = PreparedModelSolver::prepare_routed(fixture.data(), failing)
        .expect_err("forced CMG setup must fail closed");
    assert_eq!(forced_error.code, ErrorCode::ResourceLimit);

    failing.route = ModelSolverRoute::Auto;
    let automatic = PreparedModelSolver::prepare_routed(fixture.data(), failing)
        .expect("eligible automatic CMG setup failure may fall back");
    assert_eq!(automatic.receipt().requested, ModelSolverRoute::Auto);
    assert_eq!(automatic.receipt().selected, ModelSolverRoute::Diagonal);
    let fallback = automatic
        .receipt()
        .fallback
        .as_ref()
        .expect("fallback receipt");
    assert_eq!(fallback.from, ModelSolverRoute::Cmg);
    assert_eq!(fallback.to, ModelSolverRoute::Diagonal);

    let receipt_before = automatic.receipt().clone();
    let truth = coefficients(&fixture, 31);
    let rhs = normal_rhs(&fixture, &truth.0, &truth.1, &truth.2);
    automatic.solve(rhs.view()).expect("frozen fallback solve");
    assert_eq!(automatic.receipt().requested, receipt_before.requested);
    assert_eq!(automatic.receipt().selected, receipt_before.selected);
    assert_eq!(
        automatic.receipt().fallback.as_ref().map(|item| item.code),
        receipt_before.fallback.as_ref().map(|item| item.code)
    );

    let mut below_threshold = failing;
    below_threshold.cmg_minimum_dimension = usize::MAX;
    let direct_diagonal = PreparedModelSolver::prepare_routed(fixture.data(), below_threshold)
        .expect("auto below the CMG threshold must not attempt setup");
    assert_eq!(
        direct_diagonal.receipt().selected,
        ModelSolverRoute::Diagonal
    );
    assert!(direct_diagonal.receipt().fallback.is_none());

    let singular_control = fixture
        .firm
        .iter()
        .map(|&firm| f64::from(firm == 0))
        .collect::<Vec<_>>();
    let singular = Fixture {
        controls: vec![singular_control],
        ..fixture
    };
    let singular_error = PreparedModelSolver::prepare_routed(singular.data(), failing)
        .expect_err("scientific rank failure must never fall back");
    assert_eq!(singular_error.code, ErrorCode::SingularInformation);
}

#[test]
fn generic_cmg_supports_the_certified_control_count_boundaries() {
    for controls in [1_usize, 4, 16, 32] {
        let fixture = wide_control_fixture(controls);
        let solver = PreparedModelSolver::prepare_routed(
            fixture.data(),
            routed_options(ModelSolverRoute::Cmg),
        )
        .unwrap_or_else(|error| panic!("Q={controls} CMG prepare failed: {error}"));
        assert_eq!(solver.receipt().selected, ModelSolverRoute::Cmg);
        assert_eq!(solver.control_rank_receipt().controls, controls);
        let dimension = fixture.firms + controls;
        let mut input = (0..dimension)
            .map(|index| ((index * 17 + controls) % 29) as f64 / 13.0 - 0.8)
            .collect::<Vec<_>>();
        let firm_mean = input[..fixture.firms].iter().sum::<f64>() / fixture.firms as f64;
        for value in &mut input[..fixture.firms] {
            *value -= firm_mean;
        }
        let mut output = vec![0.0; dimension];
        solver
            .apply_preconditioner(&input, &mut output)
            .unwrap_or_else(|error| panic!("Q={controls} CMG action failed: {error}"));
        assert!(output.iter().all(|value| value.is_finite()));
        assert!(stable_dot(&input, &output) > 0.0);

        if controls <= 4 {
            let truth = coefficients(&fixture, 41 + controls as u64);
            let rhs = normal_rhs(&fixture, &truth.0, &truth.1, &truth.2);
            let solved = solver
                .solve(rhs.view())
                .unwrap_or_else(|error| panic!("Q={controls} CMG solve failed: {error}"));
            assert!(solved.residual.relative_norm <= options().full_residual_tolerance());
        }
    }
}

#[test]
fn reduction_reconstruction_and_prediction_recover_a_known_model() {
    let fixture = fixed_fixture(2);
    let operator = ModelOperator::new(fixture.data()).expect("operator");
    let (worker, firm, control) = coefficients(&fixture, 3);
    let rhs = normal_rhs(&fixture, &worker, &firm, &control);
    let reduced = operator.reduce_rhs(rhs.view()).expect("reduced RHS");
    let mut parameter = firm.clone();
    parameter.extend_from_slice(&control);
    let mut action = vec![0.0; parameter.len()];
    operator
        .apply(&parameter, &mut action)
        .expect("known action");
    assert_close(&reduced, &action, 2.0e-13);
    let reconstructed = operator
        .reconstruct_worker(&rhs.worker, &parameter)
        .expect("worker reconstruction");
    assert_close(&reconstructed, &worker, 2.0e-13);
    let mut predicted = vec![0.0; fixture.weight.len()];
    operator
        .predict_into(&worker, &firm, &control, &mut predicted)
        .expect("prediction");
    for row in 0..predicted.len() {
        let expected = worker[fixture.worker[row] as usize]
            + firm[fixture.firm[row] as usize]
            + fixture
                .controls
                .iter()
                .zip(&control)
                .map(|(column, coefficient)| column[row] * coefficient)
                .sum::<f64>();
        assert!((predicted[row] - expected).abs() < 2.0e-15);
    }
}

#[test]
fn weighted_random_q_zero_through_three_match_dense_constrained_kkt() {
    for controls in 0..=3 {
        for seed in 1..=4 {
            let fixture = random_fixture(100 * controls as u64 + seed, controls);
            let truth = coefficients(&fixture, seed);
            let rhs = normal_rhs(&fixture, &truth.0, &truth.1, &truth.2);
            let dense = dense_kkt(&fixture, &rhs);
            let solver = PreparedModelSolver::prepare(fixture.data(), options()).expect("prepare");
            let solved = solver.solve(rhs.view()).expect("solve");
            assert_close(
                &solved.coefficients.worker,
                &dense[..fixture.workers],
                2.0e-10,
            );
            assert_close(
                &solved.coefficients.firm,
                &dense[fixture.workers..fixture.workers + fixture.firms],
                2.0e-10,
            );
            assert_close(
                &solved.coefficients.control,
                &dense[fixture.workers + fixture.firms..],
                2.0e-10,
            );
            assert!(solved.residual.relative_norm <= 1.0e-11);

            let cmg = PreparedModelSolver::prepare_routed(
                fixture.data(),
                routed_options(ModelSolverRoute::Cmg),
            )
            .expect("CMG prepare");
            let cmg_solved = cmg.solve(rhs.view()).expect("CMG solve");
            assert_close(
                &cmg_solved.coefficients.worker,
                &dense[..fixture.workers],
                2.0e-10,
            );
            assert_close(
                &cmg_solved.coefficients.firm,
                &dense[fixture.workers..fixture.workers + fixture.firms],
                2.0e-10,
            );
            assert_close(
                &cmg_solved.coefficients.control,
                &dense[fixture.workers + fixture.firms..],
                2.0e-10,
            );
            assert!(cmg_solved.residual.relative_norm <= 1.0e-11);
        }
    }
}

#[test]
fn preparation_rejects_controls_in_the_joint_worker_firm_span_before_any_rhs_shortcut() {
    let base = fixed_fixture(0);
    let firm_indicator = base
        .firm
        .iter()
        .map(|firm| f64::from(*firm == 0))
        .collect::<Vec<_>>();
    let worker_plus_firm = base
        .worker
        .iter()
        .zip(&base.firm)
        .map(|(&worker, &firm)| f64::from(worker == 1) + 2.0 * f64::from(firm == 2))
        .collect::<Vec<_>>();
    for control in [firm_indicator, worker_plus_firm] {
        let fixture = Fixture {
            controls: vec![control],
            ..base.clone()
        };
        let zero_worker = vec![0.0; fixture.workers];
        let zero_firm = vec![0.0; fixture.firms];
        let zero_control = vec![0.0; 1];
        let truth = coefficients(&fixture, 17);
        let rhs = normal_rhs(&fixture, &truth.0, &truth.1, &truth.2);
        for caller_tolerance in [f64::MIN_POSITIVE, 1.0e-12, 1.0e-6, 0.1, 1.0] {
            let mut caller_options = options();
            caller_options.pcg.tolerance = caller_tolerance;
            let zero_error = solve_model_diagonal_pcg(
                fixture.data(),
                ModelRhs {
                    worker: &zero_worker,
                    firm: &zero_firm,
                    control: &zero_control,
                },
                caller_options,
            )
            .expect_err("rank failure must precede zero-RHS acceptance");
            assert_eq!(zero_error.code, ErrorCode::SingularInformation);

            let nonzero_error =
                solve_model_diagonal_pcg(fixture.data(), rhs.view(), caller_options)
                    .expect_err("rank failure must precede nonzero solve");
            assert_eq!(nonzero_error.code, ErrorCode::SingularInformation);
        }
    }
}

#[test]
fn generalized_rank_certificate_is_firm_relabel_and_control_coordinate_invariant() {
    // Review witness: unit-weight complete 2-by-3 graph with two distinct
    // worker-1 cell indicators. Its exact generalized residualized spectrum
    // is {1/6, 1/2}, hence rcond 1/3 and rank_tolerance 0.2 must be admitted.
    let worker = vec![0, 0, 0, 1, 1, 1];
    let firm = vec![0, 1, 2, 0, 1, 2];
    let weight = vec![1.0; 6];
    let first = worker
        .iter()
        .zip(&firm)
        .map(|(&worker, &firm)| f64::from(worker == 1 && firm == 1))
        .collect::<Vec<_>>();
    let second = worker
        .iter()
        .zip(&firm)
        .map(|(&worker, &firm)| f64::from(worker == 1 && firm == 2))
        .collect::<Vec<_>>();
    let original = Fixture {
        workers: 2,
        firms: 3,
        worker,
        firm,
        weight,
        controls: vec![first, second],
    };
    let relabeled = Fixture {
        firm: original
            .firm
            .iter()
            .map(|&firm| [1_u32, 0, 2][firm as usize])
            .collect(),
        ..original.clone()
    };
    let permuted = Fixture {
        controls: vec![original.controls[1].clone(), original.controls[0].clone()],
        ..original.clone()
    };
    let transformed = Fixture {
        controls: vec![
            original.controls[0]
                .iter()
                .zip(&original.controls[1])
                .map(|(&first, &second)| 2.0 * first + second)
                .collect(),
            original.controls[0]
                .iter()
                .zip(&original.controls[1])
                .map(|(&first, &second)| first - 3.0 * second)
                .collect(),
        ],
        ..original.clone()
    };
    let fixtures = [&original, &relabeled, &permuted, &transformed];

    let mut witness_options = options();
    witness_options.rank_tolerance = 0.2;
    let receipts = fixtures
        .iter()
        .map(|fixture| {
            PreparedModelSolver::prepare(fixture.data(), witness_options)
                .expect("the exact 2-by-3 witness must be identified")
                .control_rank_receipt()
                .clone()
        })
        .collect::<Vec<_>>();
    assert_rank_receipt_close(&receipts[0], &receipts[1], 1.0e-12);
    assert_close(&[receipts[0].rcond], &[1.0 / 3.0], 1.0e-10);
    assert_close(&[receipts[0].rcond], &[receipts[2].rcond], 1.0e-10);
    assert_close(&[receipts[0].rcond], &[receipts[3].rcond], 1.0e-10);

    let mut cmg_witness_options = routed_options(ModelSolverRoute::Cmg);
    cmg_witness_options.solver.rank_tolerance = 0.2;
    let cmg_receipts = fixtures
        .iter()
        .map(|fixture| {
            let solver = PreparedModelSolver::prepare_routed(fixture.data(), cmg_witness_options)
                .expect("CMG preparation must preserve witness identification");
            assert_eq!(solver.receipt().selected, ModelSolverRoute::Cmg);
            solver.control_rank_receipt().clone()
        })
        .collect::<Vec<_>>();
    assert_rank_receipt_close(&cmg_receipts[0], &cmg_receipts[1], 1.0e-12);
    assert_close(&[cmg_receipts[0].rcond], &[cmg_receipts[2].rcond], 1.0e-10);
    assert_close(&[cmg_receipts[0].rcond], &[cmg_receipts[3].rcond], 1.0e-10);

    // The same common threshold separates every relabeling and nonsingular
    // control coordinate system on both sides of the admission boundary.
    let boundary = receipts[0].rcond;
    for fixture in fixtures {
        let mut below = options();
        below.rank_tolerance = boundary * (1.0 - 1.0e-7);
        PreparedModelSolver::prepare(fixture.data(), below)
            .expect("rank just below the certified boundary must be admitted");

        let mut above = options();
        above.rank_tolerance = boundary * (1.0 + 1.0e-7);
        let error = PreparedModelSolver::prepare(fixture.data(), above)
            .expect_err("rank just above the certified boundary must fail closed");
        assert_eq!(error.code, ErrorCode::SingularInformation);
    }
}

#[test]
fn complete_residual_catches_last_firm_and_control_equations() {
    let fixture = fixed_fixture(2);
    let truth = coefficients(&fixture, 9);
    let rhs = normal_rhs(&fixture, &truth.0, &truth.1, &truth.2);
    let operator = ModelOperator::new(fixture.data()).expect("operator");
    let solver = PreparedModelSolver::prepare(fixture.data(), options()).expect("prepare");
    let solved = solver.solve(rhs.view()).expect("solve");

    let mut perturbed_firm = solved.coefficients.firm.clone();
    *perturbed_firm.last_mut().expect("last firm") += 1.0e-5;
    let firm_residual = operator
        .full_residual(
            &solved.coefficients.worker,
            &perturbed_firm,
            &solved.coefficients.control,
            rhs.view(),
        )
        .expect("firm residual");
    assert!(firm_residual.firm.last().expect("last equation").abs() > 1.0e-6);
    assert!(firm_residual.relative_norm > options().full_residual_tolerance());

    let mut perturbed_control = solved.coefficients.control.clone();
    *perturbed_control.last_mut().expect("last control") += 1.0e-5;
    let control_residual = operator
        .full_residual(
            &solved.coefficients.worker,
            &solved.coefficients.firm,
            &perturbed_control,
            rhs.view(),
        )
        .expect("control residual");
    assert!(
        control_residual
            .control
            .last()
            .expect("last equation")
            .abs()
            > 1.0e-6
    );
    assert!(control_residual.relative_norm > options().full_residual_tolerance());
}

#[test]
fn zero_incompatible_and_mixed_batches_have_typed_independent_results() {
    let fixture = fixed_fixture(1);
    let truth = coefficients(&fixture, 4);
    let rhs = normal_rhs(&fixture, &truth.0, &truth.1, &truth.2);
    let solver = PreparedModelSolver::prepare(fixture.data(), options()).expect("prepare");
    let zero_worker = vec![0.0; fixture.workers];
    let zero_firm = vec![0.0; fixture.firms];
    let zero_control = vec![0.0; fixture.controls.len()];
    let zero = solver
        .solve(ModelRhs {
            worker: &zero_worker,
            firm: &zero_firm,
            control: &zero_control,
        })
        .expect("zero solve");
    assert_eq!(zero.receipt.pcg.status, ModelPcgStatus::ZeroRhs);
    assert_eq!(zero.residual.relative_norm, 0.0);

    let mut incompatible_worker = rhs.worker.clone();
    incompatible_worker[0] += 1.0e-4;
    let error = solver
        .solve(ModelRhs {
            worker: &incompatible_worker,
            firm: &rhs.firm,
            control: &rhs.control,
        })
        .expect_err("incompatible RHS");
    assert_eq!(error.code, ErrorCode::InvalidInput);

    let mut worker_batch = rhs.worker.clone();
    worker_batch.extend_from_slice(&zero_worker);
    worker_batch.extend(rhs.worker.iter().map(|value| -value));
    let mut firm_batch = rhs.firm.clone();
    firm_batch.extend_from_slice(&zero_firm);
    firm_batch.extend(rhs.firm.iter().map(|value| -value));
    let mut control_batch = rhs.control.clone();
    control_batch.extend_from_slice(&zero_control);
    control_batch.extend(rhs.control.iter().map(|value| -value));
    let wide = solver
        .solve_batch(&worker_batch, &firm_batch, &control_batch, 3, 3)
        .expect("wide batch");
    let narrow = solver
        .solve_batch(&worker_batch, &firm_batch, &control_batch, 3, 1)
        .expect("narrow batch");
    assert_eq!(wide.solution[1].receipt.pcg.status, ModelPcgStatus::ZeroRhs);
    for column in 0..3 {
        assert_eq!(
            wide.solution[column].coefficients.worker,
            narrow.solution[column].coefficients.worker
        );
        assert_eq!(
            wide.solution[column].coefficients.firm,
            narrow.solution[column].coefficients.firm
        );
        assert_eq!(
            wide.solution[column].coefficients.control,
            narrow.solution[column].coefficients.control
        );
        assert_eq!(
            wide.solution[column].receipt.pcg.iterations,
            narrow.solution[column].receipt.pcg.iterations
        );
        assert_eq!(
            wide.solution[column].receipt.pcg.relative_residual,
            narrow.solution[column].receipt.pcg.relative_residual
        );
    }
}

#[test]
fn batched_action_is_bitwise_scalar_and_workspace_overflow_is_typed() {
    let fixture = fixed_fixture(2);
    let operator = ModelOperator::new(fixture.data()).expect("operator");
    let dimension = operator.parameter_count();
    let columns = 3;
    let input = (0..dimension * columns)
        .map(|index| (index as f64 - 5.0) / 7.0)
        .collect::<Vec<_>>();
    let mut output = vec![0.0; input.len()];
    let mut workspace = ModelBatchWorkspace::new(&operator, columns).expect("workspace");
    apply_model_batch(&operator, &input, &mut output, columns, &mut workspace)
        .expect("batch action");
    for column in 0..columns {
        let range = column * dimension..(column + 1) * dimension;
        let mut scalar = vec![0.0; dimension];
        let mut scalar_workspace = ModelWorkspace::new(&operator).expect("scalar workspace");
        operator
            .apply_with_workspace(&input[range.clone()], &mut scalar, &mut scalar_workspace)
            .expect("scalar action");
        assert_eq!(&output[range], scalar.as_slice());
    }
    let error = ModelBatchWorkspaceLayout::checked(usize::MAX, 2, 2)
        .expect_err("workspace multiplication overflow");
    assert_eq!(error.code, ErrorCode::ResourceLimit);
}

#[test]
fn row_firm_and_control_relabeling_preserve_the_solution() {
    let fixture = fixed_fixture(2);
    let truth = coefficients(&fixture, 5);
    let rhs = normal_rhs(&fixture, &truth.0, &truth.1, &truth.2);
    let original = PreparedModelSolver::prepare(fixture.data(), options())
        .expect("original prepare")
        .solve(rhs.view())
        .expect("original solve");

    let row_order = [7, 0, 11, 3, 5, 9, 1, 10, 6, 2, 8, 4];
    let row_fixture = Fixture {
        workers: fixture.workers,
        firms: fixture.firms,
        worker: row_order.iter().map(|&row| fixture.worker[row]).collect(),
        firm: row_order.iter().map(|&row| fixture.firm[row]).collect(),
        weight: row_order.iter().map(|&row| fixture.weight[row]).collect(),
        controls: fixture
            .controls
            .iter()
            .map(|column| row_order.iter().map(|&row| column[row]).collect())
            .collect(),
    };
    let row_solution = PreparedModelSolver::prepare(row_fixture.data(), options())
        .expect("row prepare")
        .solve(rhs.view())
        .expect("row solve");
    assert_close(
        &row_solution.coefficients.worker,
        &original.coefficients.worker,
        1.0e-12,
    );
    assert_close(
        &row_solution.coefficients.firm,
        &original.coefficients.firm,
        1.0e-12,
    );
    assert_close(
        &row_solution.coefficients.control,
        &original.coefficients.control,
        1.0e-12,
    );

    let firm_map = [2_u32, 0, 1];
    let firm_fixture = Fixture {
        firm: fixture
            .firm
            .iter()
            .map(|&firm| firm_map[firm as usize])
            .collect(),
        ..fixture.clone()
    };
    let mut firm_rhs = vec![0.0; fixture.firms];
    for old in 0..fixture.firms {
        firm_rhs[firm_map[old] as usize] = rhs.firm[old];
    }
    let firm_solution = PreparedModelSolver::prepare(firm_fixture.data(), options())
        .expect("firm prepare")
        .solve(ModelRhs {
            worker: &rhs.worker,
            firm: &firm_rhs,
            control: &rhs.control,
        })
        .expect("firm solve");
    for old in 0..fixture.firms {
        assert!(
            (firm_solution.coefficients.firm[firm_map[old] as usize]
                - original.coefficients.firm[old])
                .abs()
                < 1.0e-11
        );
    }

    let control_fixture = Fixture {
        controls: vec![fixture.controls[1].clone(), fixture.controls[0].clone()],
        ..fixture
    };
    let control_rhs = vec![rhs.control[1], rhs.control[0]];
    let control_solution = PreparedModelSolver::prepare(control_fixture.data(), options())
        .expect("control prepare")
        .solve(ModelRhs {
            worker: &rhs.worker,
            firm: &rhs.firm,
            control: &control_rhs,
        })
        .expect("control solve");
    assert!(
        (control_solution.coefficients.control[1] - original.coefficients.control[0]).abs()
            < 1.0e-11
    );
    assert!(
        (control_solution.coefficients.control[0] - original.coefficients.control[1]).abs()
            < 1.0e-11
    );
}

#[test]
fn q_zero_agrees_with_existing_two_way_diagonal_and_cmg_routes() {
    let rows = 12;
    let problem = CanonicalInput::from_validated(
        InputColumns {
            worker: vec![1, 1, 2, 2, 3, 3, 4, 4, 5, 5, 6, 6],
            firm: vec![1, 2, 2, 3, 3, 4, 4, 1, 1, 3, 2, 4],
            deletion: (1..=rows as u64).collect(),
            outcome: vec![
                1.0, -1.0, 2.0, -2.0, 3.0, -3.0, 4.0, -4.0, 2.0, -2.0, 1.0, -1.0,
            ],
            frequency: vec![1; rows],
            target_weight: vec![1.0; rows],
            controls: Vec::new(),
        }
        .validate()
        .expect("validated"),
    )
    .expect("canonical")
    .compress(&vec![true; rows])
    .expect("compressed");
    let route_options = LinearSolverOptions {
        route: LinearSolverRoute::DiagonalPcg,
        pcg: options().pcg,
        full_residual_tolerance: options().full_residual_tolerance(),
        ..LinearSolverOptions::default()
    };
    let two_way = PreparedTwoWaySolver::prepare(&problem, route_options).expect("two-way prepare");
    let (worker_rhs, firm_rhs) = two_way.operator().outcome_rhs().expect("outcome RHS");
    let expected = two_way
        .solve(&worker_rhs, &firm_rhs)
        .expect("two-way solve");
    let no_controls: Vec<Vec<f64>> = Vec::new();
    let generic_data = CanonicalModelData {
        workers: problem.workers(),
        firms: problem.firms(),
        row_worker: &problem.cell_worker,
        row_firm: &problem.cell_firm,
        weight: &problem.cell_weight,
        controls: &no_controls,
    };
    let generic = PreparedModelSolver::prepare(generic_data, options())
        .expect("generic prepare")
        .solve(ModelRhs {
            worker: &worker_rhs,
            firm: &firm_rhs,
            control: &[],
        })
        .expect("generic solve");
    assert_close(
        &generic.coefficients.worker,
        &expected.solution.worker,
        1.0e-12,
    );
    assert_close(&generic.coefficients.firm, &expected.solution.firm, 1.0e-12);

    let cmg_route_options = LinearSolverOptions {
        route: LinearSolverRoute::CmgPcg,
        ..route_options
    };
    let two_way_cmg =
        PreparedTwoWaySolver::prepare(&problem, cmg_route_options).expect("two-way CMG prepare");
    let expected_cmg = two_way_cmg
        .solve(&worker_rhs, &firm_rhs)
        .expect("two-way CMG solve");
    let generic_cmg =
        PreparedModelSolver::prepare_routed(generic_data, routed_options(ModelSolverRoute::Cmg))
            .expect("generic Q=0 CMG prepare");
    let generic_receipt = generic_cmg
        .receipt()
        .cmg
        .as_ref()
        .expect("generic CMG receipt");
    let two_way_receipt = two_way_cmg
        .receipt()
        .cmg
        .as_ref()
        .expect("two-way CMG receipt");
    assert_eq!(generic_receipt.fine_vertices, two_way_receipt.fine_vertices);
    assert_eq!(generic_receipt.fine_edges, two_way_receipt.fine_edges);
    assert_eq!(generic_receipt.levels, two_way_receipt.levels);
    let generic_cmg_solution = generic_cmg
        .solve(ModelRhs {
            worker: &worker_rhs,
            firm: &firm_rhs,
            control: &[],
        })
        .expect("generic Q=0 CMG solve");
    assert_close(
        &generic_cmg_solution.coefficients.worker,
        &expected_cmg.solution.worker,
        2.0e-10,
    );
    assert_close(
        &generic_cmg_solution.coefficients.firm,
        &expected_cmg.solution.firm,
        2.0e-10,
    );
}

struct BreakOnPhase {
    phase: &'static str,
    calls: usize,
    stop: usize,
}

impl InterruptCheck for BreakOnPhase {
    fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
        if phase == self.phase {
            self.calls += 1;
            if self.calls == self.stop {
                return Err(BackendError::new(
                    ErrorCode::UserBreak,
                    phase,
                    "injected generic-model break",
                ));
            }
        }
        Ok(())
    }
}

#[test]
fn generic_cmg_setup_and_block_actions_preserve_user_break() {
    let fixture = fixed_fixture(2);
    let mut setup_break = BreakOnPhase {
        phase: "cmg_graph_build",
        calls: 0,
        stop: 1,
    };
    let setup_error = PreparedModelSolver::prepare_routed_with_interrupt(
        fixture.data(),
        routed_options(ModelSolverRoute::Auto),
        &mut setup_break,
    )
    .expect_err("automatic CMG setup must not swallow UserBreak");
    assert_eq!(setup_error.code, ErrorCode::UserBreak);

    let solver =
        PreparedModelSolver::prepare_routed(fixture.data(), routed_options(ModelSolverRoute::Cmg))
            .expect("CMG prepare");
    let input = vec![0.4, -0.7, 0.3, 0.2, -0.5];
    let mut output = vec![0.0; input.len()];
    let mut apply_break = BreakOnPhase {
        phase: "model_cmg_control_solve",
        calls: 0,
        stop: 1,
    };
    let apply_error = solver
        .apply_preconditioner_with_interrupt(&input, &mut output, &mut apply_break)
        .expect_err("CMG block action must preserve UserBreak");
    assert_eq!(apply_error.code, ErrorCode::UserBreak);
}

#[test]
fn large_setup_and_apply_are_interruptible_inside_bounded_work() {
    let workers = 2_500;
    let mut worker = Vec::with_capacity(workers * 2);
    let mut firm = Vec::with_capacity(workers * 2);
    for index in 0..workers {
        worker.extend([index as u32, index as u32]);
        firm.extend([0_u32, 1_u32]);
    }
    let weight = vec![1.0; worker.len()];
    let controls: Vec<Vec<f64>> = Vec::new();
    let data = CanonicalModelData {
        workers,
        firms: 2,
        row_worker: &worker,
        row_firm: &firm,
        weight: &weight,
        controls: &controls,
    };
    let mut setup_break = BreakOnPhase {
        phase: "model_operator_validate_rows",
        calls: 0,
        stop: 2,
    };
    let error = ModelOperator::new_with_interrupt(data, &mut setup_break)
        .expect_err("setup must poll after 4096 rows");
    assert_eq!(error.code, ErrorCode::UserBreak);

    let operator = ModelOperator::new(data).expect("operator");
    let input = vec![0.5, -0.5];
    let mut output = vec![0.0; 2];
    let mut workspace = ModelWorkspace::new(&operator).expect("workspace");
    let mut apply_break = BreakOnPhase {
        phase: "model_operator_worker_accumulate",
        calls: 0,
        stop: 2,
    };
    let error = operator
        .apply_with_workspace_and_interrupt(&input, &mut output, &mut workspace, &mut apply_break)
        .expect_err("apply must poll after 4096 rows");
    assert_eq!(error.code, ErrorCode::UserBreak);
}

#[test]
fn large_firm_projection_is_interruptible_inside_centering_work() {
    let firms = 5_001;
    let worker = vec![0_u32; firms];
    let firm = (0..firms as u32).collect::<Vec<_>>();
    let weight = vec![1.0; firms];
    let controls: Vec<Vec<f64>> = Vec::new();
    let operator = ModelOperator::new(CanonicalModelData {
        workers: 1,
        firms,
        row_worker: &worker,
        row_firm: &firm,
        weight: &weight,
        controls: &controls,
    })
    .expect("large-F operator");
    let mut values = vec![1.0; firms];
    let mut interrupt = BreakOnPhase {
        phase: "model_operator_project",
        calls: 0,
        stop: 2,
    };
    let error = operator
        .project_parameters_with_interrupt(&mut values, &mut interrupt)
        .expect_err("large-F projection must poll within the validation/sum loop");
    assert_eq!(error.code, ErrorCode::UserBreak);
    assert_eq!(interrupt.calls, 2);
}

#[test]
fn generic_cmg_requires_symmetric_smoothing_and_multilevel_action_is_spd() {
    let fixture = wide_control_fixture(1);
    let mut asymmetric = routed_options(ModelSolverRoute::Cmg);
    asymmetric.cmg.terminal_vertices = 2;
    asymmetric.cmg.pre_sweeps = 1;
    asymmetric.cmg.post_sweeps = 2;
    let error = PreparedModelSolver::prepare_routed(fixture.data(), asymmetric)
        .expect_err("ordinary PCG must reject an asymmetric CMG V-cycle");
    assert_eq!(error.code, ErrorCode::CmgSetupFailed);

    let mut symmetric = asymmetric;
    symmetric.cmg.pre_sweeps = 2;
    let solver = PreparedModelSolver::prepare_routed(fixture.data(), symmetric)
        .expect("equal-sweep multilevel CMG prepare");
    let receipt = solver.receipt().cmg.as_ref().expect("CMG receipt");
    assert!(receipt.levels > 1, "test fixture must exercise a hierarchy");

    let dimension = fixture.firms + fixture.controls.len();
    let mut left = (0..dimension)
        .map(|index| ((index * 13 + 5) % 23) as f64 / 11.0 - 0.8)
        .collect::<Vec<_>>();
    let mut right = (0..dimension)
        .map(|index| ((index * 17 + 3) % 29) as f64 / 13.0 - 0.9)
        .collect::<Vec<_>>();
    for values in [&mut left, &mut right] {
        let mean = values[..fixture.firms].iter().sum::<f64>() / fixture.firms as f64;
        for value in &mut values[..fixture.firms] {
            *value -= mean;
        }
    }
    let mut action_left = vec![0.0; dimension];
    let mut action_right = vec![0.0; dimension];
    solver
        .apply_preconditioner(&left, &mut action_left)
        .expect("left multilevel CMG action");
    solver
        .apply_preconditioner(&right, &mut action_right)
        .expect("right multilevel CMG action");
    let left_cross = stable_dot(&left, &action_right);
    let right_cross = stable_dot(&action_left, &right);
    assert!((left_cross - right_cross).abs() < 3.0e-10 * (1.0 + left_cross.abs()));
    assert!(stable_dot(&left, &action_left) > 0.0);
    assert!(stable_dot(&right, &action_right) > 0.0);
}

#[test]
fn model_cmg_control_cross_passes_poll_inside_large_firm_dimension() {
    let firms = 4_097;
    let rows = 2 * firms;
    let mut worker = Vec::with_capacity(rows);
    let mut firm = Vec::with_capacity(rows);
    let mut control = Vec::with_capacity(rows);
    for worker_index in 0..2 {
        for firm_index in 0..firms {
            worker.push(worker_index);
            firm.push(firm_index as u32);
            control.push(if worker_index == 0 && firm_index % 2 == 0 {
                1.0
            } else {
                0.0
            });
        }
    }
    let weight = vec![1.0; rows];
    let controls = vec![control];
    let data = CanonicalModelData {
        workers: 2,
        firms,
        row_worker: &worker,
        row_firm: &firm,
        weight: &weight,
        controls: &controls,
    };
    let solver = PreparedModelSolver::prepare_routed(data, routed_options(ModelSolverRoute::Cmg))
        .expect("large-F Q=1 CMG prepare");
    let mut input = vec![0.0; firms + 1];
    input[firms] = 1.0;
    let mut output = vec![0.0; input.len()];

    for phase in ["model_cmg_control_rhs", "model_cmg_second_firm_rhs"] {
        let mut interrupt = BreakOnPhase {
            phase,
            calls: 0,
            stop: 2,
        };
        let error = solver
            .apply_preconditioner_with_interrupt(&input, &mut output, &mut interrupt)
            .expect_err("large F-by-Q pass must poll beyond its first chunk");
        assert_eq!(error.code, ErrorCode::UserBreak, "phase {phase}");
        assert_eq!(interrupt.calls, 2, "phase {phase}");
    }
}

#[test]
fn model_cmg_group_item_copy_polls_inside_large_groups() {
    let firms = 4_097;
    let worker = vec![0_u32; firms];
    let firm = (0..firms as u32).collect::<Vec<_>>();
    let weight = vec![1.0; firms];
    let no_controls = Vec::new();
    let worker_group_data = CanonicalModelData {
        workers: 1,
        firms,
        row_worker: &worker,
        row_firm: &firm,
        weight: &weight,
        controls: &no_controls,
    };
    let mut worker_break = BreakOnPhase {
        phase: "model_cmg_worker_items",
        calls: 0,
        stop: 2,
    };
    let error = PreparedModelSolver::prepare_routed_with_interrupt(
        worker_group_data,
        routed_options(ModelSolverRoute::Cmg),
        &mut worker_break,
    )
    .expect_err("large worker group item copy must poll beyond its first chunk");
    assert_eq!(error.code, ErrorCode::UserBreak);
    assert_eq!(worker_break.calls, 2);

    let workers = 4_097;
    let mut firm_group_worker = (0..workers as u32).collect::<Vec<_>>();
    let mut firm_group_firm = vec![0_u32; workers];
    firm_group_worker.push(0);
    firm_group_firm.push(1);
    let firm_group_weight = vec![1.0; firm_group_worker.len()];
    let firm_group_data = CanonicalModelData {
        workers,
        firms: 2,
        row_worker: &firm_group_worker,
        row_firm: &firm_group_firm,
        weight: &firm_group_weight,
        controls: &no_controls,
    };
    let mut firm_break = BreakOnPhase {
        phase: "model_cmg_firm_items",
        calls: 0,
        stop: 2,
    };
    let error = PreparedModelSolver::prepare_routed_with_interrupt(
        firm_group_data,
        routed_options(ModelSolverRoute::Cmg),
        &mut firm_break,
    )
    .expect_err("large firm group item copy must poll beyond its first chunk");
    assert_eq!(error.code, ErrorCode::UserBreak);
    assert_eq!(firm_break.calls, 2);
}
