// SPDX-License-Identifier: MIT

use super::*;
use crate::interrupt::NeverInterrupt;

fn splitmix(state: &mut u64) -> f64 {
    *state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
    let mut z = *state;
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
    ((z ^ (z >> 31)) >> 11) as f64 / (1_u64 << 53) as f64
}

fn normal(state: &mut u64) -> f64 {
    let u = splitmix(state).max(f64::MIN_POSITIVE);
    let v = splitmix(state);
    (-2.0 * u.ln()).sqrt() * (2.0 * core::f64::consts::PI * v).cos()
}

#[test]
fn working_variance_recovers_a_function_in_its_span_by_type() {
    let movers = 600;
    let stayers = 300;
    let rows = movers + stayers;
    let mut state = 7;
    let leverage = (0..rows)
        .map(|_| 0.1 + 0.8 * splitmix(&mut state))
        .collect::<Vec<_>>();
    let mass = (0..rows)
        .map(|row| {
            if row < movers {
                1.0 + (row % 6) as f64
            } else {
                1.0
            }
        })
        .collect::<Vec<_>>();
    let lever_rank = [
        normalized_midranks(&leverage[..movers], &mut NeverInterrupt).unwrap(),
        normalized_midranks(&leverage[movers..], &mut NeverInterrupt).unwrap(),
    ];
    let mass_rank = normalized_midranks(&mass[..movers], &mut NeverInterrupt).unwrap();
    let truth = (0..rows)
        .map(|row| {
            if row < movers {
                1.0 + 0.3 * lever_rank[0][row] + 0.2 * mass_rank[row] * mass_rank[row]
            } else {
                0.5 - 0.1 * lever_rank[1][row - movers]
            }
        })
        .collect::<Vec<_>>();
    let residual = vec![1.0; rows];
    let maker = vec![2.0; rows];
    let fit = fit_working_variance(
        &truth,
        &residual,
        &maker,
        &leverage,
        Some(&mass),
        movers,
        1.0e-8,
        &mut NeverInterrupt,
    )
    .unwrap();
    assert_eq!(fit.terms, [5, 3]);
    assert_eq!(fit.floored, 0);
    for row in 0..rows {
        assert!(
            (fit.variance[row] - truth[row]).abs() < 1.0e-10,
            "row {row}"
        );
    }
}

#[test]
fn working_variance_shortens_small_types_and_floors_negative_fits() {
    let rows = 150;
    let leverage = (0..rows)
        .map(|row| row as f64 / rows as f64)
        .collect::<Vec<_>>();
    // One permitted term: the fit is the mean of the leave-out products.
    let proxy = (0..rows)
        .map(|row| if row % 2 == 0 { 3.0 } else { 1.0 })
        .collect::<Vec<_>>();
    let residual = vec![1.0; rows];
    let maker = vec![1.0; rows];
    let fit = fit_working_variance(
        &proxy,
        &residual,
        &maker,
        &leverage,
        None,
        rows,
        1.0e-8,
        &mut NeverInterrupt,
    )
    .unwrap();
    assert_eq!(fit.terms, [1, 0]);
    assert!(fit
        .variance
        .iter()
        .all(|value| (value - 2.0).abs() < 1.0e-12));

    let rows = 400;
    let leverage = (0..rows).map(|row| row as f64).collect::<Vec<_>>();
    let rank = normalized_midranks(&leverage, &mut NeverInterrupt).unwrap();
    let proxy = rank.iter().map(|value| 0.2 + value).collect::<Vec<_>>();
    let fit = fit_working_variance(
        &proxy,
        &vec![1.0; rows],
        &vec![1.0; rows],
        &leverage,
        None,
        rows,
        1.0e-3,
        &mut NeverInterrupt,
    )
    .unwrap();
    assert!((fit.floor - 1.0e-3).abs() < 1.0e-15);
    assert!(fit.floored > 0 && fit.floored < rows as u64);
    assert!(fit.variance.iter().all(|value| *value >= fit.floor));
    let all_negative = vec![-1.0; rows];
    assert!(fit_working_variance(
        &all_negative,
        &vec![1.0; rows],
        &vec![1.0; rows],
        &leverage,
        None,
        rows,
        1.0e-3,
        &mut NeverInterrupt,
    )
    .is_err());
}

