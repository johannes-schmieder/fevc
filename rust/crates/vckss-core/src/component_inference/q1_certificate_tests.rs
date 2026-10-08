// SPDX-License-Identifier: GPL-3.0-only
use super::{finish_q1_target, finish_q1_target_with_solve_defect};
use crate::error::ErrorCode;

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn action(matrix: &[f64], rows: usize, columns: usize, vector: &[f64]) -> Vec<f64> {
    (0..rows)
        .map(|row| dot(&matrix[row * columns..(row + 1) * columns], vector))
        .collect()
}

// Small dense Gauss--Jordan inverse is independent of all production solves,
// residual certification, centering, and inference operators.
fn inverse3(matrix: &[f64]) -> Vec<f64> {
    let mut a = [[0.0; 6]; 3];
    for row in 0..3 {
        a[row][..3].copy_from_slice(&matrix[row * 3..(row + 1) * 3]);
        a[row][3 + row] = 1.0;
    }
    for column in 0..3 {
        let pivot = (column..3)
            .max_by(|&a_row, &b_row| a[a_row][column].abs().total_cmp(&a[b_row][column].abs()))
            .unwrap();
        a.swap(column, pivot);
        let diagonal = a[column][column];
        assert!(diagonal.abs() > 1.0e-10);
        for value in &mut a[column] {
            *value /= diagonal;
        }
        let normalized = a[column];
        for row in 0..3 {
            if row != column {
                let factor = a[row][column];
                for entry in 0..6 {
                    a[row][entry] -= factor * normalized[entry];
                }
            }
        }
    }
    a.into_iter().flat_map(|row| row[3..].to_vec()).collect()
}

#[test]
fn q1_solve_defect_dense_certificate_accepts_only_the_independently_explained_error() {
    for grouped in [false, true] {
        for center_mean in [false, true] {
            for perturb in [[false, false], [true, false], [false, true], [true, true]] {
                check_certificate(grouped, center_mean, perturb);
            }
        }
    }
}

