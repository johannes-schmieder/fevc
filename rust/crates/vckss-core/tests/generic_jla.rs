// SPDX-License-Identifier: GPL-3.0-only

use std::mem::size_of;

use vckss_core::batch_plan::BatchRequest;
use vckss_core::cmg::CmgOptions;
use vckss_core::component_inference::{
    prepare_grouped_oracle_component_inference, prepare_grouped_structured_component_inference,
    prepare_oracle_component_inference, prepare_structured_component_inference,
    ComponentInferenceOptions, ComponentInferenceUnit, ComponentReferenceDistribution,
    ComponentVarianceSource, PRIMITIVE_TARGETS,
};
use vckss_core::engine::{run_jla_no_controls_planned, JlaEngineOptions, PlannedJlaEngineOptions};
use vckss_core::error::{BackendError, ErrorCode, Result};
use vckss_core::exact_estimator::{
    run_exact_estimator, run_exact_stayer_hybrid, ExactEstimatorOptions,
};
use vckss_core::generic_batch::ModelPcgStatus;
use vckss_core::generic_jla::{
    run_generic_jla, run_generic_jla_routed,
    run_generic_jla_routed_with_attachments_and_hybrid_interrupt,
    run_generic_jla_routed_with_interrupt,
    run_generic_jla_routed_with_projection_and_hybrid_interrupt, run_generic_jla_with_interrupt,
    GenericJlaExecutionOptions, GenericJlaOptions, GenericJlaResult, GenericJlaRhsPhase,
    GenericJlaRhsReceipt, GenericJlaRhsSide, GENERIC_JLA_AUTO_FIRM_THRESHOLD_V1,
    GENERIC_JLA_AUTO_PLANNED_RHS_THRESHOLD_V1,
};
use vckss_core::interrupt::{InterruptCheck, NeverInterrupt};
use vckss_core::jla::JlaPlan;
use vckss_core::krylov::PcgOptions;
use vckss_core::model_solver::{
    ControlProjectionReceipt, ModelRoutingOptions, ModelSolverOptions, ModelSolverRoute,
};
use vckss_core::problem::{CanonicalInput, CompressedProblem};
use vckss_core::projection::{prepare_projection, ProjectionEffect, ProjectionWeight};
use vckss_core::solver::{LinearSolverOptions, LinearSolverRoute};
use vckss_core::stayer_hybrid::{prepare_exact_stayer_hybrid, StayerAugmentationInput};
use vckss_core::structured_variance::StructuredVarianceOptions;
use vckss_core::types::{DeletionMode, InputColumns, NuisanceMode};
use vckss_core::wall_plan::WallAdvisoryStatus;

fn fixture(controls: bool) -> CompressedProblem {
    let mut worker = Vec::new();
    let mut firm = Vec::new();
    let mut deletion = Vec::new();
    let mut outcome = Vec::new();
    let mut frequency = Vec::new();
    let mut target_weight = Vec::new();
    let mut first_control = Vec::new();
    let mut second_control = Vec::new();
    for worker_index in 0..3_u64 {
        for firm_index in 0..3_u64 {
            for replicate in 0..2_u64 {
                let row = worker.len() as u64;
                worker.push(10 + 10 * worker_index);
                firm.push(101 + 100 * firm_index);
                deletion.push(1_000 + row);
                outcome.push(
                    0.7 * worker_index as f64 - 0.45 * firm_index as f64
                        + 0.3 * replicate as f64
                        + ((row * 7) % 5) as f64 / 11.0,
                );
                frequency.push(1 + (row % 3));
                target_weight.push(0.5 + ((row * 5) % 7) as f64 / 3.0);
                first_control.push(
                    (worker_index as f64 - firm_index as f64) * (1.0 + replicate as f64)
                        + ((row * 3) % 4) as f64 / 7.0,
                );
                second_control.push(
                    (worker_index + firm_index) as f64 * (0.5 + replicate as f64)
                        + ((row * 11) % 6) as f64 / 13.0,
                );
            }
        }
    }
    let controls = if controls {
        vec![first_control, second_control]
    } else {
        Vec::new()
    };
    CanonicalInput::from_validated(
        InputColumns {
            worker,
            firm,
            deletion,
            outcome,
            frequency,
            target_weight,
            controls,
        }
        .validate()
        .expect("fixture validates"),
    )
    .expect("fixture canonicalizes")
    .compress(&[true; 18])
    .expect("fixture compresses")
}

fn component_inference_fixture(controls: bool) -> CompressedProblem {
    let mut worker = Vec::new();
    let mut firm = Vec::new();
    let mut deletion = Vec::new();
    let mut outcome = Vec::new();
    let mut target_weight = Vec::new();
    let mut first_control = Vec::new();
    let mut second_control = Vec::new();
    for worker_index in 0..4_u64 {
        for firm_index in 0..4_u64 {
            for replicate in 0..2_u64 {
                let row = worker.len() as u64;
                worker.push(10 + worker_index);
                firm.push(100 + firm_index);
                deletion.push(1_000 + row);
                outcome.push(
                    0.45 * worker_index as f64 - 0.28 * firm_index as f64
                        + 0.17 * replicate as f64
                        + ((row * 7) % 13) as f64 / 17.0,
                );
                target_weight.push(0.8 + ((row * 5) % 11) as f64 / 13.0);
                first_control.push(
                    (worker_index as f64 - 0.3 * firm_index as f64) * (0.5 + replicate as f64)
                        + (row % 5) as f64 / 19.0,
                );
                second_control.push(
                    (0.2 * worker_index as f64 + firm_index as f64)
                        * (1.5 - 0.25 * replicate as f64)
                        + (row % 7) as f64 / 23.0,
                );
            }
        }
    }
    let rows = worker.len();
    CanonicalInput::from_validated(
        InputColumns {
            worker,
            firm,
            deletion,
            outcome,
            frequency: vec![1; rows],
            target_weight,
            controls: if controls {
                vec![first_control, second_control]
            } else {
                Vec::new()
            },
        }
        .validate()
        .expect("component-inference fixture validates"),
    )
    .expect("component-inference fixture canonicalizes")
    .compress(&vec![true; rows])
    .expect("component-inference fixture compresses")
}

fn structured_component_inference_fixture() -> CompressedProblem {
    let mut worker = Vec::new();
    let mut firm = Vec::new();
    let mut deletion = Vec::new();
    let mut outcome = Vec::new();
    let mut target_weight = Vec::new();
    let mut first_control = Vec::new();
    let mut second_control = Vec::new();
    for worker_index in 0..9_u64 {
        for firm_index in 0..9_u64 {
            for replicate in 0..2_u64 {
                let row = worker.len() as u64;
                worker.push(10 + worker_index);
                firm.push(100 + firm_index);
                deletion.push(10_000 + row);
                let scale = 0.04 + 0.008 * worker_index as f64 + 0.005 * firm_index as f64;
                let shock = (((row * 37 + 11) % 101) as f64 / 50.0 - 1.0) * scale;
                outcome.push(
                    0.31 * worker_index as f64 - 0.23 * firm_index as f64
                        + 0.09 * replicate as f64
                        + shock,
                );
                target_weight.push(0.75 + ((row * 13) % 29) as f64 / 31.0);
                first_control.push(
                    (worker_index as f64 - 0.4 * firm_index as f64)
                        * (0.6 + 0.3 * replicate as f64)
                        + (row % 11) as f64 / 37.0,
                );
                second_control.push(
                    (0.25 * worker_index as f64 + firm_index as f64)
                        * (1.2 - 0.2 * replicate as f64)
                        + (row % 17) as f64 / 41.0,
                );
            }
        }
    }
    let rows = worker.len();
    CanonicalInput::from_validated(
        InputColumns {
            worker,
            firm,
            deletion,
            outcome,
            frequency: vec![1; rows],
            target_weight,
            controls: vec![first_control, second_control],
        }
        .validate()
        .expect("structured component fixture validates"),
    )
    .expect("structured component fixture canonicalizes")
    .compress(&vec![true; rows])
    .expect("structured component fixture compresses")
}

fn grouped_component_inference_fixture(dimension: u64) -> CompressedProblem {
    let mut worker = Vec::new();
    let mut firm = Vec::new();
    let mut deletion = Vec::new();
    let mut outcome = Vec::new();
    let mut frequency = Vec::new();
    let mut target_weight = Vec::new();
    let mut first_control = Vec::new();
    let mut second_control = Vec::new();
    for worker_index in 0..dimension {
        for firm_index in 0..dimension {
            let match_id = 50_000 + worker_index * dimension + firm_index;
            for replicate in 0..2_u64 {
                let row = worker.len() as u64;
                worker.push(100 + worker_index);
                firm.push(1_000 + firm_index);
                deletion.push(match_id);
                frequency.push(1 + (row + replicate) % 3);
                target_weight.push(0.6 + ((row * 17 + 3) % 23) as f64 / 19.0);
                first_control.push(
                    0.2 * worker_index as f64 - 0.13 * firm_index as f64
                        + 0.31 * replicate as f64
                        + (row % 7) as f64 / 29.0,
                );
                second_control.push(
                    -0.11 * worker_index as f64 + 0.18 * firm_index as f64
                        - 0.27 * replicate as f64
                        + (row % 11) as f64 / 31.0,
                );
                let first = *first_control.last().expect("first control appended");
                let second = *second_control.last().expect("second control appended");
                let scale = 1.4 + 0.05 * worker_index as f64 + 0.036 * firm_index as f64;
                let shock = (((row * 37 + 11) % 101) as f64 / 50.0 - 1.0) * scale;
                outcome.push(
                    1.4 * first - 0.9 * second + worker_index as f64 - 0.8 * firm_index as f64
                        + shock,
                );
            }
        }
    }
    let rows = worker.len();
    CanonicalInput::from_validated(
        InputColumns {
            worker,
            firm,
            deletion,
            outcome,
            frequency,
            target_weight,
            controls: vec![first_control, second_control],
        }
        .validate()
        .expect("grouped component fixture validates"),
    )
    .expect("grouped component fixture canonicalizes")
    .compress(&vec![true; rows])
    .expect("grouped component fixture compresses")
}

fn bridge_match_component_inference_fixture() -> CompressedProblem {
    let edges = [
        (0_u64, 0_u64),
        (0, 1),
        (1, 0),
        (1, 1),
        (1, 2),
        (2, 2),
        (2, 3),
        (3, 2),
        (3, 3),
    ];
    let rows = edges.len();
    CanonicalInput::from_validated(
        InputColumns {
            worker: edges.iter().map(|&(worker, _)| 100 + worker).collect(),
            firm: edges.iter().map(|&(_, firm)| 1_000 + firm).collect(),
            deletion: (0..rows).map(|row| 70_000 + row as u64).collect(),
            outcome: (0..rows)
                .map(|row| ((row * 17 + 3) % 29) as f64 / 11.0)
                .collect(),
            frequency: vec![1; rows],
            target_weight: vec![1.0; rows],
            controls: Vec::new(),
        }
        .validate()
        .expect("bridge fixture validates"),
    )
    .expect("bridge fixture canonicalizes")
    .compress(&vec![true; rows])
    .expect("bridge fixture compresses")
}

fn component_inference_stayer_fixture() -> CompressedProblem {
    let worker = vec![10, 10, 20, 20, 20, 20];
    let firm = vec![100, 100, 100, 100, 200, 200];
    let rows = worker.len();
    CanonicalInput::from_validated(
        InputColumns {
            worker,
            firm,
            deletion: (0..rows).map(|row| 1_000 + row as u64).collect(),
            outcome: vec![0.2, 0.7, -0.1, 0.4, 0.8, 1.3],
            frequency: vec![1; rows],
            target_weight: vec![1.0; rows],
            controls: Vec::new(),
        }
        .validate()
        .expect("stayer fixture validates"),
    )
    .expect("stayer fixture canonicalizes")
    .compress(&vec![true; rows])
    .expect("stayer fixture compresses")
}

fn dense_multiply(
    left: &[f64],
    left_rows: usize,
    inner: usize,
    right: &[f64],
    right_columns: usize,
) -> Vec<f64> {
    let mut output = vec![0.0; left_rows * right_columns];
    for row in 0..left_rows {
        for column in 0..right_columns {
            for k in 0..inner {
                output[row * right_columns + column] +=
                    left[row * inner + k] * right[k * right_columns + column];
            }
        }
    }
    output
}

fn dense_transpose(value: &[f64], rows: usize, columns: usize) -> Vec<f64> {
    let mut output = vec![0.0; value.len()];
    for row in 0..rows {
        for column in 0..columns {
            output[column * rows + row] = value[row * columns + column];
        }
    }
    output
}

fn dense_inverse(value: &[f64], dimension: usize) -> Vec<f64> {
    let mut augmented = vec![0.0; dimension * 2 * dimension];
    for row in 0..dimension {
        for column in 0..dimension {
            augmented[row * 2 * dimension + column] = value[row * dimension + column];
        }
        augmented[row * 2 * dimension + dimension + row] = 1.0;
    }
    for pivot in 0..dimension {
        let selected = (pivot..dimension)
            .max_by(|left, right| {
                augmented[*left * 2 * dimension + pivot]
                    .abs()
                    .partial_cmp(&augmented[*right * 2 * dimension + pivot].abs())
                    .expect("finite dense pivot")
            })
            .expect("dense pivot exists");
        assert!(augmented[selected * 2 * dimension + pivot].abs() > 1.0e-12);
        for column in 0..2 * dimension {
            augmented.swap(
                pivot * 2 * dimension + column,
                selected * 2 * dimension + column,
            );
        }
        let scale = augmented[pivot * 2 * dimension + pivot];
        for column in 0..2 * dimension {
            augmented[pivot * 2 * dimension + column] /= scale;
        }
        for row in 0..dimension {
            if row == pivot {
                continue;
            }
            let multiplier = augmented[row * 2 * dimension + pivot];
            for column in 0..2 * dimension {
                augmented[row * 2 * dimension + column] -=
                    multiplier * augmented[pivot * 2 * dimension + column];
            }
        }
    }
    let mut inverse = vec![0.0; dimension * dimension];
    for row in 0..dimension {
        inverse[row * dimension..(row + 1) * dimension].copy_from_slice(
            &augmented[row * 2 * dimension + dimension..(row + 1) * 2 * dimension],
        );
    }
    inverse
}

