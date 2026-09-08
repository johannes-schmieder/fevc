// SPDX-License-Identifier: GPL-3.0-only

use super::*;
use crate::interrupt::NeverInterrupt;
use crate::model_operator::{CanonicalModelData, ModelRhs};
use crate::model_solver::{ModelSolverOptions, PreparedModelSolver};

fn addresses(n: usize) -> Vec<(u64, u64)> {
    (0..n).map(|i| ((i / 3) as u64, (i % 3) as u64)).collect()
}

fn zero_projection(
    _: &[f64],
    _: usize,
    out: &mut [f64],
    residual: &mut [f64],
    _: &mut dyn InterruptCheck,
) -> Result<()> {
    out.fill(0.0);
    residual.fill(0.0);
    Ok(())
}

fn dense_projection(
    p: &[f64],
    input: &[f64],
    columns: usize,
    output: &mut [f64],
    residual: &mut [f64],
    _: &mut dyn InterruptCheck,
) -> Result<()> {
    let n = input.len() / columns;
    for col in 0..columns {
        for row in 0..n {
            output[col * n + row] = (0..n).map(|j| p[row * n + j] * input[col * n + j]).sum();
        }
    }
    residual.fill(0.0);
    Ok(())
}

// Independent test-only Gauss--Jordan inverse, never the production SPD helper.
fn inverse(a: &[f64], n: usize) -> Vec<f64> {
    let mut aug = vec![0.0; n * 2 * n];
    for i in 0..n {
        aug[i * 2 * n..i * 2 * n + n].copy_from_slice(&a[i * n..(i + 1) * n]);
        aug[i * 2 * n + n + i] = 1.0;
    }
    for k in 0..n {
        let pivot = (k..n)
            .max_by(|&i, &j| {
                aug[i * 2 * n + k]
                    .abs()
                    .total_cmp(&aug[j * 2 * n + k].abs())
            })
            .unwrap();
        for j in 0..2 * n {
            aug.swap(k * 2 * n + j, pivot * 2 * n + j);
        }
        let scale = aug[k * 2 * n + k];
        assert!(scale.abs() > 1.0e-14);
        for j in 0..2 * n {
            aug[k * 2 * n + j] /= scale;
        }
        for i in 0..n {
            if i == k {
                continue;
            }
            let factor = aug[i * 2 * n + k];
            for j in 0..2 * n {
                aug[i * 2 * n + j] -= factor * aug[k * 2 * n + j];
            }
        }
    }
    (0..n)
        .flat_map(|i| aug[i * 2 * n + n..(i + 1) * 2 * n].to_vec())
        .collect()
}

fn exact_gram(z: &[f64], p: &[f64], n: usize, k: usize) -> Vec<f64> {
    let mut result = vec![0.0; k * k];
    for a in 0..k {
        for b in 0..k {
            for i in 0..n {
                for j in 0..n {
                    let m = f64::from(i == j) - p[i * n + j];
                    result[a * k + b] += z[i * k + a] * m * m * z[j * k + b];
                }
            }
        }
    }
    result
}

fn close(a: &[f64], b: &[f64], tolerance: f64) {
    assert_eq!(a.len(), b.len());
    for (i, (&x, &y)) in a.iter().zip(b).enumerate() {
        assert!(
            (x - y).abs() <= tolerance * y.abs().max(1.0),
            "entry {i}: {x} != {y}"
        );
    }
}

