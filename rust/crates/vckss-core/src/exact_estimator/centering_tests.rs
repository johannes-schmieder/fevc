// SPDX-License-Identifier: GPL-3.0-only
use super::{parallel, run_exact_estimator, ExactEstimatorOptions};
use crate::interrupt::NeverInterrupt;
use crate::jla::VarianceComponents;
use crate::problem::{CanonicalInput, CompressedProblem};
use crate::types::{Centering, DeletionMode, InputColumns, NuisanceMode};

fn fixture(weighted: bool, controls: bool) -> CompressedProblem {
    let mut columns = InputColumns {
        worker: vec![],
        firm: vec![],
        deletion: vec![],
        outcome: vec![],
        frequency: vec![],
        target_weight: vec![],
        controls: vec![],
    };
    let mut covariate = vec![];
    for worker in 0..4 {
        for firm in 0..3 {
            for repeat in 0..2 {
                let row = columns.outcome.len();
                let control = (0.73 * row as f64).sin() + 0.3 * repeat as f64;
                columns.worker.push(10 + worker);
                columns.firm.push(100 + firm);
                columns.deletion.push(1000 + 3 * worker + firm);
                columns.outcome.push(
                    3.2 + 0.7 * worker as f64 - 0.4 * firm as f64
                        + 0.6 * control
                        + (0.91 * row as f64).cos(),
                );
                columns
                    .frequency
                    .push(if weighted { 1 + row as u64 % 3 } else { 1 });
                columns
                    .target_weight
                    .push(0.5 + (row * 7 % 11) as f64 / 9.0);
                covariate.push(control);
            }
        }
    }
    if controls {
        columns.controls.push(covariate);
    }
    CanonicalInput::from_validated(columns.validate().unwrap())
        .unwrap()
        .compress(&[true; 24])
        .unwrap()
}

fn components(value: VarianceComponents) -> [f64; 4] {
    [value.worker, value.firm, value.covariance, value.total]
}

#[test]
fn exact_centering_serial_parallel_preserve_accounting() {
    for weighted in [false, true] {
        for (controls, nuisance) in [
            (false, NuisanceMode::Joint),
            (true, NuisanceMode::Joint),
            (true, NuisanceMode::FixedOffset),
        ] {
            let problem = fixture(weighted, controls);
            for deletion in [DeletionMode::Observation, DeletionMode::Match] {
                for centering in [Centering::None, Centering::Mean, Centering::Corrected] {
                    let options = ExactEstimatorOptions {
                        centering,
                        deletion,
                        nuisance,
                        ..ExactEstimatorOptions::default()
                    };
                    let serial = run_exact_estimator(&problem, options).unwrap();
                    let parallel = parallel::run_with_interrupt(
                        &problem,
                        options,
                        None,
                        3,
                        &mut NeverInterrupt,
                    )
                    .unwrap()
                    .estimator;
                    // Route parity protects the scalar primitive accumulator:
                    // Corrected must materialize total before its checked add.
                    // The unchanged Stata suite supplies the independent oracle.
                    for (serial, parallel) in [serial.plugin, serial.correction, serial.corrected]
                        .into_iter()
                        .zip([parallel.plugin, parallel.correction, parallel.corrected])
                    {
                        serial.verify_accounting(1.0e-12).unwrap();
                        parallel.verify_accounting(1.0e-12).unwrap();
                        for (actual, expected) in
                            components(serial).into_iter().zip(components(parallel))
                        {
                            assert!((actual - expected).abs() < 2.0e-9 * actual.abs().max(expected.abs()).max(1.0),
                                "weighted={weighted} controls={controls} {nuisance:?} {deletion:?} {centering:?}: {actual} vs {expected}");
                        }
                    }
                    assert!((serial.weighted_rss - parallel.weighted_rss).abs() < 1.0e-10);
                }
            }
        }
    }
}