fn dense_two_mode_spectrum(matrix: &[f64], dimension: usize) -> (f64, f64, f64) {
    let apply = |vector: &[f64]| {
        (0..dimension)
            .map(|row| {
                (0..dimension)
                    .map(|column| matrix[row * dimension + column] * vector[column])
                    .sum()
            })
            .collect::<Vec<f64>>()
    };
    let normalize = |mut vector: Vec<f64>| {
        let norm = vector.iter().map(|value| value * value).sum::<f64>().sqrt();
        for value in &mut vector {
            *value /= norm;
        }
        vector
    };
    let mut first = normalize(
        (0..dimension)
            .map(|row| ((row + 1) as f64 * 0.731).sin())
            .collect(),
    );
    for _ in 0..512 {
        first = normalize(apply(&apply(&first)));
    }
    let mut second = (0..dimension)
        .map(|row| ((row + 1) as f64 * 1.173).cos())
        .collect::<Vec<_>>();
    for _ in 0..512 {
        second = apply(&apply(&second));
        let projection = second
            .iter()
            .zip(&first)
            .map(|(left, right)| left * right)
            .sum::<f64>();
        for row in 0..dimension {
            second[row] -= projection * first[row];
        }
        second = normalize(second);
    }
    let action_first = apply(&first);
    let action_second = apply(&second);
    let first_rayleigh = first
        .iter()
        .zip(&action_first)
        .map(|(left, right)| left * right)
        .sum::<f64>();
    let cross = 0.5
        * (first
            .iter()
            .zip(&action_second)
            .map(|(left, right)| left * right)
            .sum::<f64>()
            + second
                .iter()
                .zip(&action_first)
                .map(|(left, right)| left * right)
                .sum::<f64>());
    let second_rayleigh = second
        .iter()
        .zip(&action_second)
        .map(|(left, right)| left * right)
        .sum::<f64>();
    let center = 0.5 * (first_rayleigh + second_rayleigh);
    let radius = (0.25 * (first_rayleigh - second_rayleigh).powi(2) + cross * cross).sqrt();
    let mut eigenvalues = [center + radius, center - radius];
    eigenvalues.sort_by(|left, right| right.abs().partial_cmp(&left.abs()).unwrap());
    let trace_square = (0..dimension)
        .flat_map(|row| {
            (0..dimension).map(move |column| {
                matrix[row * dimension + column] * matrix[column * dimension + row]
            })
        })
        .sum();
    (eigenvalues[0], eigenvalues[1], trace_square)
}

fn dense_component_spectra(problem: &CompressedProblem) -> [(f64, f64, f64); 4] {
    assert!(problem.controls.is_empty());
    let rows = problem.outcome.len();
    let workers = problem.workers();
    let firms = problem.firms();
    let parameters = workers + firms - 1;
    let mut design = vec![0.0; rows * parameters];
    for row in 0..rows {
        design[row * parameters + problem.row_worker[row] as usize] = 1.0;
        let firm = problem.row_firm[row] as usize;
        if firm + 1 < firms {
            design[row * parameters + workers + firm] = 1.0;
        }
    }
    let transpose = dense_transpose(&design, rows, parameters);
    let information = dense_multiply(&transpose, parameters, rows, &design, parameters);
    let inverse = dense_inverse(&information, parameters);
    let mut worker_mean = vec![0.0; parameters];
    let mut firm_mean = vec![0.0; parameters];
    for cell in 0..problem.cells() {
        let mass = problem.cell_target_sum[cell] / problem.target_total;
        worker_mean[problem.cell_worker[cell] as usize] += mass;
        let firm = problem.cell_firm[cell] as usize;
        if firm + 1 < firms {
            firm_mean[workers + firm] += mass;
        }
    }
    let mut target = [
        vec![0.0; parameters * parameters],
        vec![0.0; parameters * parameters],
        vec![0.0; parameters * parameters],
        vec![0.0; parameters * parameters],
    ];
    for cell in 0..problem.cells() {
        let mass = problem.cell_target_sum[cell] / problem.target_total;
        let mut worker = worker_mean.iter().map(|value| -value).collect::<Vec<_>>();
        let mut firm = firm_mean.iter().map(|value| -value).collect::<Vec<_>>();
        worker[problem.cell_worker[cell] as usize] += 1.0;
        let firm_index = problem.cell_firm[cell] as usize;
        if firm_index + 1 < firms {
            firm[workers + firm_index] += 1.0;
        }
        for row in 0..parameters {
            for column in 0..parameters {
                target[0][row * parameters + column] += mass * worker[row] * worker[column];
                target[1][row * parameters + column] += mass * firm[row] * firm[column];
                target[2][row * parameters + column] +=
                    0.5 * mass * (worker[row] * firm[column] + firm[row] * worker[column]);
            }
        }
    }
    for index in 0..parameters * parameters {
        target[3][index] = target[0][index] + target[1][index] + 2.0 * target[2][index];
    }
    core::array::from_fn(|target_index| {
        let inverse_target = dense_multiply(
            &inverse,
            parameters,
            parameters,
            &target[target_index],
            parameters,
        );
        let inverse_target_inverse = dense_multiply(
            &inverse_target,
            parameters,
            parameters,
            &inverse,
            parameters,
        );
        let observation_kernel = dense_multiply(
            &dense_multiply(
                &design,
                rows,
                parameters,
                &inverse_target_inverse,
                parameters,
            ),
            rows,
            parameters,
            &transpose,
            rows,
        );
        dense_two_mode_spectrum(&observation_kernel, rows)
    })
}

fn dense_grouped_component_kernels(
    problem: &CompressedProblem,
    target_diagonal: &[Vec<f64>; PRIMITIVE_TARGETS],
    maker_inverse: &[f64],
) -> [Vec<f64>; PRIMITIVE_TARGETS] {
    let groups = problem.deletion_units();
    assert_eq!(groups, problem.cells());
    assert_eq!(maker_inverse.len(), groups);
    assert!(target_diagonal.iter().all(|value| value.len() == groups));
    assert!(problem
        .row_deletion
        .iter()
        .zip(&problem.row_cell)
        .all(|(deletion, cell)| deletion == cell));
    let workers = problem.workers();
    let firms = problem.firms();
    let parameters = workers + firms - 1;
    let mut design = vec![0.0; groups * parameters];
    for group in 0..groups {
        let root_mass = problem.cell_weight[group].sqrt();
        design[group * parameters + problem.cell_worker[group] as usize] = root_mass;
        let firm = problem.cell_firm[group] as usize;
        if firm + 1 < firms {
            design[group * parameters + workers + firm] = root_mass;
        }
    }
    let transpose = dense_transpose(&design, groups, parameters);
    let information = dense_multiply(&transpose, parameters, groups, &design, parameters);
    let inverse = dense_inverse(&information, parameters);
    let projection = dense_multiply(
        &dense_multiply(&design, groups, parameters, &inverse, parameters),
        groups,
        parameters,
        &transpose,
        groups,
    );
    let mut maker = projection.iter().map(|value| -*value).collect::<Vec<_>>();
    for group in 0..groups {
        maker[group * groups + group] += 1.0;
    }

    let mut worker_mean = vec![0.0; parameters];
    let mut firm_mean = vec![0.0; parameters];
    for cell in 0..problem.cells() {
        let mass = problem.cell_target_sum[cell] / problem.target_total;
        worker_mean[problem.cell_worker[cell] as usize] += mass;
        let firm = problem.cell_firm[cell] as usize;
        if firm + 1 < firms {
            firm_mean[workers + firm] += mass;
        }
    }
    let mut target: [Vec<f64>; PRIMITIVE_TARGETS] =
        core::array::from_fn(|_| vec![0.0; parameters * parameters]);
    for cell in 0..problem.cells() {
        let mass = problem.cell_target_sum[cell] / problem.target_total;
        let mut worker = worker_mean.iter().map(|value| -value).collect::<Vec<_>>();
        let mut firm = firm_mean.iter().map(|value| -value).collect::<Vec<_>>();
        worker[problem.cell_worker[cell] as usize] += 1.0;
        let firm_index = problem.cell_firm[cell] as usize;
        if firm_index + 1 < firms {
            firm[workers + firm_index] += 1.0;
        }
        for row in 0..parameters {
            for column in 0..parameters {
                target[0][row * parameters + column] += mass * worker[row] * worker[column];
                target[1][row * parameters + column] += mass * firm[row] * firm[column];
                target[2][row * parameters + column] +=
                    0.5 * mass * (worker[row] * firm[column] + firm[row] * worker[column]);
            }
        }
    }
    core::array::from_fn(|target_index| {
        let inverse_target = dense_multiply(
            &inverse,
            parameters,
            parameters,
            &target[target_index],
            parameters,
        );
        let middle = dense_multiply(
            &inverse_target,
            parameters,
            parameters,
            &inverse,
            parameters,
        );
        let mut kernel = dense_multiply(
            &dense_multiply(&design, groups, parameters, &middle, parameters),
            groups,
            parameters,
            &transpose,
            groups,
        );
        let ratio = (0..groups)
            .map(|group| target_diagonal[target_index][group] * maker_inverse[group])
            .collect::<Vec<_>>();
        for row in 0..groups {
            for column in 0..groups {
                kernel[row * groups + column] -= 0.5
                    * (ratio[row] * maker[row * groups + column]
                        + maker[row * groups + column] * ratio[column]);
            }
        }
        kernel
    })
}

fn maximum_control_fixture() -> CompressedProblem {
    let workers = 8_usize;
    let firms = 8_usize;
    let cells = workers * firms;
    let rows = 2 * cells;
    let mut worker = Vec::with_capacity(rows);
    let mut firm = Vec::with_capacity(rows);
    let mut deletion = Vec::with_capacity(rows);
    let mut outcome = Vec::with_capacity(rows);
    let mut frequency = Vec::with_capacity(rows);
    let mut target_weight = Vec::with_capacity(rows);
    let mut controls = (0..32)
        .map(|_| Vec::with_capacity(rows))
        .collect::<Vec<_>>();
    for worker_index in 0..workers {
        for firm_index in 0..firms {
            let cell = worker_index * firms + firm_index;
            for replicate in 0..2_usize {
                let sign = if replicate == 0 { 1.0 } else { -1.0 };
                worker.push(10 + worker_index as u64);
                firm.push(100 + firm_index as u64);
                deletion.push(1_000 + cell as u64);
                outcome.push(
                    0.3 * worker_index as f64 - 0.2 * firm_index as f64
                        + sign * (0.25 + (cell % 7) as f64 / 19.0),
                );
                frequency.push(1);
                target_weight.push(0.5 + ((cell * 3 + replicate) % 11) as f64 / 7.0);
                for (control, column) in controls.iter_mut().enumerate() {
                    let hadamard = if (cell & control).count_ones() % 2 == 0 {
                        1.0
                    } else {
                        -1.0
                    };
                    column.push(sign * hadamard);
                }
            }
        }
    }
    CanonicalInput::from_validated(
        InputColumns {
            worker,
            firm,
            deletion,
            outcome,
            frequency,
            target_weight,
            controls,
        }
        .validate()
        .expect("maximum-control fixture validates"),
    )
    .expect("maximum-control fixture canonicalizes")
    .compress(&vec![true; rows])
    .expect("maximum-control fixture compresses")
}

fn rebuild(
    problem: &CompressedProblem,
    split_row: Option<usize>,
    split_deletion: bool,
    reverse: bool,
) -> CompressedProblem {
    let mut worker = Vec::new();
    let mut firm = Vec::new();
    let mut deletion = Vec::new();
    let mut outcome = Vec::new();
    let mut frequency = Vec::new();
    let mut target_weight = Vec::new();
    let mut controls = vec![Vec::new(); problem.controls.len()];
    let mut order = (0..problem.outcome.len()).collect::<Vec<_>>();
    if reverse {
        order.reverse();
    }
    for row in order {
        let pieces = if split_row == Some(row) { 2 } else { 1 };
        for piece in 0..pieces {
            let piece_frequency = if pieces == 1 {
                problem.frequency[row]
            } else if piece == 0 {
                1
            } else {
                problem.frequency[row] - 1
            };
            assert!(
                piece_frequency > 0,
                "split row must have frequency above one"
            );
            worker.push(u64::from(problem.row_worker[row]) + 1);
            firm.push(u64::from(problem.row_firm[row]) + 1);
            let deletion_id = if split_deletion && pieces == 2 && piece == 1 {
                10_000 + row as u64
            } else {
                u64::from(problem.row_deletion[row]) + 1
            };
            deletion.push(deletion_id);
            outcome.push(problem.outcome[row]);
            frequency.push(piece_frequency);
            target_weight.push(if pieces == 1 {
                problem.target_weight[row]
            } else {
                problem.target_weight[row] * piece_frequency as f64 / problem.frequency[row] as f64
            });
            for (target, column) in controls.iter_mut().zip(&problem.controls) {
                target.push(column[row]);
            }
        }
    }
    let retained = worker.len();
    CanonicalInput::from_validated(
        InputColumns {
            worker,
            firm,
            deletion,
            outcome,
            frequency,
            target_weight,
            controls,
        }
        .validate()
        .expect("rebuilt fixture validates"),
    )
    .expect("rebuilt fixture canonicalizes")
    .compress(&vec![true; retained])
    .expect("rebuilt fixture compresses")
}

fn rebuild_order_preserving_relabel(
    problem: &CompressedProblem,
    reverse: bool,
) -> CompressedProblem {
    let mut worker = Vec::new();
    let mut firm = Vec::new();
    let mut deletion = Vec::new();
    let mut outcome = Vec::new();
    let mut frequency = Vec::new();
    let mut target_weight = Vec::new();
    let mut controls = vec![Vec::new(); problem.controls.len()];
    let mut order = (0..problem.outcome.len()).collect::<Vec<_>>();
    if reverse {
        order.reverse();
    }
    for row in order {
        worker.push(10_000 + 17 * u64::from(problem.row_worker[row]));
        firm.push(20_000 + 19 * u64::from(problem.row_firm[row]));
        deletion.push(30_000 + 23 * u64::from(problem.row_deletion[row]));
        outcome.push(problem.outcome[row]);
        frequency.push(problem.frequency[row]);
        target_weight.push(problem.target_weight[row]);
        for (target, column) in controls.iter_mut().zip(&problem.controls) {
            target.push(column[row]);
        }
    }
    let retained = worker.len();
    CanonicalInput::from_validated(
        InputColumns {
            worker,
            firm,
            deletion,
            outcome,
            frequency,
            target_weight,
            controls,
        }
        .validate()
        .expect("relabelled fixture validates"),
    )
    .expect("relabelled fixture canonicalizes")
    .compress(&vec![true; retained])
    .expect("relabelled fixture compresses")
}