#[test]
fn direct_gram_matches_independent_same_probe_covariance() {
    let f = fixture(true);
    let n = f.h.len();
    let ids = addresses(n);
    for probes in [512, 1024, 2048] {
        let mut draws = Vec::new();
        let options = ResidualMomentOptions {
            probes,
            gram_method: GramMethod::DirectResidualCovariance,
            batch_width: 17,
            ..Default::default()
        };
        let actual = prepare_with_interrupt(
            &f.z,
            3,
            &f.h,
            &ids,
            options,
            |input, columns, output, certificates, interrupt| {
                dense_projection(&f.p, input, columns, output, certificates, interrupt)?;
                for col in 0..columns {
                    // Independent dense M multiplication and two-pass sample
                    // covariance: do not reuse the production streaming sums.
                    let mg = (0..n)
                        .map(|i| {
                            (0..n)
                                .map(|j| (f64::from(i == j) - f.p[i * n + j]) * input[col * n + j])
                                .sum::<f64>()
                        })
                        .collect::<Vec<_>>();
                    draws.push(
                        (0..3)
                            .map(|a| (0..n).map(|i| f.z[i * 3 + a] * mg[i] * mg[i]).sum::<f64>())
                            .collect::<Vec<_>>(),
                    );
                }
                Ok(())
            },
            &mut NeverInterrupt,
        )
        .unwrap();
        let means = (0..3)
            .map(|a| draws.iter().map(|s| s[a]).sum::<f64>() / probes as f64)
            .collect::<Vec<_>>();
        let mut expected = vec![0.0; 9];
        for a in 0..3 {
            for b in 0..3 {
                expected[a * 3 + b] = draws
                    .iter()
                    .map(|s| (s[a] - means[a]) * (s[b] - means[b]))
                    .sum::<f64>()
                    / (2 * (probes - 1)) as f64;
            }
        }
        close(actual.gram(), &expected, 1e-12);
        assert_eq!(actual.diagnostic.probes, probes);
        assert_eq!(actual.diagnostic.counter_atoms, (n * probes) as u64);
        assert_eq!(actual.diagnostic.counter_words, (2 * n * probes) as u64);
        // Approximate leverage still belongs to the basis and positivity
        // diagnostics, but must not enter an additional subtractive Gram term.
        let changed_h = vec![0.2; n];
        let batched = prepare_with_interrupt(
            &f.z,
            3,
            &changed_h,
            &ids,
            ResidualMomentOptions {
                batch_width: 1,
                ..options
            },
            |a, b, c, d, e| dense_projection(&f.p, a, b, c, d, e),
            &mut NeverInterrupt,
        )
        .unwrap();
        assert_eq!(actual.gram(), batched.gram());
    }
}

#[test]
fn zero_projection_matches_independent_ols_and_accounting() {
    let n = 30;
    let z = (0..n)
        .flat_map(|i| {
            [
                1.0,
                (i as f64 - 14.5) / 15.0,
                ((i * 7 % n) as f64 - 14.5) / 15.0,
            ]
        })
        .collect::<Vec<_>>();
    let h = vec![0.0; n];
    let ids = addresses(n);
    let e = (0..n)
        .map(|i| (0.7 + i as f64 / 20.0).sqrt())
        .collect::<Vec<_>>();
    let prepared = prepare_with_interrupt(
        &z,
        3,
        &h,
        &ids,
        ResidualMomentOptions::default(),
        zero_projection,
        &mut NeverInterrupt,
    )
    .unwrap();
    let k = exact_gram(&z, &vec![0.0; n * n], n, 3);
    close(prepared.gram(), &k, 1.0e-13);
    let fit = prepared
        .fit_with_interrupt(&e, &mut NeverInterrupt)
        .unwrap();
    let ki = inverse(&k, 3);
    let rhs = (0..3)
        .map(|a| (0..n).map(|i| z[i * 3 + a] * e[i] * e[i]).sum::<f64>())
        .collect::<Vec<_>>();
    let gamma = (0..3)
        .map(|a| (0..3).map(|b| ki[a * 3 + b] * rhs[b]).sum())
        .collect::<Vec<_>>();
    close(&fit.coefficients, &gamma, 1.0e-12);
    assert_eq!(prepared.diagnostic.counter_atoms, 512 * n as u64);
    assert_eq!(prepared.diagnostic.counter_words, 1024 * n as u64);
    assert_eq!(fit.floored_predictions, 0);
    assert!(fit.moment_relative_residual < 1.0e-13);
}

#[derive(Debug)]
struct Fixture {
    workers: Vec<u32>,
    firms: Vec<u32>,
    controls: Vec<Vec<f64>>,
    weight: Vec<f64>,
    x: Vec<f64>,
    p: Vec<f64>,
    h: Vec<f64>,
    z: Vec<f64>,
}