fn check_certificate(grouped: bool, center_mean: bool, perturb: [bool; 2]) {
    let raw_y = [3.1, 2.4, 3.6, 5.2, 4.3, 6.5, 5.4, 7.9];
    let first = [-1.8, -1.1, -0.7, 0.1, 0.4, 0.9, 1.5, 2.2];
    let second = [0.7, -0.4, 1.2, -0.8, -1.3, 0.5, 1.1, -0.2];
    let mass: [f64; 8] = if grouped {
        [1.0, 3.0, 2.0, 5.0, 2.0, 4.0, 3.0, 1.0]
    } else {
        [1.0; 8]
    };
    let n = raw_y.len();
    let center = if center_mean {
        dot(&mass, &raw_y) / mass.iter().sum::<f64>()
    } else {
        0.0
    };
    let mut x = vec![0.0; n * 3];
    let mut xt = vec![0.0; n * 3];
    let mut z = vec![0.0; n];
    for row in 0..n {
        let root = mass[row].sqrt();
        z[row] = root * (raw_y[row] - center);
        for (column, value) in [1.0, first[row], second[row]].into_iter().enumerate() {
            x[row * 3 + column] = root * value;
            xt[column * n + row] = root * value;
        }
    }
    let h: Vec<f64> = (0..3)
        .flat_map(|row| (0..3).map(move |column| (row, column)))
        .map(|(row, column)| {
            dot(
                &xt[row * n..(row + 1) * n],
                &xt[column * n..(column + 1) * n],
            )
        })
        .collect();
    let inverse = inverse3(&h);
    let q = [0.0, 0.0, 0.0, 0.0, 1.0, 0.2, 0.0, 0.2, 0.7];
    let score = action(&xt, 3, n, &z);
    let mut beta = action(&inverse, 3, 3, &score);
    if perturb[0] {
        for (value, error) in beta.iter_mut().zip([1.0e-4, -2.0e-4, 4.0e-4]) {
            *value += error;
        }
    }
    let fit = action(&x, n, 3, &beta);
    let residual: Vec<f64> = z.iter().zip(&fit).map(|(z, fit)| z - fit).collect();
    let mut b = vec![0.0; n * n];
    let mut maker_inverse = vec![0.0; n];
    for column in 0..n {
        let row = &x[column * 3..(column + 1) * 3];
        let inverse_row = action(&inverse, 3, 3, row);
        maker_inverse[column] = (1.0 - dot(row, &inverse_row)).recip();
        let kernel_column = action(
            &x,
            n,
            3,
            &action(&inverse, 3, 3, &action(&q, 3, 3, &inverse_row)),
        );
        for row in 0..n {
            b[row * n + column] = kernel_column[row];
        }
    }
    let mut mode = action(&x, n, 3, &[0.0, 0.7, -0.3]);
    for _ in 0..1_024 {
        mode = action(&b, n, n, &mode);
        let norm = dot(&mode, &mode).sqrt();
        mode.iter_mut().for_each(|value| *value /= norm);
    }
    let b_mode = action(&b, n, n, &mode);
    let eigenvalue = dot(&mode, &b_mode);
    assert!(b_mode
        .iter()
        .zip(&mode)
        .all(|(b, u)| (b - eigenvalue * u).abs() < 1.0e-13));
    let ratio: Vec<f64> = (0..n)
        .map(|row| (b[row * n + row] - eigenvalue * mode[row].powi(2)) * maker_inverse[row])
        .collect();
    let half_rz: Vec<f64> = ratio.iter().zip(&z).map(|(r, z)| 0.5 * r * z).collect();
    let q_beta = action(&q, 3, 3, &beta);
    let rhs: Vec<f64> = q_beta
        .iter()
        .zip(action(&xt, 3, n, &half_rz))
        .map(|(a, b)| a + b)
        .collect();
    let mut u = action(&inverse, 3, 3, &rhs);
    if perturb[1] {
        for (value, error) in u.iter_mut().zip([1.5e-4, -3.0e-4, 1.0e-4]) {
            *value += error;
        }
    }
    let e0: Vec<f64> = score
        .iter()
        .zip(action(&h, 3, 3, &beta))
        .map(|(a, b)| a - b)
        .collect();
    let e1: Vec<f64> = rhs
        .iter()
        .zip(action(&h, 3, 3, &u))
        .map(|(a, b)| a - b)
        .collect();
    let defect = dot(&e0, &u) - dot(&beta, &e1);
    let leading_score = dot(&mode, &z);
    let prediction = action(&x, n, 3, &u);
    let influence: Vec<f64> = (0..n)
        .map(|row| {
            prediction[row]
                - 0.5 * ratio[row] * (z[row] + residual[row])
                - eigenvalue * mode[row] * leading_score
        })
        .collect();
    let point = dot(&beta, &q_beta)
        - (0..n)
            .map(|row| b[row * n + row] * z[row] * residual[row] * maker_inverse[row])
            .sum::<f64>();
    let raw_recenter: f64 = (0..n)
        .map(|row| mode[row].powi(2) * z[row] * residual[row] * maker_inverse[row])
        .sum();
    let direct = dot(&z, &influence);
    let remainder = point - eigenvalue * (leading_score.powi(2) - raw_recenter);
    let tolerance = 256.0 * f64::EPSILON;
    let scale = point.abs().max(direct.abs()).max(remainder.abs()).max(1.0);
    assert!((direct - remainder - defect).abs() < tolerance * scale);
    if perturb[0] || perturb[1] {
        assert!(defect.abs() > 1.0e-7);
    }
    let variance = vec![0.01; n];
    let finish = |point, recenter, direct, certificate| {
        finish_q1_target_with_solve_defect(
            point,
            leading_score,
            recenter,
            direct,
            tolerance,
            certificate,
            eigenvalue,
            &mode,
            &influence,
            &variance,
            0.0,
            0.0,
            1.0e-10,
        )
    };
    let certified =
        finish(point, raw_recenter, direct, defect).expect("independent solve certificate");
    assert!(
        (certified.remainder_identity_error - (remainder - direct).abs()).abs()
            <= f64::EPSILON * scale
    );
    let old_wrapper = finish_q1_target(
        point,
        leading_score,
        raw_recenter,
        direct,
        tolerance,
        eigenvalue,
        &mode,
        &influence,
        &variance,
        0.0,
        0.0,
        1.0e-10,
    );
    if perturb[0] || perturb[1] {
        assert_eq!(
            old_wrapper.unwrap_err().code,
            ErrorCode::TargetIdentityFailed
        );
        assert_eq!(
            finish(point, raw_recenter, direct, 0.0).unwrap_err().code,
            ErrorCode::TargetIdentityFailed
        );
    } else {
        old_wrapper.expect("the public zero-defect wrapper preserves exact-solve callers");
    }
    let modeled_recenter = dot(
        &mode.iter().map(|value| value * value).collect::<Vec<_>>(),
        &variance,
    );
    assert!((modeled_recenter - raw_recenter).abs() > 1.0e-3);
    let wrong_ratio_direct: f64 = (0..n)
        .map(|row| {
            z[row]
                * (prediction[row]
                    - 0.55 * ratio[row] * (z[row] + residual[row])
                    - eigenvalue * mode[row] * leading_score)
        })
        .sum();
    let mut wrong_influence = influence.clone();
    wrong_influence[0] += 0.02;
    for (corrupt_point, corrupt_recenter, corrupt_direct) in [
        (point + 0.01, raw_recenter, direct),
        (point, modeled_recenter, direct),
        (point, raw_recenter, wrong_ratio_direct),
        (point, raw_recenter, dot(&z, &wrong_influence)),
    ] {
        let error = finish(corrupt_point, corrupt_recenter, corrupt_direct, defect)
            .expect_err("a fixed independent certificate cannot conceal scientific corruption");
        assert_eq!(error.code, ErrorCode::TargetIdentityFailed);
    }
    for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            finish(point, raw_recenter, direct, invalid)
                .unwrap_err()
                .code,
            ErrorCode::InvalidInput
        );
    }
}