fn transformed_control_fixture(problem: &CompressedProblem) -> CompressedProblem {
    assert_eq!(problem.controls.len(), 2);
    let mut worker = Vec::new();
    let mut firm = Vec::new();
    let mut deletion = Vec::new();
    let mut outcome = Vec::new();
    let mut frequency = Vec::new();
    let mut target_weight = Vec::new();
    let mut first = Vec::new();
    let mut second = Vec::new();
    for row in 0..problem.outcome.len() {
        worker.push(1 + u64::from(problem.row_worker[row]));
        firm.push(1 + u64::from(problem.row_firm[row]));
        deletion.push(1 + u64::from(problem.row_deletion[row]));
        outcome.push(problem.outcome[row]);
        frequency.push(problem.frequency[row]);
        target_weight.push(problem.target_weight[row]);
        let x = problem.controls[0][row];
        let z = problem.controls[1][row];
        first.push(x + 0.25 * z);
        second.push(-0.5 * x + 1.5 * z);
    }
    CanonicalInput::from_validated(
        InputColumns {
            worker,
            firm,
            deletion,
            outcome,
            frequency,
            target_weight,
            controls: vec![first, second],
        }
        .validate()
        .expect("transformed fixture validates"),
    )
    .expect("transformed fixture canonicalizes")
    .compress(&vec![true; problem.outcome.len()])
    .expect("transformed fixture compresses")
}

fn tied_control_fixture(reverse: bool, transform: bool) -> CompressedProblem {
    let mut rows = Vec::new();
    for worker in 0..4_u64 {
        for firm in 0..4_u64 {
            let repetitions = if worker == 0 && firm == 0 { 3 } else { 2 };
            for repetition in 0..repetitions {
                let serial = rows.len() as u64;
                let tied = worker == 0 && firm == 0;
                let (outcome, deletion, first, second) = if tied {
                    let controls = [(-2.0, 0.3), (0.4, 1.7), (2.1, -0.8)];
                    let (first, second) = controls[repetition];
                    (0.25, 900_u64, first, second)
                } else {
                    (
                        0.7 * worker as f64 - 0.2 * firm as f64
                            + repetition as f64 / 7.0
                            + (serial % 5) as f64 / 13.0,
                        1_000 + serial,
                        (worker as f64 - 0.4 * firm as f64) * (repetition as f64 + 1.0)
                            + (serial % 7) as f64 / 17.0,
                        (0.3 * worker as f64 + firm as f64) * (repetition as f64 + 0.5)
                            + (serial % 11) as f64 / 19.0,
                    )
                };
                let (first, second) = if transform {
                    (first + 0.25 * second, -0.5 * first + 1.5 * second)
                } else {
                    (first, second)
                };
                rows.push((
                    10 + worker,
                    100 + firm,
                    deletion,
                    outcome,
                    1_u64,
                    1.0_f64,
                    first,
                    second,
                ));
            }
        }
    }
    if reverse {
        rows.reverse();
    }
    let retained = rows.len();
    CanonicalInput::from_validated(
        InputColumns {
            worker: rows.iter().map(|row| row.0).collect(),
            firm: rows.iter().map(|row| row.1).collect(),
            deletion: rows.iter().map(|row| row.2).collect(),
            outcome: rows.iter().map(|row| row.3).collect(),
            frequency: rows.iter().map(|row| row.4).collect(),
            target_weight: rows.iter().map(|row| row.5).collect(),
            controls: vec![
                rows.iter().map(|row| row.6).collect(),
                rows.iter().map(|row| row.7).collect(),
            ],
        }
        .validate()
        .expect("tied-row fixture validates"),
    )
    .expect("tied-row fixture canonicalizes")
    .compress(&vec![true; retained])
    .expect("tied-row fixture compresses")
}

fn match_block_fixture() -> CompressedProblem {
    let problem = fixture(false);
    let rows = problem.outcome.len();
    CanonicalInput::from_validated(
        InputColumns {
            worker: problem
                .row_worker
                .iter()
                .map(|&value| 1 + u64::from(value))
                .collect(),
            firm: problem
                .row_firm
                .iter()
                .map(|&value| 1 + u64::from(value))
                .collect(),
            deletion: problem
                .row_cell
                .iter()
                .map(|&value| 1 + u64::from(value))
                .collect(),
            outcome: problem.outcome,
            frequency: problem.frequency,
            target_weight: problem.target_weight,
            controls: Vec::new(),
        }
        .validate()
        .expect("match-block fixture validates"),
    )
    .expect("match-block fixture canonicalizes")
    .compress(&vec![true; rows])
    .expect("match-block fixture compresses")
}

fn large_interrupt_fixture(rows: usize) -> CompressedProblem {
    let mut worker = Vec::with_capacity(rows);
    let mut firm = Vec::with_capacity(rows);
    let mut deletion = Vec::with_capacity(rows);
    let mut outcome = Vec::with_capacity(rows);
    let mut frequency = Vec::with_capacity(rows);
    let mut target_weight = Vec::with_capacity(rows);
    for row in 0..rows {
        worker.push(1 + (row % 3) as u64);
        firm.push(10 + ((row / 3) % 3) as u64);
        deletion.push(100 + row as u64);
        outcome.push(row as f64 / 17.0 + ((row * 7) % 11) as f64);
        frequency.push(1);
        target_weight.push(1.0);
    }
    CanonicalInput::from_validated(
        InputColumns {
            worker,
            firm,
            deletion,
            outcome,
            frequency,
            target_weight,
            controls: Vec::new(),
        }
        .validate()
        .expect("large interrupt fixture validates"),
    )
    .expect("large interrupt fixture canonicalizes")
    .compress(&vec![true; rows])
    .expect("large interrupt fixture compresses")
}

fn structural_route_fixture(firms: usize) -> CompressedProblem {
    let rows = 2 * firms;
    let mut worker = Vec::with_capacity(rows);
    let mut firm = Vec::with_capacity(rows);
    let mut deletion = Vec::with_capacity(rows);
    let mut outcome = Vec::with_capacity(rows);
    let mut frequency = Vec::with_capacity(rows);
    let mut target_weight = Vec::with_capacity(rows);
    for worker_index in 0..2_usize {
        for firm_index in 0..firms {
            let row = worker.len();
            worker.push(10 + worker_index as u64);
            firm.push(1_000 + firm_index as u64);
            deletion.push(10_000 + row as u64);
            outcome.push(
                0.4 * worker_index as f64 - 0.01 * firm_index as f64
                    + if (worker_index + firm_index) % 2 == 0 {
                        0.3
                    } else {
                        -0.2
                    },
            );
            frequency.push(1 + (row % 3) as u64);
            target_weight.push(0.5 + (row % 7) as f64 / 5.0);
        }
    }
    CanonicalInput::from_validated(
        InputColumns {
            worker,
            firm,
            deletion,
            outcome,
            frequency,
            target_weight,
            controls: Vec::new(),
        }
        .validate()
        .expect("structural route fixture validates"),
    )
    .expect("structural route fixture canonicalizes")
    .compress(&vec![true; rows])
    .expect("structural route fixture compresses")
}

fn options(deletion: DeletionMode, nuisance: NuisanceMode) -> GenericJlaOptions {
    GenericJlaOptions {
        seed: 0x55aa_1122_3344_7788,
        probes: 96,
        leverage_batch_width: 7,
        target_batch_width: 5,
        deletion,
        nuisance,
        rank_tolerance: 1.0e-10,
        block_tolerance: 1.0e-10,
        solver: ModelSolverOptions {
            pcg: PcgOptions {
                tolerance: 1.0e-12,
                maximum_iterations: 5_000,
                residual_replacement_interval: 17,
            },
            rank_tolerance: 1.0e-11,
        },
        ..GenericJlaOptions::default()
    }
}

fn routed_options(
    estimator: GenericJlaOptions,
    route: ModelSolverRoute,
) -> GenericJlaExecutionOptions {
    GenericJlaExecutionOptions {
        estimator,
        routing: ModelRoutingOptions {
            route,
            cmg_minimum_dimension: 1,
            allow_automatic_cmg_setup_fallback: true,
            solver: estimator.solver,
            cmg: CmgOptions::default(),
        },
        leverage_batch: BatchRequest::Explicit(estimator.leverage_batch_width),
        target_batch: BatchRequest::Explicit(estimator.target_batch_width),
        wallseconds: None,
    }
}

fn components(value: vckss_core::jla::VarianceComponents) -> [f64; 4] {
    [value.worker, value.firm, value.covariance, value.total]
}

fn assert_close(left: f64, right: f64, tolerance: f64) {
    assert!(
        (left - right).abs() <= tolerance * left.abs().max(right.abs()).max(1.0),
        "{left} versus {right} at tolerance {tolerance}"
    );
}

fn assert_components_bits(
    label: &str,
    left: vckss_core::jla::VarianceComponents,
    right: vckss_core::jla::VarianceComponents,
) {
    for (index, (left, right)) in components(left)
        .into_iter()
        .zip(components(right))
        .enumerate()
    {
        assert_eq!(left.to_bits(), right.to_bits(), "{label} component {index}");
    }
}

fn assert_counter_result_bits(left: &GenericJlaResult, right: &GenericJlaResult) {
    assert_components_bits("plugin", left.plugin, right.plugin);
    assert_components_bits("correction", left.correction, right.correction);
    assert_components_bits("corrected", left.corrected, right.corrected);
    assert_components_bits("mcse", left.numerical_mcse, right.numerical_mcse);
    assert_eq!(left.weighted_rss.to_bits(), right.weighted_rss.to_bits());
    let left_receipt = &left.receipt;
    let right_receipt = &right.receipt;
    assert_eq!(left_receipt.parameters, right_receipt.parameters);
    assert_eq!(left_receipt.full_parameters, right_receipt.full_parameters);
    assert_eq!(
        left_receipt.correction_parameters,
        right_receipt.correction_parameters
    );
    assert_eq!(left_receipt.deletion_units, right_receipt.deletion_units);
    assert_eq!(left_receipt.target_strata, right_receipt.target_strata);
    assert_eq!(
        left_receipt.leverage_probes_accepted,
        right_receipt.leverage_probes_accepted
    );
    assert_eq!(
        left_receipt.target_probes_accepted,
        right_receipt.target_probes_accepted
    );
    for (left, right) in [
        left_receipt.maximum_leverage,
        left_receipt.full_fit_relres,
        left_receipt.working_fit_relres,
        left_receipt.maximum_solve_relres,
        left_receipt.maximum_maker_relres,
        left_receipt.control_basis_relres,
        left_receipt.control_basis_forward_error,
        left_receipt.control_schur_rcond,
        left_receipt.control_schur_relres,
        left_receipt.deletion_rank_gap,
    ]
    .into_iter()
    .zip([
        right_receipt.maximum_leverage,
        right_receipt.full_fit_relres,
        right_receipt.working_fit_relres,
        right_receipt.maximum_solve_relres,
        right_receipt.maximum_maker_relres,
        right_receipt.control_basis_relres,
        right_receipt.control_basis_forward_error,
        right_receipt.control_schur_rcond,
        right_receipt.control_schur_relres,
        right_receipt.deletion_rank_gap,
    ]) {
        assert_eq!(left.to_bits(), right.to_bits());
    }
    assert_eq!(
        left_receipt.topology_checksum,
        right_receipt.topology_checksum
    );
}

fn assert_resource_receipt_eq(left: &GenericJlaResult, right: &GenericJlaResult) {
    assert_eq!(
        [
            left.receipt.peak_forecast_bytes,
            left.receipt.canonicalization_peak_forecast_bytes,
            left.receipt.fit_peak_forecast_bytes,
            left.receipt.geometry_peak_forecast_bytes,
            left.receipt.leverage_peak_forecast_bytes,
            left.receipt.target_peak_forecast_bytes,
            left.receipt.maker_peak_forecast_bytes,
            left.receipt.result_forecast_bytes,
        ],
        [
            right.receipt.peak_forecast_bytes,
            right.receipt.canonicalization_peak_forecast_bytes,
            right.receipt.fit_peak_forecast_bytes,
            right.receipt.geometry_peak_forecast_bytes,
            right.receipt.leverage_peak_forecast_bytes,
            right.receipt.target_peak_forecast_bytes,
            right.receipt.maker_peak_forecast_bytes,
            right.receipt.result_forecast_bytes,
        ]
    );
}

#[test]
fn dense_exact_oracle_covers_all_deletion_and_nuisance_combinations() {
    let problem = fixture(true);
    for deletion in [DeletionMode::Match, DeletionMode::Observation] {
        for nuisance in [NuisanceMode::Joint, NuisanceMode::FixedOffset] {
            let approximate =
                run_generic_jla(&problem, options(deletion, nuisance)).expect("generic JLA result");
            let exact = run_exact_estimator(
                &problem,
                ExactEstimatorOptions {
                    deletion,
                    nuisance,
                    rank_tolerance: 1.0e-10,
                    block_tolerance: 1.0e-10,
                    solver_tolerance: 1.0e-12,
                    ..ExactEstimatorOptions::default()
                },
            )
            .expect("dense exact oracle");
            for ((estimate, oracle), mcse) in components(approximate.correction)
                .into_iter()
                .zip(components(exact.correction))
                .zip(components(approximate.numerical_mcse))
            {
                assert!(
                    (estimate - oracle).abs() <= 12.0 * mcse + 5.0e-7,
                    "JLA {estimate}, dense oracle {oracle}, MCSE {mcse}, {deletion:?}/{nuisance:?}"
                );
            }
            approximate
                .plugin
                .verify_accounting(1.0e-11)
                .expect("plugin accounting");
            approximate
                .correction
                .verify_accounting(1.0e-11)
                .expect("correction accounting");
            approximate
                .corrected
                .verify_accounting(1.0e-11)
                .expect("corrected accounting");
            assert!(approximate.receipt.maximum_solve_relres <= 1.0e-10);
        }
    }
}