fn fixture(with_control: bool) -> Fixture {
    let (n, k) = (96, 7 + usize::from(with_control));
    let workers = (0..n).map(|i| ((i / 4) % 4) as u32).collect::<Vec<_>>();
    let firms = (0..n).map(|i| (i % 4) as u32).collect::<Vec<_>>();
    let controls: Vec<Vec<f64>> = if with_control {
        vec![(0..n).map(|i| (i as f64 * 1.7).sin()).collect()]
    } else {
        vec![]
    };
    let mut x = vec![0.0; n * k];
    for i in 0..n {
        x[i * k + workers[i] as usize] = 1.0;
        if firms[i] > 0 {
            x[i * k + 3 + firms[i] as usize] = 1.0;
        }
        if with_control {
            x[i * k + 7] = controls[0][i];
        }
    }
    let mut xtx = vec![0.0; k * k];
    for a in 0..k {
        for b in 0..k {
            xtx[a * k + b] = (0..n).map(|i| x[i * k + a] * x[i * k + b]).sum();
        }
    }
    let xi = inverse(&xtx, k);
    let mut p = vec![0.0; n * n];
    for i in 0..n {
        for j in 0..n {
            for a in 0..k {
                for b in 0..k {
                    p[i * n + j] += x[i * k + a] * xi[a * k + b] * x[j * k + b];
                }
            }
        }
    }
    let h = (0..n).map(|i| p[i * n + i]).collect();
    // A full-rank, fixed three-column basis; tests of the engine do not choose
    // data-dependent variance terms or reuse production moment construction.
    let z = (0..n)
        .flat_map(|i| [1.0, (i as f64 - 47.5) / 48.0, (i as f64 * 0.6).cos()])
        .collect();
    Fixture {
        workers,
        firms,
        controls,
        weight: vec![1.0; n],
        x,
        p,
        h,
        z,
    }
}

#[test]
fn exact_dense_identity_noiseless_recovery_and_constant_df() {
    let f = fixture(true);
    let n = f.h.len();
    let k = exact_gram(&f.z, &f.p, n, 3);
    let mut identity = vec![0.0; 9];
    for a in 0..3 {
        for b in 0..3 {
            for i in 0..n {
                identity[a * 3 + b] += f.z[i * 3 + a] * (1.0 - 2.0 * f.h[i]) * f.z[i * 3 + b];
                for j in 0..n {
                    identity[a * 3 + b] += f.z[i * 3 + a] * f.p[i * n + j].powi(2) * f.z[j * 3 + b];
                }
            }
        }
    }
    close(&identity, &k, 1.0e-12);
    let gamma = [1.0, 0.2, 0.15];
    let s = (0..n)
        .map(|i| (0..3).map(|a| f.z[i * 3 + a] * gamma[a]).sum::<f64>())
        .collect::<Vec<_>>();
    let expected_e2 = (0..n)
        .map(|i| {
            (0..n)
                .map(|j| (f64::from(i == j) - f.p[i * n + j]).powi(2) * s[j])
                .sum::<f64>()
        })
        .collect::<Vec<_>>();
    let rhs = (0..3)
        .map(|a| (0..n).map(|i| f.z[i * 3 + a] * expected_e2[i]).sum::<f64>())
        .collect::<Vec<_>>();
    let ki = inverse(&k, 3);
    let recovered = (0..3)
        .map(|a| (0..3).map(|b| ki[a * 3 + b] * rhs[b]).sum())
        .collect::<Vec<_>>();
    close(&recovered, &gamma, 1.0e-12);
    // Exercise the candidate's fit with exact K, not only oracle algebra.
    let ids = addresses(n);
    let mut prepared = prepare_with_interrupt(
        &f.z,
        3,
        &f.h,
        &ids,
        ResidualMomentOptions::default(),
        |a, b, c, d, e| dense_projection(&f.p, a, b, c, d, e),
        &mut NeverInterrupt,
    )
    .unwrap();
    prepared.gram = k;
    prepared.inverse = ki;
    let fit = prepared
        .fit_with_interrupt(
            &expected_e2.iter().map(|x| x.sqrt()).collect::<Vec<_>>(),
            &mut NeverInterrupt,
        )
        .unwrap();
    close(&fit.coefficients, &gamma, 1.0e-12);
    let ones = vec![1.0; n];
    let constant_k = exact_gram(&ones, &f.p, n, 1);
    close(&constant_k, &[(n - 8) as f64], 1.0e-12);
    let mut constant = prepare_with_interrupt(
        &ones,
        1,
        &f.h,
        &ids,
        ResidualMomentOptions::default(),
        |a, b, c, d, e| dense_projection(&f.p, a, b, c, d, e),
        &mut NeverInterrupt,
    )
    .unwrap();
    constant.gram = constant_k.clone();
    constant.inverse = vec![1.0 / constant_k[0]];
    let e = (0..n).map(|i| (i as f64).cos()).collect::<Vec<_>>();
    let fit = constant
        .fit_with_interrupt(&e, &mut NeverInterrupt)
        .unwrap();
    close(
        &fit.coefficients,
        &[e.iter().map(|x| x * x).sum::<f64>() / (n - 8) as f64],
        1.0e-12,
    );
}