#[test]
fn centering_removes_the_collapsed_mean_direction() {
    let mass = [4.0, 1.0, 9.0];
    // A common physical shift c appears as c * sqrt(mass) after collapsing.
    let mut values = [2.0 * 2.0 + 0.5, 2.0 * 1.0 - 0.25, 2.0 * 3.0];
    let mut expected = [0.5, -0.25, 0.0];
    center_units(&mut expected, Some(&mass), &mut NeverInterrupt).unwrap();
    center_units(&mut values, Some(&mass), &mut NeverInterrupt).unwrap();
    for index in 0..3 {
        assert!((values[index] - expected[index]).abs() < 1.0e-14);
    }
    let projection = values
        .iter()
        .zip(mass)
        .map(|(v, m)| v * m.sqrt())
        .sum::<f64>();
    assert!(projection.abs() < 1.0e-13);
}

/// A small symmetric kernel with zero diagonal and an idempotent residual maker.
fn dense_design(rows: usize) -> (Vec<Vec<f64>>, [Vec<Vec<f64>>; PRIMITIVE_TARGETS]) {
    let mut state = 11;
    let columns = 3;
    let x = (0..rows)
        .map(|_| (0..columns).map(|_| normal(&mut state)).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let mut gram = vec![vec![0.0; columns]; columns];
    for row in &x {
        for i in 0..columns {
            for j in 0..columns {
                gram[i][j] += row[i] * row[j];
            }
        }
    }
    // Gauss-Jordan inverse of a well-conditioned 3x3 Gram matrix.
    let mut inverse = vec![vec![0.0; columns]; columns];
    for i in 0..columns {
        inverse[i][i] = 1.0;
    }
    for pivot in 0..columns {
        let scale = gram[pivot][pivot];
        for j in 0..columns {
            gram[pivot][j] /= scale;
            inverse[pivot][j] /= scale;
        }
        for i in (0..columns).filter(|&i| i != pivot) {
            let factor = gram[i][pivot];
            for j in 0..columns {
                gram[i][j] -= factor * gram[pivot][j];
                inverse[i][j] -= factor * inverse[pivot][j];
            }
        }
    }
    let mut maker = vec![vec![0.0; rows]; rows];
    for a in 0..rows {
        for b in 0..rows {
            let mut value = 0.0;
            for i in 0..columns {
                for j in 0..columns {
                    value += x[a][i] * inverse[i][j] * x[b][j];
                }
            }
            maker[a][b] = f64::from(u8::from(a == b)) - value;
        }
    }
    let kernel = core::array::from_fn(|target| {
        let mut matrix = vec![vec![0.0; rows]; rows];
        for a in 0..rows {
            for b in a + 1..rows {
                let value = ((a * 7 + b * 3 + target * 5) % 11) as f64 / 11.0 - 0.4;
                matrix[a][b] = value / rows as f64;
                matrix[b][a] = matrix[a][b];
            }
        }
        matrix
    });
    (maker, kernel)
}

fn apply(matrix: &[Vec<f64>], vector: &[f64]) -> Vec<f64> {
    matrix
        .iter()
        .map(|row| row.iter().zip(vector).map(|(a, b)| a * b).sum())
        .collect()
}

#[test]
fn pseudo_noise_moments_match_closed_forms() {
    let rows = 40;
    let (maker, kernel) = dense_design(rows);
    let variance = (0..rows)
        .map(|row| 0.5 + (row % 5) as f64 * 0.2)
        .collect::<Vec<_>>();
    let weight = (0..rows)
        .map(|row| 1.0 / maker[row][row])
        .collect::<Vec<_>>();
    // Closed forms: tr(K_t V K_s V) and sum_a w_a [E_a c_a + 2 k_ta k_sa].
    let mut trace = [0.0; SQUARE];
    let mut noise = [0.0; SQUARE];
    for left in 0..PRIMITIVE_TARGETS {
        for right in 0..PRIMITIVE_TARGETS {
            for a in 0..rows {
                let mut expected_square = 0.0;
                let mut cross = 0.0;
                let mut covariance = [0.0; 2];
                for b in 0..rows {
                    expected_square += maker[a][b] * maker[a][b] * variance[b];
                    cross += kernel[left][a][b] * kernel[right][a][b] * variance[b];
                    covariance[0] += kernel[left][a][b] * maker[a][b] * variance[b];
                    covariance[1] += kernel[right][a][b] * maker[a][b] * variance[b];
                    trace[left * PRIMITIVE_TARGETS + right] +=
                        kernel[left][a][b] * variance[b] * kernel[right][b][a] * variance[a];
                }
                noise[left * PRIMITIVE_TARGETS + right] +=
                    weight[a] * (expected_square * cross + 2.0 * covariance[0] * covariance[1]);
            }
        }
    }
    let mut accumulator = NoiseAccumulator::default();
    let mut state = 23;
    let draws = 20_000;
    for _ in 0..draws {
        let zeta = variance
            .iter()
            .map(|v| v.sqrt() * normal(&mut state))
            .collect::<Vec<_>>();
        let pseudo_residual = apply(&maker, &zeta);
        let image = core::array::from_fn(|target| apply(&kernel[target], &zeta));
        accumulator
            .push(
                &variance,
                &weight,
                &pseudo_residual,
                image,
                &mut NeverInterrupt,
            )
            .unwrap();
    }
    let moments = accumulator.finish().unwrap();
    assert_eq!(moments.draws, draws);
    for index in 0..SQUARE {
        let scale = trace[0].abs().max(trace[4].abs());
        assert!(
            (moments.trace[index] - trace[index]).abs()
                < 6.0 * moments.trace_mcse[index] + 1e-12 * scale,
            "trace {index}: {} vs {}",
            moments.trace[index],
            trace[index]
        );
        assert!(
            (moments.residual_noise[index] - noise[index]).abs()
                < 6.0 * moments.residual_noise_mcse[index] + 1e-12 * noise[0].abs(),
            "noise {index}: {} vs {}",
            moments.residual_noise[index],
            noise[index]
        );
    }
    // Effective rank from consecutive pairs against tr(W^2)^2 / tr(W^4).
    for target in 0..PRIMITIVE_TARGETS {
        let w = |a: usize, b: usize| variance[a].sqrt() * kernel[target][a][b] * variance[b].sqrt();
        let mut second = vec![vec![0.0; rows]; rows];
        for a in 0..rows {
            for b in 0..rows {
                second[a][b] = (0..rows).map(|c| w(a, c) * w(c, b)).sum();
            }
        }
        let trace_second = (0..rows).map(|a| second[a][a]).sum::<f64>();
        let trace_fourth = (0..rows)
            .map(|a| (0..rows).map(|b| second[a][b] * second[b][a]).sum::<f64>())
            .sum::<f64>();
        let exact = trace_second * trace_second / trace_fourth;
        assert!(
            (moments.effective_rank[target] / exact - 1.0).abs() < 0.1,
            "effective rank {target}: {} vs {exact}",
            moments.effective_rank[target]
        );
    }
    assert!(moments.effective_rank[PRIMITIVE_TARGETS] >= 1.0);
}

#[test]
fn covariance_assembly_truncates_only_the_signal_and_applies_controls() {
    let rows = 6;
    let influence = [
        vec![1.0, -2.0, 0.5, 0.0, 1.5, -1.0],
        vec![0.5, 1.0, -1.0, 2.0, 0.0, 0.25],
        vec![-0.5, 0.25, 0.75, -1.0, 0.5, 0.0],
    ];
    let residual = vec![0.4, -0.3, 0.2, 0.5, -0.6, 0.1];
    let maker = vec![2.0, 1.5, 1.25, 4.0, 1.0, 3.0];
    let variance = vec![0.2; rows];
    let mut moments = NoiseMoments {
        trace: [0.3, 0.02, 0.01, 0.02, 0.2, 0.0, 0.01, 0.0, 0.1],
        trace_mcse: [0.0; SQUARE],
        residual_noise: [0.05, 0.0, 0.0, 0.0, 0.04, 0.0, 0.0, 0.0, 0.03],
        residual_noise_mcse: [0.0; SQUARE],
        draws: 8,
        effective_rank: [f64::NAN; REPORTED_TARGETS],
    };
    let plain = finish_covariance(
        &influence,
        &residual,
        &maker,
        &variance,
        None,
        &moments,
        &mut NeverInterrupt,
    )
    .unwrap();
    for left in 0..PRIMITIVE_TARGETS {
        for right in 0..PRIMITIVE_TARGETS {
            let index = left * PRIMITIVE_TARGETS + right;
            let stat = (0..rows)
                .map(|a| {
                    4.0 * influence[left][a]
                        * influence[right][a]
                        * residual[a]
                        * residual[a]
                        * maker[a]
                })
                .sum::<f64>();
            assert!(
                (plain.signal[index] - (stat - 4.0 * moments.residual_noise[index])).abs() < 1e-13
            );
            assert!((plain.trace_term[index] - 2.0 * moments.trace[index]).abs() < 1e-15);
        }
    }
    assert_eq!(plain.signal_truncation, 0.0);
    for index in 0..SQUARE {
        assert!(
            (plain.primitive[index] - plain.signal[index] - plain.trace_term[index]).abs() < 1e-12
        );
    }
    for share in plain.influence_concentration {
        assert!(share > 0.0 && share <= 1.0);
    }

    // A noise correction larger than the statistic makes the raw signal
    // indefinite; the reported covariance must still dominate the trace term.
    moments.residual_noise = [5.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
    let clipped = finish_covariance(
        &influence,
        &residual,
        &maker,
        &variance,
        None,
        &moments,
        &mut NeverInterrupt,
    )
    .unwrap();
    assert!(clipped.smallest_signal_eigenvalue < 0.0);
    assert!(clipped.signal_truncation > 0.0);
    assert!(clipped.primitive[0] >= clipped.trace_term[0] - 1e-12);
    assert!(clipped.primitive[0] > 0.0);

    // One control: scores gain the residualized-control slope and within term.
    moments.residual_noise = [0.0; SQUARE];
    let control = ControlAdjustment {
        controls: 1,
        collapsed: vec![1.0, 0.0, -1.0, 0.5, 2.0, 0.0],
        residualized: vec![0.2, -0.1, 0.0, 0.3, -0.2, 0.1],
        within: vec![0.05, 0.0, -0.02, 0.0, 0.01, 0.0],
        precision: vec![0.5],
    };
    let adjusted = finish_covariance(
        &influence,
        &residual,
        &maker,
        &variance,
        Some(&control),
        &moments,
        &mut NeverInterrupt,
    )
    .unwrap();
    let load = (0..PRIMITIVE_TARGETS)
        .map(|t| {
            0.5 * (0..rows)
                .map(|a| -2.0 * control.collapsed[a] * influence[t][a])
                .sum::<f64>()
        })
        .collect::<Vec<_>>();
    let score = |t: usize, a: usize| {
        (2.0 * influence[t][a] + control.residualized[a] * load[t]) * residual[a] * maker[a].sqrt()
            + control.within[a] * load[t]
    };
    for left in 0..PRIMITIVE_TARGETS {
        for right in 0..PRIMITIVE_TARGETS {
            let expected = (0..rows)
                .map(|a| score(left, a) * score(right, a))
                .sum::<f64>();
            assert!((adjusted.signal[left * PRIMITIVE_TARGETS + right] - expected).abs() < 1e-12);
        }
    }
}

#[test]
fn positive_part_matches_a_known_spectrum() {
    // Eigenvalues 3, 1, -2 in a rotated basis.
    let q = [
        [2.0_f64 / 3.0, -2.0 / 3.0, 1.0 / 3.0],
        [1.0 / 3.0, 2.0 / 3.0, 2.0 / 3.0],
        [2.0 / 3.0, 1.0 / 3.0, -2.0 / 3.0],
    ];
    let lambda = [3.0, 1.0, -2.0];
    let mut matrix = [0.0; SQUARE];
    let mut expected = [0.0; SQUARE];
    for row in 0..3 {
        for column in 0..3 {
            for k in 0..3 {
                matrix[row * 3 + column] += q[row][k] * lambda[k] * q[column][k];
                expected[row * 3 + column] += q[row][k] * lambda[k].max(0.0) * q[column][k];
            }
        }
    }
    let (output, smallest, largest) = positive_part(&matrix).unwrap();
    assert!((smallest + 2.0).abs() < 1e-12 && (largest - 3.0).abs() < 1e-12);
    for index in 0..SQUARE {
        assert!((output[index] - expected[index]).abs() < 1e-12);
    }
}