#[test]
fn private_oracle_variance_component_attachment_is_coherent_and_batch_invariant() {
    let problem = component_inference_fixture(true);
    let variance = (0..problem.outcome.len())
        .map(|row| 0.002 + (row % 9) as f64 / 10_000.0)
        .collect::<Vec<_>>();
    let mut estimator = options(DeletionMode::Observation, NuisanceMode::Joint);
    estimator.probes = 512;
    estimator.leverage_batch_width = 17;
    estimator.target_batch_width = 19;
    estimator.solver.pcg.tolerance = 1.0e-12;
    estimator.memory_limit_bytes = u64::MAX;
    let baseline = run_generic_jla_routed(
        &problem,
        routed_options(estimator, ModelSolverRoute::Diagonal),
    )
    .expect("baseline point estimate");

    let run = |batch_width| {
        let prepared = prepare_oracle_component_inference(
            &problem,
            &variance,
            ComponentInferenceOptions {
                seed: 0x91a2_b3c4_d5e6_f708,
                probes: 2_048,
                batch_width,
                psd_tolerance: 1.0e-8,
                spectrum_probes: 64,
                spectrum_iterations: 32,
                spectrum_tolerance: 1.0e-3,
                reference_distribution:
                    vckss_core::component_inference::ComponentReferenceDistribution::Q0,
                confidence_level: 0.95,
                critical_simulations: 100_000,
            },
        )
        .expect("private oracle-variance preparation");
        run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
            &problem,
            routed_options(estimator, ModelSolverRoute::Diagonal),
            None,
            Some(&prepared),
            None,
            &mut NeverInterrupt,
        )
        .expect("private component-inference attachment")
    };
    let scalar = run(1);
    let batched = run(23);
    assert_components_bits("attached plugin", baseline.plugin, scalar.plugin);
    assert_components_bits(
        "attached correction",
        baseline.correction,
        scalar.correction,
    );
    assert_components_bits("attached corrected", baseline.corrected, scalar.corrected);
    assert_components_bits(
        "attached point MCSE",
        baseline.numerical_mcse,
        scalar.numerical_mcse,
    );
    assert_eq!(
        baseline.weighted_rss.to_bits(),
        scalar.weighted_rss.to_bits()
    );
    assert_counter_result_bits(&scalar, &batched);
    let left = scalar
        .component_inference
        .as_ref()
        .expect("scalar attachment result");
    let right = batched
        .component_inference
        .as_ref()
        .expect("batched attachment result");
    assert_eq!(left.primitive_covariance, right.primitive_covariance);
    assert_eq!(left.covariance, right.covariance);
    assert_eq!(left.trace_term, right.trace_term);
    assert_eq!(left.trace_mcse, right.trace_mcse);
    assert_eq!(left.probe_mean, right.probe_mean);
    assert_eq!(left.influence_concentration, right.influence_concentration);
    assert_eq!(left.spectrum, right.spectrum);
    assert_eq!(
        left.point_correction_identity_error,
        right.point_correction_identity_error
    );
    let spectrum_receipts = 5 * 64 + 16 * 32 + 18;
    assert_eq!(
        left.solve_receipts.len(),
        2_048 + PRIMITIVE_TARGETS + spectrum_receipts
    );
    assert_eq!(
        left.counter_atoms,
        (2_048 + 64 + 2) * problem.outcome.len() as u64
    );
    assert_eq!(left.counter_words, 2 * left.counter_atoms);
    assert!(left.maximum_complete_residual <= left.full_residual_tolerance);
    assert!(left.point_correction_identity_error < 1.0e-10);
    assert!(left
        .influence_concentration
        .iter()
        .all(|value| value.is_finite() && (0.0..=1.0).contains(value)));
    assert_eq!(
        scalar.receipt.component_inference_peak_forecast_bytes,
        left.peak_forecast_bytes
    );
    assert_eq!(
        scalar
            .receipt
            .execution
            .memory
            .component_inference_peak_bytes,
        left.peak_forecast_bytes
    );
    for index in 0..4 {
        assert_eq!(
            left.covariance[12 + index],
            left.covariance[index] + left.covariance[4 + index] + 2.0 * left.covariance[8 + index]
        );
    }
}

#[test]
fn structured_common_variance_is_attached_once_and_preserves_point_estimates() {
    let problem = structured_component_inference_fixture();
    let mut estimator = options(DeletionMode::Observation, NuisanceMode::Joint);
    estimator.probes = 256;
    estimator.leverage_batch_width = 13;
    estimator.target_batch_width = 17;
    estimator.solver.pcg.tolerance = 1.0e-12;
    estimator.memory_limit_bytes = u64::MAX;
    let baseline = run_generic_jla_routed(
        &problem,
        routed_options(estimator, ModelSolverRoute::Diagonal),
    )
    .expect("structured-variance baseline point estimate");
    let run = |batch_width| {
        let prepared = prepare_structured_component_inference(
            &problem,
            ComponentVarianceSource::StructuredCommon,
            ComponentInferenceOptions {
                seed: 0x31c0_ffee_782a_19d4,
                probes: 512,
                batch_width,
                spectrum_probes: 32,
                spectrum_iterations: 24,
                spectrum_tolerance: 2.0e-3,
                ..ComponentInferenceOptions::default()
            },
            StructuredVarianceOptions {
                seed: 0x83d4_2556_97ab_c10e,
                ..StructuredVarianceOptions::default()
            },
        )
        .expect("structured common-variance preparation");
        run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
            &problem,
            routed_options(estimator, ModelSolverRoute::Diagonal),
            None,
            Some(&prepared),
            None,
            &mut NeverInterrupt,
        )
        .expect("structured common-variance attachment")
    };
    let scalar = run(1);
    let batched = run(19);
    assert_components_bits("structured plugin", baseline.plugin, scalar.plugin);
    assert_components_bits(
        "structured correction",
        baseline.correction,
        scalar.correction,
    );
    assert_components_bits("structured corrected", baseline.corrected, scalar.corrected);
    assert_components_bits(
        "structured point MCSE",
        baseline.numerical_mcse,
        scalar.numerical_mcse,
    );
    assert_counter_result_bits(&scalar, &batched);
    let left = scalar
        .component_inference
        .expect("structured inference result");
    let right = batched
        .component_inference
        .expect("batched structured inference result");
    assert_eq!(
        left.variance_source,
        ComponentVarianceSource::StructuredCommon
    );
    assert_eq!(left.primitive_covariance, right.primitive_covariance);
    assert_eq!(left.covariance, right.covariance);
    assert_eq!(left.trace_mcse, right.trace_mcse);
    assert_eq!(left.spectrum, right.spectrum);
    assert_eq!(left.structured_variance, right.structured_variance);
    let fitted = left
        .structured_variance
        .expect("both structured sensitivity fits are retained");
    assert_eq!(fitted.common.len(), problem.outcome.len());
    assert_eq!(fitted.leverage_only.len(), problem.outcome.len());
    assert!(fitted
        .common
        .iter()
        .chain(&fitted.leverage_only)
        .all(|value| value.is_finite() && *value > 0.0));
    assert_eq!(fitted.folds.len(), 10);
    assert_eq!(fitted.cv.len(), 70);
    assert_eq!(fitted.counter_atoms, 5 * problem.outcome.len() as u64);
    let gaussian_atoms = (512 + 32 + 2) * problem.outcome.len() as u64;
    assert_eq!(left.counter_atoms, gaussian_atoms + fitted.counter_atoms);
    assert_eq!(
        left.counter_words,
        2 * gaussian_atoms + fitted.counter_words
    );
}

#[test]
fn structured_leverage_variance_supports_explicit_q1() {
    let problem = structured_component_inference_fixture();
    let mut estimator = options(DeletionMode::Observation, NuisanceMode::Joint);
    estimator.probes = 192;
    estimator.solver.pcg.tolerance = 1.0e-12;
    estimator.memory_limit_bytes = u64::MAX;
    let prepared = prepare_structured_component_inference(
        &problem,
        ComponentVarianceSource::StructuredLeverage,
        ComponentInferenceOptions {
            seed: 0x1764_53a2_98de_bc01,
            probes: 512,
            batch_width: 16,
            spectrum_probes: 48,
            spectrum_iterations: 32,
            reference_distribution: ComponentReferenceDistribution::Q1,
            critical_simulations: 2_000,
            ..ComponentInferenceOptions::default()
        },
        StructuredVarianceOptions::default(),
    )
    .expect("structured leverage-only q=1 preparation");
    let result = run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        &problem,
        routed_options(estimator, ModelSolverRoute::Diagonal),
        None,
        Some(&prepared),
        None,
        &mut NeverInterrupt,
    )
    .expect("structured leverage-only q=1 attachment");
    let inference = result.component_inference.expect("q=1 inference result");
    assert_eq!(
        inference.variance_source,
        ComponentVarianceSource::StructuredLeverage
    );
    let q1 = inference.q1.expect("explicit q=1 results");
    assert!(q1.iter().all(|target| {
        target.confidence_lower.is_finite()
            && target.confidence_upper.is_finite()
            && target.confidence_lower < target.confidence_upper
    }));
    assert!(inference.structured_variance.is_some());
}

#[test]
fn internal_fixedoffset_match_q0_uses_one_inference_unit_per_declared_match() {
    let problem = grouped_component_inference_fixture(7);
    let mut estimator = options(DeletionMode::Match, NuisanceMode::FixedOffset);
    estimator.probes = 96;
    estimator.leverage_batch_width = 11;
    estimator.target_batch_width = 13;
    let routed = routed_options(estimator, ModelSolverRoute::Diagonal);
    let baseline = run_generic_jla_routed(&problem, routed).expect("point-only grouped result");
    let prepared = prepare_grouped_oracle_component_inference(
        &problem,
        &vec![0.04; problem.deletion_units()],
        ComponentInferenceOptions {
            probes: 192,
            batch_width: 17,
            spectrum_probes: 96,
            spectrum_iterations: 96,
            spectrum_tolerance: 1.0e-2,
            ..ComponentInferenceOptions::default()
        },
    )
    .expect("grouped oracle preparation");
    let mut scalar_options = prepared.options;
    scalar_options.batch_width = 1;
    let scalar_prepared = prepare_grouped_oracle_component_inference(
        &problem,
        &vec![0.04; problem.deletion_units()],
        scalar_options,
    )
    .expect("scalar-batch grouped oracle preparation");
    let attached = run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        &problem,
        routed,
        None,
        Some(&prepared),
        None,
        &mut NeverInterrupt,
    )
    .expect("internal fixed-offset grouped q=0 result");
    let scalar_attached = run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        &problem,
        routed,
        None,
        Some(&scalar_prepared),
        None,
        &mut NeverInterrupt,
    )
    .expect("scalar-batch internal fixed-offset grouped q=0 result");
    let cmg_attached = run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        &problem,
        routed_options(estimator, ModelSolverRoute::Cmg),
        None,
        Some(&prepared),
        None,
        &mut NeverInterrupt,
    )
    .expect("internal fixed-offset grouped CMG q=0 result");
    assert_counter_result_bits(&attached, &scalar_attached);
    for (left, right) in components(baseline.plugin)
        .into_iter()
        .zip(components(attached.plugin))
        .chain(
            components(baseline.correction)
                .into_iter()
                .zip(components(attached.correction)),
        )
        .chain(
            components(baseline.corrected)
                .into_iter()
                .zip(components(attached.corrected)),
        )
    {
        assert_eq!(left.to_bits(), right.to_bits());
    }
    let inference = attached
        .component_inference
        .expect("grouped q=0 attachment");
    let scalar_inference = scalar_attached
        .component_inference
        .expect("scalar-batch grouped q=0 attachment");
    let cmg_inference = cmg_attached
        .component_inference
        .expect("grouped CMG q=0 attachment");
    assert_eq!(
        inference.primitive_covariance,
        scalar_inference.primitive_covariance
    );
    assert_eq!(inference.covariance, scalar_inference.covariance);
    assert_eq!(inference.trace_mcse, scalar_inference.trace_mcse);
    assert_eq!(inference.spectrum, scalar_inference.spectrum);
    for (left, right) in inference
        .covariance
        .iter()
        .zip(cmg_inference.covariance.iter())
    {
        assert_close(*left, *right, 2.0e-8);
    }
    assert_eq!(inference.inference_unit, ComponentInferenceUnit::Match);
    assert_eq!(inference.independent_units, problem.deletion_units() as u64);
    assert!(inference.nuisance_uncertainty_conditioned_away);
    assert!(inference.effective_match_count > 1.0);
    assert!((0.0..1.0).contains(&inference.largest_match_mass_share));
    assert!((0.0..1.0).contains(&inference.largest_match_leverage));
    assert!(inference.smallest_maker_denominator > 0.0);
    assert!(inference.structured_variance.is_none());
    assert!(inference.q1.is_none());
    assert_eq!(inference.leverage.len(), problem.deletion_units());
    assert_eq!(inference.target_diagonal[0].len(), problem.deletion_units());
    let gaussian_atoms = (192 + 96 + 2) * problem.deletion_units() as u64;
    assert_eq!(inference.counter_atoms, gaussian_atoms);
    assert_eq!(inference.counter_words, 2 * gaussian_atoms);
    let dense_kernels = dense_grouped_component_kernels(
        &problem,
        &inference.target_diagonal,
        &inference.maker_inverse,
    );
    let aggregate_variance = 0.04_f64;
    for left in 0..PRIMITIVE_TARGETS {
        for right in 0..PRIMITIVE_TARGETS {
            let expected = 2.0
                * aggregate_variance.powi(2)
                * dense_kernels[left]
                    .iter()
                    .zip(&dense_kernels[right])
                    .map(|(left, right)| left * right)
                    .sum::<f64>();
            let index = left * PRIMITIVE_TARGETS + right;
            let tolerance = 6.0 * inference.trace_mcse[index]
                + 1.0e-9
                    * expected
                        .abs()
                        .max(inference.trace_term[index].abs())
                        .max(1.0);
            assert!(
                (inference.trace_term[index] - expected).abs() <= tolerance,
                "grouped trace ({left},{right}) observed={} expected={expected} mcse={} tolerance={tolerance}",
                inference.trace_term[index],
                inference.trace_mcse[index]
            );
        }
    }
    assert!(inference.maximum_complete_residual <= inference.full_residual_tolerance);
    assert!(cmg_inference.maximum_complete_residual <= cmg_inference.full_residual_tolerance);
}

#[test]
fn internal_fixedoffset_match_q0_fits_registered_match_variance_models() {
    let problem = grouped_component_inference_fixture(14);
    let mut estimator = options(DeletionMode::Match, NuisanceMode::FixedOffset);
    estimator.probes = 64;
    estimator.leverage_batch_width = 16;
    estimator.target_batch_width = 16;
    let prepared = prepare_grouped_structured_component_inference(
        &problem,
        ComponentVarianceSource::StructuredCommon,
        ComponentInferenceOptions {
            probes: 512,
            batch_width: 16,
            spectrum_probes: 128,
            spectrum_iterations: 64,
            spectrum_tolerance: 1.0e-2,
            ..ComponentInferenceOptions::default()
        },
        StructuredVarianceOptions::default(),
    )
    .expect("grouped structured q=0 preparation");
    let result = run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        &problem,
        routed_options(estimator, ModelSolverRoute::Diagonal),
        None,
        Some(&prepared),
        None,
        &mut NeverInterrupt,
    )
    .expect("grouped structured q=0 result");
    let inference = result.component_inference.expect("grouped inference");
    let structured = inference
        .structured_variance
        .expect("structured match variances");
    assert_eq!(structured.common.len(), problem.deletion_units());
    assert_eq!(structured.leverage_only.len(), problem.deletion_units());
    assert_eq!(
        structured.summary[0].observations,
        problem.deletion_units() as u64
    );
    assert!(structured.folds.iter().any(|fold| fold.active_terms > 15));
    assert!(structured
        .common
        .iter()
        .all(|value| value.is_finite() && *value > 0.0));
    assert!(structured
        .leverage_only
        .iter()
        .all(|value| value.is_finite() && *value > 0.0));
}