#[test]
fn projected_counter_gram_batch_invariance_and_dense_accuracy() {
    let f = fixture(true);
    let ids = addresses(f.h.len());
    let options = ResidualMomentOptions {
        probes: 4096,
        batch_width: 1,
        ..Default::default()
    };
    let a = prepare_with_interrupt(
        &f.z,
        3,
        &f.h,
        &ids,
        options,
        |a, b, c, d, e| dense_projection(&f.p, a, b, c, d, e),
        &mut NeverInterrupt,
    )
    .unwrap();
    let b = prepare_with_interrupt(
        &f.z,
        3,
        &f.h,
        &ids,
        ResidualMomentOptions {
            batch_width: 17,
            ..options
        },
        |a, b, c, d, e| dense_projection(&f.p, a, b, c, d, e),
        &mut NeverInterrupt,
    )
    .unwrap();
    assert_eq!(a.gram, b.gram);
    assert_eq!(a.inverse, b.inverse);
    let k = exact_gram(&f.z, &f.p, f.h.len(), 3);
    for i in 0..3 {
        for j in 0..3 {
            assert!(
                (a.gram[i * 3 + j] - k[i * 3 + j]).abs() / (k[i * 3 + i] * k[j * 3 + j]).sqrt()
                    < 0.1
            );
        }
    }
}

#[test]
fn full_quotient_solver_projection_matches_dense_and_signal_shift_is_invariant() {
    for with_control in [false, true] {
        let f = fixture(with_control);
        let n = f.h.len();
        let ids = addresses(n);
        let data = CanonicalModelData {
            workers: 4,
            firms: 4,
            row_worker: &f.workers,
            row_firm: &f.firms,
            weight: &f.weight,
            controls: &f.controls,
        };
        let solver = PreparedModelSolver::prepare(data, ModelSolverOptions::default()).unwrap();
        let options = ResidualMomentOptions {
            probes: 128,
            batch_width: 7,
            projection_workspace_bytes: 1024 * 1024,
            ..Default::default()
        };
        let actual = prepare_with_interrupt(
            &f.z,
            3,
            &f.h,
            &ids,
            options,
            |input, columns, output, certificates, interrupt| {
                for col in 0..columns {
                    let mut wr = vec![0.0; 4];
                    let mut fr = vec![0.0; 4];
                    let mut cr = vec![0.0; f.controls.len()];
                    for i in 0..n {
                        wr[f.workers[i] as usize] += input[col * n + i];
                        fr[f.firms[i] as usize] += input[col * n + i];
                        for q in 0..cr.len() {
                            cr[q] += f.controls[q][i] * input[col * n + i];
                        }
                    }
                    let solved = solver.solve_with_interrupt(
                        ModelRhs {
                            worker: &wr,
                            firm: &fr,
                            control: &cr,
                        },
                        interrupt,
                    )?;
                    certificates[col] = solved.receipt.full_residual;
                    solver.operator().predict_into_with_interrupt(
                        &solved.coefficients.worker,
                        &solved.coefficients.firm,
                        &solved.coefficients.control,
                        &mut output[col * n..(col + 1) * n],
                        interrupt,
                    )?;
                    assert!(solved.coefficients.firm.iter().sum::<f64>().abs() < 1.0e-10);
                }
                Ok(())
            },
            &mut NeverInterrupt,
        )
        .unwrap();
        let oracle = prepare_with_interrupt(
            &f.z,
            3,
            &f.h,
            &ids,
            options,
            |a, b, c, d, e| dense_projection(&f.p, a, b, c, d, e),
            &mut NeverInterrupt,
        )
        .unwrap();
        close(actual.gram(), oracle.gram(), 1.0e-8);
        assert!(actual.diagnostic.maximum_full_residual < actual.diagnostic.full_residual_gate);
        let y = (0..n).map(|i| (i as f64 * 0.73).cos()).collect::<Vec<_>>();
        let parameters = f.x.len() / n;
        let shifted = (0..n)
            .map(|i| {
                y[i] + (0..parameters)
                    .map(|a| f.x[i * parameters + a] * (a + 1) as f64 * 4.0)
                    .sum::<f64>()
            })
            .collect::<Vec<_>>();
        let residual = |y: &[f64]| {
            (0..n)
                .map(|i| y[i] - (0..n).map(|j| f.p[i * n + j] * y[j]).sum::<f64>())
                .collect::<Vec<_>>()
        };
        let e = residual(&y);
        let shifted_e = residual(&shifted);
        let fit = actual.fit_with_interrupt(&e, &mut NeverInterrupt).unwrap();
        let shifted_fit = actual
            .fit_with_interrupt(&shifted_e, &mut NeverInterrupt)
            .unwrap();
        let oracle_fit = oracle.fit_with_interrupt(&e, &mut NeverInterrupt).unwrap();
        close(&fit.raw_variance, &shifted_fit.raw_variance, 1.0e-10);
        close(&fit.raw_variance, &oracle_fit.raw_variance, 1.0e-8);
    }
}

#[test]
fn input_rank_address_and_memory_errors_are_pre_rng() {
    let n = 30;
    let h = vec![0.0; n];
    let ids = addresses(n);
    let z = vec![1.0; n];
    let options = ResidualMomentOptions::default();
    let plan = memory_plan(n, 1, options).unwrap();
    let boundary = ResidualMomentOptions {
        additional_memory_limit_bytes: plan.additional_peak_bytes,
        ..options
    };
    assert!(prepare_with_interrupt(
        &z,
        1,
        &h,
        &ids,
        boundary,
        zero_projection,
        &mut NeverInterrupt
    )
    .is_ok());
    let no_project = |_: &[f64],
                      _: usize,
                      _: &mut [f64],
                      _: &mut [f64],
                      _: &mut dyn InterruptCheck|
     -> Result<()> {
        panic!("must reject before projection/RNG");
    };
    assert_eq!(
        prepare_with_interrupt(
            &z,
            1,
            &h,
            &ids,
            ResidualMomentOptions {
                additional_memory_limit_bytes: plan.additional_peak_bytes - 1,
                ..options
            },
            no_project,
            &mut NeverInterrupt
        )
        .unwrap_err()
        .code,
        ErrorCode::ResourceLimit
    );
    let duplicate_z = vec![1.0; n * 2];
    assert_eq!(
        prepare_with_interrupt(
            &duplicate_z,
            2,
            &h,
            &ids,
            options,
            no_project,
            &mut NeverInterrupt
        )
        .unwrap_err()
        .code,
        ErrorCode::SingularInformation
    );
    let duplicate_ids = vec![(0, 0); n];
    assert_eq!(
        prepare_with_interrupt(
            &z,
            1,
            &h,
            &duplicate_ids,
            options,
            no_project,
            &mut NeverInterrupt
        )
        .unwrap_err()
        .code,
        ErrorCode::RngContractFailed
    );
    let mut capped = ids.clone();
    capped[n - 1].1 = MAX_PHYSICAL_WORDS_PER_ATOM / 2;
    assert_eq!(
        prepare_with_interrupt(&z, 1, &h, &capped, options, no_project, &mut NeverInterrupt)
            .unwrap_err()
            .code,
        ErrorCode::RngContractFailed
    );
    for bad in [f64::NAN, -0.1, 1.0, f64::INFINITY] {
        let mut bad_h = h.clone();
        bad_h[0] = bad;
        assert_eq!(
            prepare_with_interrupt(
                &z,
                1,
                &bad_h,
                &ids,
                options,
                no_project,
                &mut NeverInterrupt
            )
            .unwrap_err()
            .code,
            ErrorCode::InvalidInput
        );
    }
    assert!(memory_plan(
        1_usize << 52,
        15,
        ResidualMomentOptions {
            additional_memory_limit_bytes: usize::MAX,
            probes: 1_usize << 52,
            ..options
        }
    )
    .is_err());
    assert!(memory_plan(
        n,
        1,
        ResidualMomentOptions {
            projection_workspace_bytes: usize::MAX,
            ..options
        }
    )
    .is_err());
}