#[test]
fn grouped_q1_and_wrong_nuisance_fail_before_inference_rng() {
    let problem = grouped_component_inference_fixture(5);
    let q1 = prepare_grouped_structured_component_inference(
        &problem,
        ComponentVarianceSource::StructuredCommon,
        ComponentInferenceOptions {
            reference_distribution: ComponentReferenceDistribution::Q1,
            ..ComponentInferenceOptions::default()
        },
        StructuredVarianceOptions::default(),
    )
    .expect_err("grouped q=1 remains staged");
    assert_eq!(q1.code, ErrorCode::UnsupportedFeature);
    assert_eq!(q1.phase, "component_inference_prepare");

    let prepared = prepare_grouped_oracle_component_inference(
        &problem,
        &vec![0.03; problem.deletion_units()],
        ComponentInferenceOptions {
            probes: 16,
            spectrum_probes: 16,
            spectrum_iterations: 16,
            spectrum_tolerance: 5.0e-2,
            ..ComponentInferenceOptions::default()
        },
    )
    .expect("grouped q=0 preparation");
    let error = run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        &problem,
        routed_options(
            options(DeletionMode::Match, NuisanceMode::Joint),
            ModelSolverRoute::Diagonal,
        ),
        None,
        Some(&prepared),
        None,
        &mut NeverInterrupt,
    )
    .expect_err("grouped joint-nuisance inference rejects");
    assert_eq!(error.code, ErrorCode::UnsupportedFeature);
    assert_eq!(error.phase, "generic_jla_component_inference");
}

#[test]
fn grouped_inference_preserves_cross_coordinate_and_delete_match_failures() {
    let invalid = InputColumns {
        worker: vec![1, 2, 1, 2],
        firm: vec![10, 20, 20, 10],
        deletion: vec![100, 100, 200, 300],
        outcome: vec![0.2, 0.4, 0.7, 0.1],
        frequency: vec![1; 4],
        target_weight: vec![1.0; 4],
        controls: Vec::new(),
    }
    .validate()
    .expect("column validation precedes coordinate validation");
    let error = CanonicalInput::from_validated(invalid)
        .expect_err("one deletion identifier cannot cross worker-firm coordinates");
    assert_eq!(error.code, ErrorCode::InvalidIdentifier);
    assert_eq!(error.phase, "canonicalize");

    let bridge = bridge_match_component_inference_fixture();
    let prepared = prepare_grouped_oracle_component_inference(
        &bridge,
        &vec![0.05; bridge.deletion_units()],
        ComponentInferenceOptions {
            probes: 32,
            spectrum_probes: 16,
            spectrum_iterations: 16,
            spectrum_tolerance: 5.0e-2,
            ..ComponentInferenceOptions::default()
        },
    )
    .expect("bridge grouped preparation");
    let mut estimator = options(DeletionMode::Match, NuisanceMode::FixedOffset);
    estimator.probes = 128;
    let error = run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        &bridge,
        routed_options(estimator, ModelSolverRoute::Diagonal),
        None,
        Some(&prepared),
        None,
        &mut NeverInterrupt,
    )
    .expect_err("deleting the bridge match must fail closed");
    assert!(matches!(
        error.code,
        ErrorCode::JlaConstraintFailed
            | ErrorCode::NonestimableDeletion
            | ErrorCode::BlockInverseFailed
            | ErrorCode::GraphUnidentified
            | ErrorCode::GraphCertificateFailed
    ));
}

#[test]
fn private_oracle_q1_decomposition_is_explicit_and_batch_invariant() {
    let problem = component_inference_fixture(false);
    let variance = (0..problem.outcome.len())
        .map(|row| 0.003 + (row % 7) as f64 / 20_000.0)
        .collect::<Vec<_>>();
    let mut estimator = options(DeletionMode::Observation, NuisanceMode::Joint);
    estimator.probes = 384;
    estimator.solver.pcg.tolerance = 1.0e-12;
    estimator.memory_limit_bytes = u64::MAX;
    let baseline = run_generic_jla_routed(
        &problem,
        routed_options(estimator, ModelSolverRoute::Diagonal),
    )
    .expect("q=1 baseline point estimate");
    let run = |batch_width| {
        let prepared = prepare_oracle_component_inference(
            &problem,
            &variance,
            ComponentInferenceOptions {
                seed: 0x17c3_45a9_d20e_6b81,
                probes: 4_096,
                batch_width,
                psd_tolerance: 1.0e-8,
                spectrum_probes: 128,
                spectrum_iterations: 48,
                spectrum_tolerance: 2.0e-3,
                reference_distribution: ComponentReferenceDistribution::Q1,
                confidence_level: 0.95,
                critical_simulations: 20_000,
            },
        )
        .expect("private oracle q=1 preparation");
        run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
            &problem,
            routed_options(estimator, ModelSolverRoute::Diagonal),
            None,
            Some(&prepared),
            None,
            &mut NeverInterrupt,
        )
        .expect("private oracle q=1 attachment")
    };
    let scalar = run(1);
    let batched = run(29);
    assert_components_bits("q=1 attached plugin", baseline.plugin, scalar.plugin);
    assert_components_bits(
        "q=1 attached correction",
        baseline.correction,
        scalar.correction,
    );
    assert_components_bits(
        "q=1 attached corrected",
        baseline.corrected,
        scalar.corrected,
    );
    assert_counter_result_bits(&scalar, &batched);
    let left = scalar.component_inference.expect("q=1 result");
    let right = batched.component_inference.expect("batched q=1 result");
    assert_eq!(left.spectrum, right.spectrum);
    assert_eq!(left.q1, right.q1);
    let dense_spectrum = dense_component_spectra(&problem);
    for target in 0..4 {
        let observed = left.spectrum[target];
        let expected = dense_spectrum[target];
        let leading_scale = expected.0.abs().max(1.0e-12);
        let second_scale = expected.1.abs().max(leading_scale * 1.0e-3);
        assert!(
            (observed.leading_eigenvalue.abs() - expected.0.abs()).abs() <= 4.0e-3 * leading_scale,
            "target {target} leading mode: {} versus {}",
            observed.leading_eigenvalue,
            expected.0
        );
        assert!(
            (observed.second_eigenvalue.abs() - expected.1.abs()).abs() <= 8.0e-3 * second_scale,
            "target {target} second mode: {} versus {}",
            observed.second_eigenvalue,
            expected.1
        );
        assert!(
            (observed.trace_square_raw - expected.2).abs()
                <= 6.0 * observed.trace_square_mcse + 1.0e-10,
            "target {target} trace square: {} versus {} with MCSE {}",
            observed.trace_square_raw,
            expected.2,
            observed.trace_square_mcse
        );
    }
    let q1 = left.q1.expect("explicit q=1 decomposition");
    for (target, result) in q1.iter().enumerate() {
        let reconstructed = result.leading_recentered_component + result.remainder_estimate;
        assert!(
            (result.point_estimate - reconstructed).abs()
                <= 2.0e-15 * result.point_estimate.abs().max(1.0),
            "target {target} must retain the q=1 decomposition"
        );
        assert!(result.leading_variance > 0.0);
        assert!(result.remainder_variance > 0.0);
        assert!(result.remainder_trace_mcse >= 0.0);
        assert!(result.curvature.is_finite() && result.curvature > 0.0);
        assert!(result.critical_value.is_finite() && result.critical_value > 0.0);
        assert!(result.confidence_lower < result.confidence_upper);
        assert!(result.leading_f_statistic.is_finite() && result.leading_f_statistic >= 0.0);
        assert!(
            result.remainder_influence_concentration.is_finite()
                && (0.0..=1.0).contains(&result.remainder_influence_concentration)
        );
        assert!(left.spectrum[target].leading_share > 0.0);
        assert!(left.spectrum[target].remainder_leading_share >= 0.0);
    }
}

#[test]
fn private_component_attachment_has_exact_memory_boundary() {
    let problem = component_inference_fixture(true);
    let variance = vec![0.002; problem.outcome.len()];
    let prepared = prepare_oracle_component_inference(
        &problem,
        &variance,
        ComponentInferenceOptions {
            seed: 0x0d15_ea5e,
            probes: 128,
            batch_width: 7,
            psd_tolerance: 1.0e-8,
            spectrum_probes: 32,
            spectrum_iterations: 24,
            spectrum_tolerance: 1.0e-3,
            reference_distribution:
                vckss_core::component_inference::ComponentReferenceDistribution::Q0,
            confidence_level: 0.95,
            critical_simulations: 100_000,
        },
    )
    .expect("private oracle-variance preparation");
    let mut estimator = options(DeletionMode::Observation, NuisanceMode::Joint);
    estimator.probes = 128;
    estimator.memory_limit_bytes = u64::MAX;
    let run = |estimator| {
        run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
            &problem,
            routed_options(estimator, ModelSolverRoute::Diagonal),
            None,
            Some(&prepared),
            None,
            &mut NeverInterrupt,
        )
    };
    let baseline = run(estimator).expect("unlimited attachment run");
    let peak = baseline.receipt.peak_forecast_bytes;
    assert_eq!(
        baseline.receipt.execution.memory.peak_bytes,
        baseline.receipt.peak_forecast_bytes
    );
    estimator.memory_limit_bytes = peak;
    run(estimator).expect("exact component forecast boundary accepts");
    estimator.memory_limit_bytes = peak.checked_sub(1).expect("positive forecast");
    let error = run(estimator).expect_err("one byte below component forecast rejects");
    assert_eq!(error.code, ErrorCode::ResourceLimit);
    assert_eq!(error.phase, "generic_jla_memory");
}

#[test]
fn grouped_component_attachment_has_exact_memory_boundary() {
    let problem = grouped_component_inference_fixture(5);
    let prepared = prepare_grouped_oracle_component_inference(
        &problem,
        &vec![0.04; problem.deletion_units()],
        ComponentInferenceOptions {
            probes: 64,
            batch_width: 7,
            spectrum_probes: 24,
            spectrum_iterations: 24,
            spectrum_tolerance: 5.0e-2,
            ..ComponentInferenceOptions::default()
        },
    )
    .expect("grouped oracle preparation");
    let mut estimator = options(DeletionMode::Match, NuisanceMode::FixedOffset);
    estimator.probes = 64;
    estimator.memory_limit_bytes = u64::MAX;
    let run = |estimator| {
        run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
            &problem,
            routed_options(estimator, ModelSolverRoute::Diagonal),
            None,
            Some(&prepared),
            None,
            &mut NeverInterrupt,
        )
    };
    let baseline = run(estimator).expect("unlimited grouped attachment run");
    let peak = baseline.receipt.peak_forecast_bytes;
    assert_eq!(
        baseline.receipt.execution.memory.peak_bytes,
        baseline.receipt.peak_forecast_bytes
    );
    estimator.memory_limit_bytes = peak;
    run(estimator).expect("exact grouped component forecast boundary accepts");
    estimator.memory_limit_bytes = peak.checked_sub(1).expect("positive forecast");
    let error = run(estimator).expect_err("one byte below grouped forecast rejects");
    assert_eq!(error.code, ErrorCode::ResourceLimit);
    assert_eq!(error.phase, "generic_jla_memory");
}

#[test]
fn private_component_attachment_accepts_cmg_and_rejects_unsupported_tuples() {
    let problem = component_inference_fixture(false);
    let variance = vec![0.002; problem.outcome.len()];
    let prepared = prepare_oracle_component_inference(
        &problem,
        &variance,
        ComponentInferenceOptions {
            probes: 512,
            batch_width: 13,
            ..ComponentInferenceOptions::default()
        },
    )
    .expect("private oracle-variance preparation");
    let run = |deletion, nuisance, route| {
        run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
            &problem,
            routed_options(options(deletion, nuisance), route),
            None,
            Some(&prepared),
            None,
            &mut NeverInterrupt,
        )
    };
    let cmg = run(
        DeletionMode::Observation,
        NuisanceMode::Joint,
        ModelSolverRoute::Cmg,
    )
    .expect("explicit CMG attachment run");
    let component = cmg.component_inference.expect("CMG inference result");
    assert!(component.maximum_complete_residual <= component.full_residual_tolerance);
    for (deletion, nuisance, route) in [
        (
            DeletionMode::Match,
            NuisanceMode::Joint,
            ModelSolverRoute::Diagonal,
        ),
        (
            DeletionMode::Observation,
            NuisanceMode::FixedOffset,
            ModelSolverRoute::Diagonal,
        ),
        (
            DeletionMode::Observation,
            NuisanceMode::Joint,
            ModelSolverRoute::Auto,
        ),
    ] {
        let error = run(deletion, nuisance, route).expect_err("unsupported tuple rejects");
        assert_eq!(error.code, ErrorCode::UnsupportedFeature);
        assert_eq!(error.phase, "generic_jla_component_inference");
    }

    let projection = prepare_projection(
        &problem,
        std::slice::from_ref(&problem.outcome),
        ProjectionEffect::Worker,
        ProjectionWeight::Frequency,
        1.0e-10,
    )
    .expect("projection preparation");
    let mut projected = options(DeletionMode::Observation, NuisanceMode::Joint);
    projected.projection_columns = projection.columns;
    projected.projection_result_bytes = 1;
    projected.projection_export_bytes = 1;
    let error = run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        &problem,
        routed_options(projected, ModelSolverRoute::Diagonal),
        Some(&projection),
        Some(&prepared),
        None,
        &mut NeverInterrupt,
    )
    .expect_err("simultaneous projection and component inference rejects");
    assert_eq!(error.code, ErrorCode::UnsupportedFeature);
    assert_eq!(error.phase, "generic_jla_component_inference");

    let stayer_problem = component_inference_stayer_fixture();
    let stayer_prepared = prepare_oracle_component_inference(
        &stayer_problem,
        &vec![0.002; stayer_problem.outcome.len()],
        ComponentInferenceOptions {
            probes: 32,
            ..ComponentInferenceOptions::default()
        },
    )
    .expect("stayer oracle-variance preparation");
    let error = run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        &stayer_problem,
        routed_options(
            options(DeletionMode::Observation, NuisanceMode::Joint),
            ModelSolverRoute::Diagonal,
        ),
        None,
        Some(&stayer_prepared),
        None,
        &mut NeverInterrupt,
    )
    .expect_err("eligible stayer rejects");
    assert_eq!(error.code, ErrorCode::UnsupportedFeature);
    assert_eq!(error.phase, "generic_jla_component_inference");

    let nonunit = fixture(false);
    let error = prepare_oracle_component_inference(
        &nonunit,
        &vec![0.002; nonunit.outcome.len()],
        ComponentInferenceOptions::default(),
    )
    .expect_err("nonunit frequencies reject before generation");
    assert_eq!(error.code, ErrorCode::UnsupportedFeature);
    assert_eq!(error.phase, "component_inference_prepare");
}

#[test]
fn mixed_mover_match_and_stayer_observation_jla_matches_exact_oracle() {
    let mover = fixture(true);
    let prepared = prepare_exact_stayer_hybrid(
        &mover,
        StayerAugmentationInput {
            firm: vec![1, 1, 2, 2],
            worker: vec![1, 1, 2, 2],
            outcome: vec![0.4, 0.9, -0.2, 0.35],
            frequency: vec![1, 1, 1, 1],
            target_weight: vec![0.7, 1.1, 0.6, 0.9],
            controls: vec![vec![-0.6, 0.8, 0.2, -0.4], vec![0.3, -0.5, 0.9, -0.1]],
        },
    )
    .expect("hybrid fixture prepares");
    let exact = run_exact_stayer_hybrid(
        &prepared.problem,
        &prepared.plan,
        ExactEstimatorOptions {
            deletion: DeletionMode::Match,
            nuisance: NuisanceMode::Joint,
            rank_tolerance: 1.0e-10,
            block_tolerance: 1.0e-10,
            solver_tolerance: 1.0e-12,
            ..ExactEstimatorOptions::default()
        },
    )
    .expect("mixed exact oracle");
    let mut estimator = options(DeletionMode::Match, NuisanceMode::Joint);
    estimator.probes = 1_024;
    estimator.leverage_batch_width = 31;
    estimator.target_batch_width = 29;
    let approximate = run_generic_jla_routed_with_projection_and_hybrid_interrupt(
        &prepared.problem,
        routed_options(estimator, ModelSolverRoute::Diagonal),
        None,
        Some(&prepared.plan),
        &mut NeverInterrupt,
    )
    .expect("mixed generic JLA result");
    for ((estimate, oracle), mcse) in components(approximate.correction)
        .into_iter()
        .zip(components(exact.estimator.correction))
        .zip(components(approximate.numerical_mcse))
    {
        assert!(
            (estimate - oracle).abs() <= 12.0 * mcse + 2.0e-6,
            "mixed JLA {estimate}, exact oracle {oracle}, MCSE {mcse}"
        );
    }
    assert_eq!(
        approximate.receipt.deletion_units,
        prepared.receipt.combined_deletion_units
    );
    assert_eq!(approximate.receipt.full_parameters, 9);
    approximate
        .corrected
        .verify_accounting(1.0e-11)
        .expect("mixed corrected accounting");
}

#[test]
fn zero_stayer_hybrid_certificate_reduces_to_mover_jla() {
    let mover = fixture(false);
    let prepared = prepare_exact_stayer_hybrid(
        &mover,
        StayerAugmentationInput {
            firm: Vec::new(),
            worker: Vec::new(),
            outcome: Vec::new(),
            frequency: Vec::new(),
            target_weight: Vec::new(),
            controls: Vec::new(),
        },
    )
    .expect("zero-stayer fixture prepares");
    let estimator = options(DeletionMode::Match, NuisanceMode::Joint);
    let mover_result = run_generic_jla(&mover, estimator).expect("mover JLA");
    let hybrid_result = run_generic_jla_routed_with_projection_and_hybrid_interrupt(
        &prepared.problem,
        routed_options(estimator, ModelSolverRoute::Diagonal),
        None,
        Some(&prepared.plan),
        &mut NeverInterrupt,
    )
    .expect("zero-stayer hybrid JLA");
    assert_counter_result_bits(&mover_result, &hybrid_result);
}

#[test]
fn counter_results_do_not_depend_on_physical_batch_width() {
    let problem = fixture(true);
    for deletion in [DeletionMode::Match, DeletionMode::Observation] {
        for nuisance in [NuisanceMode::Joint, NuisanceMode::FixedOffset] {
            let mut scalar = options(deletion, nuisance);
            scalar.probes = 23;
            scalar.leverage_batch_width = 1;
            scalar.target_batch_width = 1;
            let mut wide = scalar;
            wide.leverage_batch_width = 23;
            wide.target_batch_width = 23;
            let left = run_generic_jla(&problem, scalar).expect("scalar batches");
            let right = run_generic_jla(&problem, wide).expect("wide batches");
            // Resource forecasts encode the requested width; every scientific
            // Counter-V1 output and receipt remains bitwise invariant.
            assert_counter_result_bits(&left, &right);
        }
    }
}

#[test]
fn no_control_match_remains_supported_and_target_strata_are_compressed() {
    let problem = fixture(false);
    let mut estimator_options = options(DeletionMode::Match, NuisanceMode::Joint);
    estimator_options.probes = 17;
    let result = run_generic_jla(&problem, estimator_options).expect("no-control match");
    assert_eq!(result.receipt.full_parameters, 5);
    assert!(result.receipt.target_strata <= problem.outcome.len());
    result
        .correction
        .verify_accounting(1.0e-11)
        .expect("accounting identity");
}

#[test]
fn maximum_control_receipts_are_lossless_ordered_and_count_every_rhs() {
    let problem = maximum_control_fixture();
    let mut estimator_options = options(DeletionMode::Match, NuisanceMode::FixedOffset);
    estimator_options.probes = 3;
    estimator_options.leverage_batch_width = 2;
    estimator_options.target_batch_width = 2;
    let result = run_generic_jla(&problem, estimator_options).expect("Q=32 generic JLA result");
    let receipt = &result.receipt;
    let expected = 32 + 1 + 1 + 3 + 2 * 3;
    assert_eq!(receipt.rhs.len(), expected);
    assert_eq!(receipt.control_rank.controls, 32);
    assert_eq!(receipt.control_rank.projection_rhs.len(), 32);
    for (logical_control, (rank, rhs)) in receipt
        .control_rank
        .projection_rhs
        .iter()
        .zip(&receipt.rhs[..32])
        .enumerate()
    {
        assert_eq!(rank.logical_control, logical_control);
        assert_eq!(rank.complete_residual_equations, 16);
        assert_eq!(rank.solver_dimension, 8);
        assert_eq!(rank.complete_residual_tolerance, 1.0e-11);
        assert!(rank.complete_residual <= rank.complete_residual_tolerance);
        assert_eq!(rhs.phase, GenericJlaRhsPhase::ControlProjection);
        assert_eq!(rhs.side, GenericJlaRhsSide::Joint);
        assert_eq!(rhs.probe, Some(logical_control as u32));
        assert_eq!(rhs.pcg.status, rank.pcg.status);
        assert_eq!(rhs.pcg.iterations, rank.pcg.iterations);
        assert_eq!(
            rhs.pcg.operator_applications,
            rank.pcg.operator_applications
        );
        assert_eq!(
            rhs.pcg.preconditioner_applications,
            rank.pcg.preconditioner_applications
        );
        assert_eq!(
            rhs.pcg.residual_replacements,
            rank.pcg.residual_replacements
        );
        assert_eq!(rhs.pcg.relative_residual, rank.pcg.relative_residual);
        assert_eq!(rhs.complete_residual, rank.complete_residual);
        assert!(matches!(
            rhs.pcg.status,
            ModelPcgStatus::ZeroRhs | ModelPcgStatus::Converged
        ));
    }
    assert_eq!(receipt.rhs[32].phase, GenericJlaRhsPhase::FullJointFit);
    assert_eq!(
        receipt.rhs[32].complete_residual,
        receipt.full_fit_complete_residual
    );
    assert_eq!(
        receipt.rhs[33].phase,
        GenericJlaRhsPhase::FixedOffsetWorkingFit
    );
    assert_eq!(
        receipt.rhs[33].complete_residual,
        receipt.working_fit_complete_residual
    );
    for (probe, rhs) in receipt.rhs[34..37].iter().enumerate() {
        assert_eq!(rhs.phase, GenericJlaRhsPhase::Leverage);
        assert_eq!(rhs.probe, Some(probe as u32));
    }
    for (logical, rhs) in receipt.rhs[37..].iter().enumerate() {
        assert_eq!(rhs.phase, GenericJlaRhsPhase::Target);
        assert_eq!(rhs.probe, Some((logical / 2) as u32));
        assert_eq!(
            rhs.side,
            if logical % 2 == 0 {
                GenericJlaRhsSide::Worker
            } else {
                GenericJlaRhsSide::Firm
            }
        );
    }
    let maximum_complete = receipt
        .rhs
        .iter()
        .map(|rhs| rhs.complete_residual)
        .fold(0.0_f64, f64::max);
    assert_eq!(receipt.maximum_complete_residual, maximum_complete);
    assert_eq!(
        receipt.control_rank.maximum_projection_residual,
        receipt
            .control_rank
            .projection_rhs
            .iter()
            .map(|rhs| rhs.complete_residual)
            .fold(0.0_f64, f64::max)
    );
    let native_payload = 1024_u64
        + expected as u64 * size_of::<GenericJlaRhsReceipt>() as u64
        + 32_u64 * size_of::<ControlProjectionReceipt>() as u64;
    assert_eq!(receipt.native_result_payload_bytes, native_payload);
    assert_eq!(
        receipt.result_transition_peak_forecast_bytes,
        native_payload
    );
    assert_eq!(receipt.result_export_peak_forecast_bytes, native_payload);
    assert_eq!(receipt.result_forecast_bytes, native_payload);
    assert_eq!(receipt.retained_mask_bytes, 0);
    assert_eq!(receipt.rhs_export_bytes, 0);
}

#[test]
fn row_order_and_physical_row_splitting_preserve_counter_results() {
    let original = fixture(true);
    let reversed = rebuild(&original, None, false, true);
    let relabelled = rebuild_order_preserving_relabel(&original, true);
    let split_row = original
        .frequency
        .iter()
        .position(|&value| value > 1)
        .expect("fixture has a splittable fweight");
    let split_match = rebuild(&original, Some(split_row), false, true);
    let split_observation = rebuild(&original, Some(split_row), true, true);
    for deletion in [DeletionMode::Match, DeletionMode::Observation] {
        let mut estimator_options = options(deletion, NuisanceMode::Joint);
        estimator_options.probes = 29;
        let baseline = run_generic_jla(&original, estimator_options).expect("baseline result");
        let reordered = run_generic_jla(&reversed, estimator_options).expect("reordered result");
        let relabelled_result =
            run_generic_jla(&relabelled, estimator_options).expect("relabelled result");
        let split = if deletion == DeletionMode::Match {
            &split_match
        } else {
            &split_observation
        };
        let physically_split = run_generic_jla(split, estimator_options).expect("split-row result");
        assert_counter_result_bits(&baseline, &reordered);
        assert_counter_result_bits(&baseline, &relabelled_result);
        assert_resource_receipt_eq(&baseline, &reordered);
        assert_resource_receipt_eq(&baseline, &relabelled_result);
        assert_eq!(
            baseline.receipt.execution.counter,
            physically_split.receipt.execution.counter
        );
        for (left, right) in components(baseline.correction)
            .into_iter()
            .zip(components(reordered.correction))
            .chain(
                components(baseline.correction)
                    .into_iter()
                    .zip(components(physically_split.correction)),
            )
            .chain(
                components(baseline.corrected)
                    .into_iter()
                    .zip(components(physically_split.corrected)),
            )
            .chain(
                components(baseline.numerical_mcse)
                    .into_iter()
                    .zip(components(physically_split.numerical_mcse)),
            )
        {
            assert_close(left, right, 2.0e-9);
        }
    }
}

#[test]
fn control_coordinate_changes_obey_certified_numerical_not_bitwise_contract() {
    let original = fixture(true);
    let transformed = transformed_control_fixture(&original);
    for deletion in [DeletionMode::Match, DeletionMode::Observation] {
        for nuisance in [NuisanceMode::Joint, NuisanceMode::FixedOffset] {
            let mut estimator_options = options(deletion, nuisance);
            estimator_options.probes = 31;
            let left = run_generic_jla(&original, estimator_options)
                .expect("original control coordinates accept");
            let right = run_generic_jla(&transformed, estimator_options)
                .expect("nonsingular transformed control coordinates accept");
            let tolerance = 1.0e-8_f64.max(
                100.0
                    * left
                        .receipt
                        .control_basis_forward_error
                        .max(right.receipt.control_basis_forward_error),
            );
            for (left, right) in components(left.plugin)
                .into_iter()
                .zip(components(right.plugin))
                .chain(
                    components(left.correction)
                        .into_iter()
                        .zip(components(right.correction)),
                )
                .chain(
                    components(left.corrected)
                        .into_iter()
                        .zip(components(right.corrected)),
                )
                .chain(
                    components(left.numerical_mcse)
                        .into_iter()
                        .zip(components(right.numerical_mcse)),
                )
            {
                assert_close(left, right, tolerance);
            }
        }
    }
}

#[test]
fn tied_coarse_rows_use_invariant_control_kernel_order() {
    let original = tied_control_fixture(false, false);
    let reversed = tied_control_fixture(true, false);
    let transformed = tied_control_fixture(true, true);
    for deletion in [DeletionMode::Match, DeletionMode::Observation] {
        let mut estimator_options = options(deletion, NuisanceMode::Joint);
        estimator_options.probes = 19;
        let baseline = run_generic_jla(&original, estimator_options)
            .expect("three distinct tied control rows accept");
        let reordered = run_generic_jla(&reversed, estimator_options)
            .expect("reversed tied control rows accept");
        assert_counter_result_bits(&baseline, &reordered);
        assert_resource_receipt_eq(&baseline, &reordered);

        let changed = run_generic_jla(&transformed, estimator_options)
            .expect("nonsingular transformed tied controls have the same acceptance status");
        let tolerance = 1.0e-8_f64.max(
            100.0
                * baseline
                    .receipt
                    .control_basis_forward_error
                    .max(changed.receipt.control_basis_forward_error),
        );
        for (left, right) in components(baseline.correction)
            .into_iter()
            .zip(components(changed.correction))
        {
            assert_close(left, right, tolerance);
        }
    }
}