#[test]
fn failed_missing_or_nonfinite_projection_is_not_used() {
    let h = vec![0.0; 10];
    let z = vec![1.0; 10];
    let ids = addresses(10);
    for bad in [f64::NAN, f64::INFINITY, -1.0, 1.0] {
        let e = prepare_with_interrupt(
            &z,
            1,
            &h,
            &ids,
            ResidualMomentOptions::default(),
            |_, _, out, cert, _| {
                out.fill(0.0);
                cert.fill(bad);
                Ok(())
            },
            &mut NeverInterrupt,
        )
        .unwrap_err();
        assert_eq!(e.code, ErrorCode::FullResidualFailed);
    }
    let e = prepare_with_interrupt(
        &z,
        1,
        &h,
        &ids,
        ResidualMomentOptions::default(),
        |_, _, _, cert, _| {
            cert.fill(0.0);
            Ok(())
        },
        &mut NeverInterrupt,
    )
    .unwrap_err();
    assert_eq!(e.code, ErrorCode::CorrectionNonFinite);
    let e = prepare_with_interrupt(
        &z,
        1,
        &h,
        &ids,
        ResidualMomentOptions::default(),
        |_, _, _, _, _| {
            Err(BackendError::new(
                ErrorCode::PcgMaxIterations,
                "injected",
                "no convergence",
            ))
        },
        &mut NeverInterrupt,
    )
    .unwrap_err();
    assert_eq!(e.code, ErrorCode::PcgMaxIterations);
}

#[test]
fn positivity_counts_invalid_outcomes_and_entire_floor_fail_closed() {
    let n = 30;
    let z = (0..n)
        .flat_map(|i| [1.0, i as f64 / 29.0])
        .collect::<Vec<_>>();
    let h = vec![0.0; n];
    let ids = addresses(n);
    let mut prepared = prepare_with_interrupt(
        &z,
        2,
        &h,
        &ids,
        ResidualMomentOptions::default(),
        zero_projection,
        &mut NeverInterrupt,
    )
    .unwrap();
    let e = (0..n)
        .map(|i| if i == 29 { 10.0 } else { 0.1 })
        .collect::<Vec<_>>();
    let fit = prepared
        .fit_with_interrupt(&e, &mut NeverInterrupt)
        .unwrap();
    assert!(fit.nonpositive_predictions > 0);
    assert_eq!(
        fit.nonpositive_predictions,
        fit.raw_variance.iter().filter(|&&x| x <= 0.0).count()
    );
    assert_eq!(
        fit.floored_predictions,
        fit.raw_variance
            .iter()
            .filter(|&&x| x < fit.positivity_floor)
            .count()
    );
    assert!(fit
        .positive_variance
        .iter()
        .all(|&x| x >= fit.positivity_floor));
    for bad in [vec![0.0; n], vec![f64::NAN; n], vec![f64::MAX; n]] {
        assert_eq!(
            prepared
                .fit_with_interrupt(&bad, &mut NeverInterrupt)
                .unwrap_err()
                .code,
            ErrorCode::CorrectionNonFinite
        );
    }
    assert_eq!(
        prepared
            .fit_with_interrupt(&[1.0], &mut NeverInterrupt)
            .unwrap_err()
            .code,
        ErrorCode::InvalidInput
    );
    // Inject a corrupted inverse and matching Gram to exercise the final
    // all-floored fence separately from the normal SPD preparation rejection.
    prepared.inverse.iter_mut().for_each(|x| *x = -*x);
    prepared.gram.iter_mut().for_each(|x| *x = -*x);
    let error = prepared
        .fit_with_interrupt(&vec![1.0; n], &mut NeverInterrupt)
        .unwrap_err();
    assert!(error.message.contains("entire variance fit"));
}