#[test]
fn memory_admission_has_an_exact_reported_boundary_in_every_mode() {
    let problem = fixture(true);
    for deletion in [DeletionMode::Match, DeletionMode::Observation] {
        for nuisance in [NuisanceMode::Joint, NuisanceMode::FixedOffset] {
            let mut estimator_options = options(deletion, nuisance);
            estimator_options.probes = 17;
            let baseline =
                run_generic_jla(&problem, estimator_options).expect("unlimited memory result");
            let receipt = &baseline.receipt;
            assert_eq!(
                receipt.peak_forecast_bytes,
                [
                    receipt.canonicalization_peak_forecast_bytes,
                    receipt.fit_peak_forecast_bytes,
                    receipt.geometry_peak_forecast_bytes,
                    receipt.leverage_peak_forecast_bytes,
                    receipt.target_peak_forecast_bytes,
                    receipt.maker_peak_forecast_bytes,
                    receipt.result_forecast_bytes,
                ]
                .into_iter()
                .max()
                .expect("phase receipt")
            );
            estimator_options.memory_limit_bytes = receipt.peak_forecast_bytes;
            run_generic_jla(&problem, estimator_options).expect("exact forecast boundary accepts");
            estimator_options.memory_limit_bytes = receipt
                .peak_forecast_bytes
                .checked_sub(1)
                .expect("positive forecast");
            let error = run_generic_jla(&problem, estimator_options)
                .expect_err("one byte below forecast rejects");
            assert_eq!(error.code, ErrorCode::ResourceLimit);
            assert_eq!(error.phase, "generic_jla_memory");
        }
    }
}

#[test]
fn no_control_match_enforces_blocksize_limit() {
    let problem = match_block_fixture();
    let mut estimator_options = options(DeletionMode::Match, NuisanceMode::Joint);
    estimator_options.probes = 3;
    estimator_options.blocksize_limit = 1;
    let error = run_generic_jla(&problem, estimator_options)
        .expect_err("stored-row match block above limit rejects");
    assert_eq!(error.code, ErrorCode::ResourceLimit);
    assert_eq!(error.phase, "generic_jla_match_plan");
}

#[derive(Debug)]
struct BreakAfterPhase {
    target: &'static str,
    seen: usize,
    break_at: usize,
}

impl InterruptCheck for BreakAfterPhase {
    fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
        if phase == self.target {
            self.seen += 1;
            if self.seen == self.break_at {
                return Err(BackendError::new(
                    ErrorCode::UserBreak,
                    phase,
                    "delayed generic-JLA break",
                ));
            }
        }
        Ok(())
    }
}

fn assert_delayed_break(
    problem: &CompressedProblem,
    estimator_options: GenericJlaOptions,
    phase: &'static str,
) {
    let mut interrupt = BreakAfterPhase {
        target: phase,
        seen: 0,
        break_at: 2,
    };
    let error = match run_generic_jla_with_interrupt(problem, estimator_options, &mut interrupt) {
        Ok(_) => panic!(
            "phase {phase} completed after only {} observed checkpoints",
            interrupt.seen
        ),
        Err(error) => error,
    };
    assert_eq!(interrupt.seen, 2, "phase {phase} was not entered deeply");
    assert_eq!(error.code, ErrorCode::UserBreak);
    assert_eq!(error.phase, phase);
}

#[test]
fn every_major_generic_jla_phase_is_interruptible_after_entry() {
    let problem = fixture(true);
    let mut match_options = options(DeletionMode::Match, NuisanceMode::Joint);
    match_options.probes = 17;
    match_options.leverage_batch_width = 2;
    match_options.target_batch_width = 1;
    for phase in [
        "generic_jla_control_order",
        "generic_jla_rank_match_rows",
        "generic_jla_match_leverage_moments",
        "generic_jla_maker_assembly",
        "generic_jla_target_direction",
        "generic_jla_target_match_rows",
    ] {
        assert_delayed_break(&problem, match_options, phase);
    }

    let mut observation_options = options(DeletionMode::Observation, NuisanceMode::Joint);
    observation_options.probes = 17;
    observation_options.leverage_batch_width = 2;
    observation_options.target_batch_width = 1;
    for phase in [
        "generic_jla_observation_correlations",
        "generic_jla_observation_adjustment_copy",
        "generic_jla_target_observation",
    ] {
        assert_delayed_break(&problem, observation_options, phase);
    }

    let mut fixed_options = options(DeletionMode::Match, NuisanceMode::FixedOffset);
    fixed_options.probes = 17;
    fixed_options.target_batch_width = 1;
    assert_delayed_break(&problem, fixed_options, "generic_jla_transpose_outcome");
}

#[test]
fn private_component_attachment_cancels_atomically_during_probe_stream() {
    let problem = component_inference_fixture(false);
    let prepared = prepare_oracle_component_inference(
        &problem,
        &vec![0.002; problem.outcome.len()],
        ComponentInferenceOptions {
            probes: 32,
            batch_width: 5,
            ..ComponentInferenceOptions::default()
        },
    )
    .expect("private oracle-variance preparation");
    let mut interrupt = BreakAfterPhase {
        target: "component_inference_gaussian",
        seen: 0,
        break_at: 2,
    };
    let error = run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        &problem,
        routed_options(
            options(DeletionMode::Observation, NuisanceMode::Joint),
            ModelSolverRoute::Diagonal,
        ),
        None,
        Some(&prepared),
        None,
        &mut interrupt,
    )
    .expect_err("inference generation must not publish after cancellation");
    assert_eq!(interrupt.seen, 2);
    assert_eq!(error.code, ErrorCode::UserBreak);
    assert_eq!(error.phase, "component_inference_gaussian");
}

#[test]
fn grouped_component_attachment_cancels_and_null_signal_fails_closed() {
    let problem = grouped_component_inference_fixture(5);
    let prepared = prepare_grouped_oracle_component_inference(
        &problem,
        &vec![0.04; problem.deletion_units()],
        ComponentInferenceOptions {
            probes: 32,
            batch_width: 5,
            spectrum_probes: 16,
            spectrum_iterations: 16,
            spectrum_tolerance: 5.0e-2,
            ..ComponentInferenceOptions::default()
        },
    )
    .expect("grouped oracle preparation");
    let mut interrupt = BreakAfterPhase {
        target: "component_inference_gaussian",
        seen: 0,
        break_at: 2,
    };
    let error = run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        &problem,
        routed_options(
            options(DeletionMode::Match, NuisanceMode::FixedOffset),
            ModelSolverRoute::Diagonal,
        ),
        None,
        Some(&prepared),
        None,
        &mut interrupt,
    )
    .expect_err("grouped inference generation must not publish after cancellation");
    assert_eq!(interrupt.seen, 2);
    assert_eq!(error.code, ErrorCode::UserBreak);
    assert_eq!(error.phase, "component_inference_gaussian");

    let mut null_problem = problem.clone();
    null_problem.outcome.fill(0.0);
    null_problem.cell_outcome_sum.fill(0.0);
    let null_prepared = prepare_grouped_structured_component_inference(
        &null_problem,
        ComponentVarianceSource::StructuredCommon,
        ComponentInferenceOptions {
            probes: 32,
            batch_width: 5,
            spectrum_probes: 16,
            spectrum_iterations: 16,
            spectrum_tolerance: 5.0e-2,
            ..ComponentInferenceOptions::default()
        },
        StructuredVarianceOptions::default(),
    )
    .expect("null grouped preparation");
    let error = run_generic_jla_routed_with_attachments_and_hybrid_interrupt(
        &null_problem,
        routed_options(
            options(DeletionMode::Match, NuisanceMode::FixedOffset),
            ModelSolverRoute::Diagonal,
        ),
        None,
        Some(&null_prepared),
        None,
        &mut NeverInterrupt,
    )
    .expect_err("null grouped variance signal must fail closed");
    assert!(matches!(
        error.code,
        ErrorCode::SingularInformation | ErrorCode::JlaConstraintFailed
    ));
}

#[test]
fn memory_forecast_scan_is_deeply_interruptible() {
    let problem = large_interrupt_fixture(4_100);
    let mut estimator_options = options(DeletionMode::Match, NuisanceMode::Joint);
    estimator_options.probes = 3;
    assert_delayed_break(&problem, estimator_options, "generic_jla_memory_blocks");
}

#[derive(Debug)]
struct BatchForecastAudit {
    target: &'static str,
    memory_scan_checkpoints: usize,
    target_checkpoints: usize,
}

impl InterruptCheck for BatchForecastAudit {
    fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
        if phase == "generic_jla_memory_blocks" {
            self.memory_scan_checkpoints += 1;
            if self.memory_scan_checkpoints > 2 {
                return Err(BackendError::invariant(
                    phase,
                    "batch forecast repeated the precomputed deletion-block scan",
                ));
            }
        }
        if phase == self.target {
            self.target_checkpoints += 1;
            if self.target_checkpoints == 2 {
                return Err(BackendError::new(
                    ErrorCode::UserBreak,
                    phase,
                    "break after the width-one batch forecast",
                ));
            }
        }
        Ok(())
    }
}

#[test]
fn automatic_batch_ladders_reuse_one_interruptible_large_deletion_scan() {
    let problem = large_interrupt_fixture(4_100);
    let mut estimator = options(DeletionMode::Match, NuisanceMode::Joint);
    estimator.probes = 17;
    for target in [
        "generic_jla_batch_leverage_forecast",
        "generic_jla_batch_target_forecast",
    ] {
        let mut routed = routed_options(estimator, ModelSolverRoute::Auto);
        routed.leverage_batch = BatchRequest::Auto;
        routed.target_batch = BatchRequest::Auto;
        let mut interrupt = BatchForecastAudit {
            target,
            memory_scan_checkpoints: 0,
            target_checkpoints: 0,
        };
        let error = run_generic_jla_routed_with_interrupt(&problem, routed, &mut interrupt)
            .expect_err("second automatic batch candidate is interruptible");
        assert_eq!(error.code, ErrorCode::UserBreak);
        assert_eq!(error.phase, target);
        assert_eq!(interrupt.target_checkpoints, 2);
        assert_eq!(
            interrupt.memory_scan_checkpoints, 2,
            "the 4,100 deletion units must be scanned exactly once in two bounded chunks"
        );
    }
}

#[test]
fn resource_rank_and_interrupt_failures_are_typed() {
    let problem = fixture(true);
    let mut tiny = options(DeletionMode::Observation, NuisanceMode::Joint);
    tiny.memory_limit_bytes = 1;
    let error = run_generic_jla(&problem, tiny).expect_err("memory admission rejects");
    assert_eq!(error.code, ErrorCode::ResourceLimit);

    let mut overflow = options(DeletionMode::Match, NuisanceMode::Joint);
    overflow.prepared_persistent_bytes = u64::MAX;
    let overflow_error = run_generic_jla(&problem, overflow)
        .expect_err("public generic entry rejects an overflowing memory lifetime sum");
    assert_eq!(overflow_error.code, ErrorCode::ResourceLimit);
    assert_eq!(overflow_error.phase, "generic_jla");

    struct BreakAtEntry;
    impl InterruptCheck for BreakAtEntry {
        fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
            Err(BackendError::new(
                ErrorCode::UserBreak,
                phase,
                "injected generic-JLA break",
            ))
        }
    }
    let error = run_generic_jla_with_interrupt(
        &problem,
        options(DeletionMode::Match, NuisanceMode::Joint),
        &mut BreakAtEntry,
    )
    .expect_err("interrupt propagates");
    assert_eq!(error.code, ErrorCode::UserBreak);

    let mut rank_loss = fixture(true);
    rank_loss.controls[0].fill(0.0);
    let error = run_generic_jla(
        &rank_loss,
        options(DeletionMode::Match, NuisanceMode::Joint),
    )
    .expect_err("rank loss rejects");
    assert!(matches!(
        error.code,
        ErrorCode::SingularInformation
            | ErrorCode::AmbiguousControlBasis
            | ErrorCode::UnverifiedDeletionRank
    ));
}

#[test]
fn fixedoffset_and_joint_plugins_use_the_same_full_sample_nuisance_fit() {
    let problem = fixture(true);
    let joint = run_generic_jla(
        &problem,
        options(DeletionMode::Observation, NuisanceMode::Joint),
    )
    .expect("joint result");
    let fixed = run_generic_jla(
        &problem,
        options(DeletionMode::Observation, NuisanceMode::FixedOffset),
    )
    .expect("fixed-offset result");
    for (left, right) in components(joint.plugin)
        .into_iter()
        .zip(components(fixed.plugin))
    {
        assert_close(left, right, 1.0e-9);
    }
}

#[test]
fn single_cell_target_accepts_exact_zero_target_rhs() {
    let mut problem = fixture(true);
    problem.target_weight.fill(0.0);
    problem.target_weight[0] = 1.0;
    problem.target_total = 1.0;
    problem.cell_target_sum.fill(0.0);
    problem.cell_target_sum[problem.row_cell[0] as usize] = 1.0;
    let mut estimator_options = options(DeletionMode::Match, NuisanceMode::Joint);
    estimator_options.probes = 11;
    let result = run_generic_jla(&problem, estimator_options).expect("zero target RHS is valid");
    for value in components(result.correction)
        .into_iter()
        .chain(components(result.numerical_mcse))
    {
        assert_eq!(value.to_bits(), 0.0_f64.to_bits());
    }
}

#[test]
fn routed_diagonal_and_cmg_cover_control_and_estimator_modes() {
    for (controls, deletion, nuisance) in [
        (0, DeletionMode::Match, NuisanceMode::Joint),
        (1, DeletionMode::Observation, NuisanceMode::FixedOffset),
        (2, DeletionMode::Observation, NuisanceMode::Joint),
        (2, DeletionMode::Match, NuisanceMode::FixedOffset),
    ] {
        let mut problem = fixture(true);
        problem.controls.truncate(controls);
        let mut estimator = options(deletion, nuisance);
        estimator.probes = 17;
        estimator.leverage_batch_width = 4;
        estimator.target_batch_width = 5;
        let diagonal = run_generic_jla_routed(
            &problem,
            routed_options(estimator, ModelSolverRoute::Diagonal),
        )
        .expect("routed diagonal result");
        let cmg =
            run_generic_jla_routed(&problem, routed_options(estimator, ModelSolverRoute::Cmg))
                .expect("routed CMG result");
        assert_eq!(
            diagonal.receipt.execution.selected_route,
            ModelSolverRoute::Diagonal
        );
        assert_eq!(cmg.receipt.execution.selected_route, ModelSolverRoute::Cmg);
        assert!(cmg.receipt.execution.fe_hierarchy_reused);
        assert_eq!(cmg.receipt.execution.threads.used, 1);
        assert_eq!(cmg.receipt.execution.threads.parallel_regions, 0);
        assert!(cmg.receipt.execution.plan_frozen_before_rng);
        assert_eq!(cmg.receipt.execution.counter_atoms_before_plan_freeze, 0);
        for (left, right) in components(diagonal.correction)
            .into_iter()
            .zip(components(cmg.correction))
        {
            assert_close(left, right, 2.0e-8);
        }
        assert_close(diagonal.weighted_rss, cmg.weighted_rss, 2.0e-9);
    }

    let problem = maximum_control_fixture();
    let mut estimator = options(DeletionMode::Match, NuisanceMode::Joint);
    estimator.probes = 3;
    estimator.leverage_batch_width = 2;
    estimator.target_batch_width = 2;
    let result = run_generic_jla_routed(&problem, routed_options(estimator, ModelSolverRoute::Cmg))
        .expect("Q=32 routed CMG result");
    assert_eq!(result.receipt.control_rank.controls, 32);
    assert!(
        result
            .receipt
            .execution
            .memory
            .full_control_block_persistent_bytes
            > 0
    );
    assert_eq!(
        result.receipt.execution.memory.retained_nq_bytes,
        (problem.outcome.len() * 32 * size_of::<f64>()) as u64
    );
    assert_eq!(
        result.receipt.execution.memory.retained_q2_bytes,
        (32 * 32 * size_of::<f64>()) as u64
    );
}

#[test]
fn automatic_route_uses_the_versioned_firm_and_planned_rhs_policy() {
    let problem = structural_route_fixture(GENERIC_JLA_AUTO_FIRM_THRESHOLD_V1);
    let mut small_rhs = options(DeletionMode::Match, NuisanceMode::Joint);
    small_rhs.probes = 2;
    let diagonal =
        run_generic_jla_routed(&problem, routed_options(small_rhs, ModelSolverRoute::Auto))
            .expect("planned-RHS structural diagonal route");
    assert_eq!(
        diagonal.receipt.execution.planned_rhs,
        GENERIC_JLA_AUTO_PLANNED_RHS_THRESHOLD_V1 - 1
    );
    assert_eq!(
        diagonal.receipt.execution.selected_route,
        ModelSolverRoute::Diagonal
    );

    let mut cmg_rhs = small_rhs;
    cmg_rhs.probes = 3;
    let cmg = run_generic_jla_routed(&problem, routed_options(cmg_rhs, ModelSolverRoute::Auto))
        .expect("structural CMG route");
    assert!(cmg.receipt.execution.planned_rhs >= GENERIC_JLA_AUTO_PLANNED_RHS_THRESHOLD_V1);
    assert_eq!(cmg.receipt.execution.selected_route, ModelSolverRoute::Cmg);
    assert!(cmg.receipt.execution.fe_hierarchy_reused);

    let smaller = structural_route_fixture(GENERIC_JLA_AUTO_FIRM_THRESHOLD_V1 - 1);
    let small_firm =
        run_generic_jla_routed(&smaller, routed_options(cmg_rhs, ModelSolverRoute::Auto))
            .expect("firm-threshold structural diagonal route");
    assert_eq!(
        small_firm.receipt.execution.selected_route,
        ModelSolverRoute::Diagonal
    );
}

#[test]
fn automatic_cmg_fallback_is_setup_only_and_pre_rng() {
    let problem = structural_route_fixture(GENERIC_JLA_AUTO_FIRM_THRESHOLD_V1);
    let mut estimator = options(DeletionMode::Match, NuisanceMode::Joint);
    estimator.probes = 3;
    let mut automatic = routed_options(estimator, ModelSolverRoute::Auto);
    automatic.routing.cmg.memory_limit_bytes = 1;
    let fallback = run_generic_jla_routed(&problem, automatic)
        .expect("eligible automatic CMG setup failure falls back");
    assert_eq!(
        fallback.receipt.execution.selected_route,
        ModelSolverRoute::Diagonal
    );
    assert_eq!(
        fallback
            .receipt
            .execution
            .fallback
            .as_ref()
            .expect("fallback receipt")
            .code,
        ErrorCode::ResourceLimit
    );
    assert_eq!(
        fallback.receipt.execution.counter_atoms_before_plan_freeze,
        0
    );

    let mut forced = automatic;
    forced.routing.route = ModelSolverRoute::Cmg;
    let error =
        run_generic_jla_routed(&problem, forced).expect_err("forced CMG setup failure is terminal");
    assert_eq!(error.code, ErrorCode::ResourceLimit);

    let mut after_rng = routed_options(estimator, ModelSolverRoute::Auto);
    after_rng.leverage_batch = BatchRequest::Explicit(1);
    after_rng.target_batch = BatchRequest::Explicit(1);
    let mut interrupt = BreakAfterPhase {
        target: "generic_jla_target_batch",
        seen: 0,
        break_at: 1,
    };
    let error = run_generic_jla_routed_with_interrupt(&problem, after_rng, &mut interrupt)
        .expect_err("post-RNG interruption must be terminal");
    assert_eq!(error.code, ErrorCode::UserBreak);
    assert_eq!(error.phase, "generic_jla_target_batch");
}

#[test]
fn automatic_and_large_explicit_batches_preserve_counter_bits() {
    let problem = fixture(true);
    let mut estimator = options(DeletionMode::Observation, NuisanceMode::Joint);
    estimator.probes = 23;
    let mut scalar = routed_options(estimator, ModelSolverRoute::Diagonal);
    scalar.leverage_batch = BatchRequest::Explicit(1);
    scalar.target_batch = BatchRequest::Explicit(1);
    let left = run_generic_jla_routed(&problem, scalar).expect("scalar routed result");

    let mut automatic = scalar;
    automatic.leverage_batch = BatchRequest::Auto;
    automatic.target_batch = BatchRequest::Auto;
    let auto = run_generic_jla_routed(&problem, automatic).expect("automatic batch result");
    assert_counter_result_bits(&left, &auto);
    assert!(auto.receipt.execution.batch.leverage_active_width > 1);

    let mut oversized = scalar;
    oversized.leverage_batch = BatchRequest::Explicit(100);
    oversized.target_batch = BatchRequest::Explicit(101);
    let wide = run_generic_jla_routed(&problem, oversized).expect("large explicit batches");
    assert_counter_result_bits(&left, &wide);
    assert_eq!(
        wide.receipt.execution.batch.leverage_requested,
        BatchRequest::Explicit(100)
    );
    assert_eq!(
        wide.receipt.execution.batch.target_requested,
        BatchRequest::Explicit(101)
    );
    assert_eq!(wide.receipt.execution.batch.leverage_active_width, 23);
    assert_eq!(wide.receipt.execution.batch.target_active_width, 23);

    estimator.probes = 11;
    let mut cmg_scalar = routed_options(estimator, ModelSolverRoute::Cmg);
    cmg_scalar.leverage_batch = BatchRequest::Explicit(1);
    cmg_scalar.target_batch = BatchRequest::Explicit(1);
    let cmg_left = run_generic_jla_routed(&problem, cmg_scalar).expect("scalar CMG batches");
    let mut cmg_auto = cmg_scalar;
    cmg_auto.leverage_batch = BatchRequest::Auto;
    cmg_auto.target_batch = BatchRequest::Auto;
    let cmg_right = run_generic_jla_routed(&problem, cmg_auto).expect("auto CMG batches");
    assert_counter_result_bits(&cmg_left, &cmg_right);
}

#[test]
fn routed_memory_boundary_is_exact_and_auto_widths_are_monotone() {
    let problem = fixture(true);
    let mut estimator = options(DeletionMode::Match, NuisanceMode::FixedOffset);
    estimator.probes = 19;
    estimator.leverage_batch_width = 7;
    estimator.target_batch_width = 5;
    let configured = routed_options(estimator, ModelSolverRoute::Cmg);
    let baseline = run_generic_jla_routed(&problem, configured).expect("baseline memory plan");
    let peak = baseline.receipt.execution.memory.peak_bytes;
    assert_eq!(peak, baseline.receipt.peak_forecast_bytes);
    assert!(
        baseline
            .receipt
            .execution
            .memory
            .shared_cmg_persistent_bytes
            > 0
    );

    let mut exact = configured;
    exact.estimator.memory_limit_bytes = peak;
    run_generic_jla_routed(&problem, exact).expect("exact one-byte boundary accepts");
    exact.estimator.memory_limit_bytes = peak - 1;
    let error = run_generic_jla_routed(&problem, exact)
        .expect_err("one byte below an explicit plan rejects without shrinking");
    assert_eq!(error.code, ErrorCode::ResourceLimit);

    let mut high = configured;
    high.leverage_batch = BatchRequest::Auto;
    high.target_batch = BatchRequest::Auto;
    let high_result = run_generic_jla_routed(&problem, high).expect("high-memory auto plan");
    let batch = high_result.receipt.execution.batch.plan;
    let minimum_memory = batch
        .leverage
        .width_one_forecast_bytes
        .max(batch.target.width_one_forecast_bytes);
    let mut low = high;
    low.estimator.memory_limit_bytes = minimum_memory;
    let low_result = run_generic_jla_routed(&problem, low).expect("width-one auto plan");
    assert!(
        low_result.receipt.execution.batch.leverage_active_width
            <= high_result.receipt.execution.batch.leverage_active_width
    );
    assert!(
        low_result.receipt.execution.batch.target_active_width
            <= high_result.receipt.execution.batch.target_active_width
    );
}

#[test]
fn wall_advisory_and_relabeling_do_not_change_routed_estimates() {
    let problem = tied_control_fixture(false, false);
    let relabelled = rebuild_order_preserving_relabel(&problem, true);
    let mut estimator = options(DeletionMode::Match, NuisanceMode::Joint);
    estimator.probes = 19;
    let configured = routed_options(estimator, ModelSolverRoute::Cmg);
    let baseline = run_generic_jla_routed(&problem, configured).expect("CMG baseline");

    let mut with_wall = configured;
    with_wall.wallseconds = Some(0.25);
    let wall = run_generic_jla_routed(&problem, with_wall).expect("wall advisory result");
    assert_counter_result_bits(&baseline, &wall);
    assert_eq!(
        wall.receipt.execution.wall.status,
        WallAdvisoryStatus::Uncalibrated
    );
    assert_eq!(wall.receipt.execution.wall.requested_seconds, Some(0.25));
    assert_eq!(
        baseline.receipt.execution.batch,
        wall.receipt.execution.batch
    );

    let relabelled_result =
        run_generic_jla_routed(&relabelled, configured).expect("relabelled CMG result");
    assert_counter_result_bits(&baseline, &relabelled_result);
    assert_eq!(
        baseline.receipt.execution.selected_route,
        relabelled_result.receipt.execution.selected_route
    );
}

#[test]
fn observation_counter_receipt_separates_atoms_words_trials_and_evaluation_work() {
    let problem = fixture(false);
    let mut estimator = options(DeletionMode::Observation, NuisanceMode::Joint);
    estimator.probes = 7;
    estimator.leverage_batch_width = 3;
    estimator.target_batch_width = 4;
    let result = run_generic_jla(&problem, estimator).expect("observation accounting result");
    let receipt = result.receipt.execution;
    let probes = u64::from(estimator.probes);
    let leverage = receipt.counter.leverage;
    assert_eq!(
        leverage.planned_logical_atoms,
        probes * problem.physical_total
    );
    assert_eq!(
        leverage.planned_physical_bernoulli_trials,
        probes * problem.physical_total
    );
    // Every retained row in this fixture has a distinct observation-semantic
    // address, and its frequency is below one packed-word boundary.
    assert_eq!(
        leverage.planned_unique_packed_words,
        probes * problem.outcome.len() as u64
    );
    assert_eq!(
        leverage.planned_generator_word_evaluations,
        Some(2 * probes * problem.physical_total)
    );
    assert_eq!(
        leverage.actual_logical_atoms,
        leverage.planned_logical_atoms
    );
    assert_eq!(
        leverage.actual_unique_packed_words,
        leverage.planned_unique_packed_words
    );
    assert_eq!(
        leverage.actual_physical_bernoulli_trials,
        leverage.planned_physical_bernoulli_trials
    );
    assert_eq!(
        leverage.actual_generator_word_evaluations,
        leverage.planned_generator_word_evaluations
    );
    assert_eq!(receipt.counter_atoms_before_plan_freeze, 0);
    assert_eq!(receipt.unique_packed_words_before_plan_freeze, 0);
    assert_eq!(receipt.physical_trials_before_plan_freeze, 0);
}

#[test]
fn compressed_and_generic_match_results_and_counter_receipts_are_equal() {
    let problem = fixture(false);
    let probes = 5;
    let seed = 0x55aa_1122_3344_7788;
    let mut generic_options = options(DeletionMode::Match, NuisanceMode::Joint);
    generic_options.seed = seed;
    generic_options.probes = probes;
    generic_options.leverage_batch_width = 2;
    generic_options.target_batch_width = 3;
    let generic = run_generic_jla(&problem, generic_options).expect("generic match result");

    let compressed = run_jla_no_controls_planned(
        &problem,
        PlannedJlaEngineOptions {
            estimator: JlaEngineOptions {
                seed,
                probes,
                leverage_batch_width: 2,
                target_batch_width: 3,
                solver: LinearSolverOptions {
                    route: LinearSolverRoute::Exact,
                    exact_dimension_limit: 64,
                    ..LinearSolverOptions::default()
                },
                ..JlaEngineOptions::default()
            },
            leverage_batch: BatchRequest::Explicit(2),
            target_batch: BatchRequest::Explicit(3),
            wallseconds: None,
            full_cmg: None,
        },
    )
    .expect("compressed match result");
    assert_eq!(
        generic.receipt.execution.counter,
        compressed.execution.counter
    );
    for (generic_value, compressed_value) in components(generic.correction)
        .into_iter()
        .zip(components(compressed.estimator.correction))
    {
        assert_close(generic_value, compressed_value, 2e-9);
    }
    for (generic_value, compressed_value) in components(generic.corrected)
        .into_iter()
        .zip(components(compressed.estimator.corrected))
    {
        assert_close(generic_value, compressed_value, 2e-9);
    }

    let plan = JlaPlan::build_no_controls(&problem).expect("shared semantic plan");
    assert_eq!(
        compressed.execution.counter.leverage.planned_logical_atoms,
        u64::from(probes) * plan.deletion_units() as u64
    );
    assert_eq!(
        compressed.execution.counter.target.planned_logical_atoms,
        u64::from(probes) * plan.target_strata() as u64
    );
}