#[test]
fn high_leverage_does_not_imply_rejection_but_indefinite_gram_does() {
    let n = 10;
    let mut p = vec![0.0; n * n];
    p[0] = 0.8;
    p[1] = 0.4;
    p[n] = 0.4;
    p[n + 1] = 0.2;
    let h = (0..n).map(|i| p[i * n + i]).collect::<Vec<_>>();
    let z = vec![1.0; n];
    let ids = addresses(n);
    let prepared = prepare_with_interrupt(
        &z,
        1,
        &h,
        &ids,
        ResidualMomentOptions::default(),
        |a, b, c, d, e| dense_projection(&p, a, b, c, d, e),
        &mut NeverInterrupt,
    )
    .unwrap();
    assert_eq!(prepared.diagnostic.maximum_leverage, 0.8);
    let false_h = vec![0.9; n];
    let err = prepare_with_interrupt(
        &z,
        1,
        &false_h,
        &ids,
        ResidualMomentOptions::default(),
        zero_projection,
        &mut NeverInterrupt,
    )
    .unwrap_err();
    assert_eq!(err.code, ErrorCode::SingularInformation);
}

#[derive(Debug)]
struct BreakPhase(&'static str);
impl InterruptCheck for BreakPhase {
    fn checkpoint(&mut self, phase: &'static str) -> Result<()> {
        if phase == self.0 {
            Err(BackendError::new(
                ErrorCode::UserBreak,
                phase,
                "injected interruption",
            ))
        } else {
            Ok(())
        }
    }
}
#[test]
fn cancellation_during_preparation_probes_and_fit_is_typed() {
    let n = 30;
    let z = vec![1.0; n];
    let h = vec![0.0; n];
    let ids = addresses(n);
    for phase in [PHASE, "observation_residual_moment_probes"] {
        let err = prepare_with_interrupt(
            &z,
            1,
            &h,
            &ids,
            ResidualMomentOptions::default(),
            zero_projection,
            &mut BreakPhase(phase),
        )
        .unwrap_err();
        assert_eq!(err.code, ErrorCode::UserBreak);
    }
    let prepared = prepare_with_interrupt(
        &z,
        1,
        &h,
        &ids,
        ResidualMomentOptions::default(),
        zero_projection,
        &mut NeverInterrupt,
    )
    .unwrap();
    assert_eq!(
        prepared
            .fit_with_interrupt(&vec![1.0; n], &mut BreakPhase(PHASE))
            .unwrap_err()
            .code,
        ErrorCode::UserBreak
    );
}

#[test]
fn centered_covariance_is_stable_under_large_offsets() {
    let mut base = CenteredMoments::new(2);
    let mut shifted = CenteredMoments::new(2);
    for i in 0..1000 {
        let mut a = [Sum::default(); 2];
        let mut b = a;
        for j in 0..2 {
            a[j].add(((i * (j + 1)) % 17) as f64);
            b[j].add(a[j].value() + 1.0e9);
        }
        base.push(&a).unwrap();
        shifted.push(&b).unwrap();
    }
    close(
        &base.cross.iter().map(|x| x.value()).collect::<Vec<_>>(),
        &shifted.cross.iter().map(|x| x.value()).collect::<Vec<_>>(),
        1.0e-7,
    );
}

#[test]
fn ill_conditioned_basis_and_malformed_options_are_rejected() {
    let n = 30;
    let z = (0..n)
        .flat_map(|i| [1.0, 1.0 + 1.0e-7 * i as f64])
        .collect::<Vec<_>>();
    let h = vec![0.0; n];
    let ids = addresses(n);
    let no_project = |_: &[f64],
                      _: usize,
                      _: &mut [f64],
                      _: &mut [f64],
                      _: &mut dyn InterruptCheck|
     -> Result<()> {
        panic!("invalid request reached projection");
    };
    let error = prepare_with_interrupt(
        &z,
        2,
        &h,
        &ids,
        ResidualMomentOptions::default(),
        no_project,
        &mut NeverInterrupt,
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::SingularInformation);
    for options in [
        ResidualMomentOptions {
            probes: 1,
            ..Default::default()
        },
        ResidualMomentOptions {
            batch_width: 0,
            ..Default::default()
        },
        ResidualMomentOptions {
            rank_tolerance: f64::NAN,
            ..Default::default()
        },
        ResidualMomentOptions {
            positivity_multiplier: 0.0,
            ..Default::default()
        },
        ResidualMomentOptions {
            positivity_multiplier: f64::INFINITY,
            ..Default::default()
        },
        ResidualMomentOptions {
            effective_projection_tolerance: 0.0,
            ..Default::default()
        },
    ] {
        assert_eq!(
            memory_plan(n, 1, options).unwrap_err().code,
            ErrorCode::InvalidInput
        );
    }
    let mut ones = vec![1.0; n];
    for value in [0.0, f64::NAN, f64::INFINITY] {
        ones[0] = value;
        assert_eq!(
            prepare_with_interrupt(
                &ones,
                1,
                &h,
                &ids,
                ResidualMomentOptions::default(),
                no_project,
                &mut NeverInterrupt
            )
            .unwrap_err()
            .code,
            ErrorCode::InvalidInput
        );
    }
}

#[test]
fn gaussian_semantic_subdraws_and_prefixes_are_fixed() {
    let rng = CounterRng::new(123);
    let a = normal(rng, 5, 7, 0);
    let b = normal(rng, 5, 7, 1);
    assert!(a.is_finite() && b.is_finite());
    assert_ne!(a, b);
    assert_eq!(a, normal(rng, 5, 7, 0));
    let tags = [
        ProbeDomain::Leverage,
        ProbeDomain::Target,
        ProbeDomain::ComponentInference,
        ProbeDomain::ComponentInferenceCritical,
        ProbeDomain::ComponentVarianceFold,
        ProbeDomain::ObservationResidualMoments,
        ProbeDomain::Diagnostic,
        ProbeDomain::Retry,
        ProbeDomain::SelfTest,
    ];
    for i in 0..tags.len() {
        for j in i + 1..tags.len() {
            assert_ne!(tags[i].tag(), tags[j].tag());
        }
    }
    let n = 10;
    let z = vec![1.0; n];
    let h = vec![0.0; n];
    let ids = addresses(n);
    let mut prefix = Vec::new();
    prepare_with_interrupt(
        &z,
        1,
        &h,
        &ids,
        ResidualMomentOptions {
            probes: 3,
            batch_width: 2,
            ..Default::default()
        },
        |input, columns, output, certificates, interrupt| {
            prefix.extend_from_slice(input);
            zero_projection(input, columns, output, certificates, interrupt)
        },
        &mut NeverInterrupt,
    )
    .unwrap();
    let mut extended = Vec::new();
    prepare_with_interrupt(
        &z,
        1,
        &h,
        &ids,
        ResidualMomentOptions {
            probes: 9,
            batch_width: 7,
            ..Default::default()
        },
        |input, columns, output, certificates, interrupt| {
            extended.extend_from_slice(input);
            zero_projection(input, columns, output, certificates, interrupt)
        },
        &mut NeverInterrupt,
    )
    .unwrap();
    assert_eq!(prefix, extended[..prefix.len()]);
}
