// SPDX-License-Identifier: GPL-3.0-only

//! Independent dense identities for the fixed-offset collapsed-match route.
//!
//! These tests deliberately do not call the production match-collapse,
//! maker, target-kernel, or covariance routines. One oracle expands every
//! frequency weight into physical rows and constructs full match blocks; the
//! other starts from separately accumulated scalar match sufficient
//! statistics.

#[derive(Clone, Debug)]
struct StoredRow {
    worker: usize,
    firm: usize,
    deletion: usize,
    outcome: f64,
    frequency: usize,
    target_mass: f64,
    controls: [f64; 2],
}

fn fixture() -> Vec<StoredRow> {
    let mut rows = Vec::new();
    for worker in 0..4 {
        for firm in 0..4 {
            let cell_deletion = worker * 4 + firm;
            let stored = 2 + (worker + 2 * firm) % 2;
            for within in 0..stored {
                let index = rows.len();
                rows.push(StoredRow {
                    worker,
                    firm,
                    deletion: if worker == 0 && firm == 0 && within == 1 {
                        16
                    } else {
                        cell_deletion
                    },
                    outcome: 0.7 * worker as f64 - 0.43 * firm as f64
                        + 0.29 * within as f64
                        + ((index * 19 + 7) % 31) as f64 / 37.0,
                    frequency: 1 + (index + worker + within) % 3,
                    target_mass: 0.4 + ((index * 13 + 5) % 23) as f64 / 17.0,
                    // Controls intentionally vary inside every match before
                    // the full-sample offset is estimated and removed.
                    controls: [
                        0.21 * worker as f64 - 0.16 * firm as f64
                            + 0.37 * within as f64
                            + (index % 5) as f64 / 11.0,
                        -0.14 * worker as f64 + 0.19 * firm as f64 - 0.31 * within as f64
                            + (index % 7) as f64 / 13.0,
                    ],
                });
            }
        }
    }
    rows
}

fn fe_row(worker: usize, firm: usize) -> Vec<f64> {
    let mut row = vec![0.0; 7];
    row[worker] = 1.0;
    if firm < 3 {
        row[4 + firm] = 1.0;
    }
    row
}

fn multiply(
    left: &[f64],
    left_rows: usize,
    inner: usize,
    right: &[f64],
    right_cols: usize,
) -> Vec<f64> {
    let mut output = vec![0.0; left_rows * right_cols];
    for row in 0..left_rows {
        for column in 0..right_cols {
            for k in 0..inner {
                output[row * right_cols + column] +=
                    left[row * inner + k] * right[k * right_cols + column];
            }
        }
    }
    output
}

fn transpose(value: &[f64], rows: usize, columns: usize) -> Vec<f64> {
    let mut output = vec![0.0; value.len()];
    for row in 0..rows {
        for column in 0..columns {
            output[column * rows + row] = value[row * columns + column];
        }
    }
    output
}

fn inverse(value: &[f64], dimension: usize) -> Vec<f64> {
    let mut augmented = vec![0.0; dimension * 2 * dimension];
    let width = 2 * dimension;
    for row in 0..dimension {
        for column in 0..dimension {
            augmented[row * width + column] = value[row * dimension + column];
        }
        augmented[row * width + dimension + row] = 1.0;
    }
    for pivot in 0..dimension {
        let selected = (pivot..dimension)
            .max_by(|&left, &right| {
                augmented[left * width + pivot]
                    .abs()
                    .total_cmp(&augmented[right * width + pivot].abs())
            })
            .expect("nonempty pivot range");
        assert!(augmented[selected * width + pivot].abs() > 1.0e-12);
        if selected != pivot {
            for column in 0..width {
                augmented.swap(pivot * width + column, selected * width + column);
            }
        }
        let scale = augmented[pivot * width + pivot];
        for column in 0..width {
            augmented[pivot * width + column] /= scale;
        }
        for row in 0..dimension {
            if row == pivot {
                continue;
            }
            let factor = augmented[row * width + pivot];
            for column in 0..width {
                augmented[row * width + column] -= factor * augmented[pivot * width + column];
            }
        }
    }
    let mut output = vec![0.0; dimension * dimension];
    for row in 0..dimension {
        output[row * dimension..(row + 1) * dimension]
            .copy_from_slice(&augmented[row * width + dimension..row * width + 2 * dimension]);
    }
    output
}

fn matvec(matrix: &[f64], rows: usize, columns: usize, vector: &[f64]) -> Vec<f64> {
    (0..rows)
        .map(|row| {
            (0..columns)
                .map(|column| matrix[row * columns + column] * vector[column])
                .sum()
        })
        .collect()
}

fn quadratic(vector: &[f64], matrix: &[f64]) -> f64 {
    let action = matvec(matrix, vector.len(), vector.len(), vector);
    vector
        .iter()
        .zip(action)
        .map(|(&left, right)| left * right)
        .sum()
}

fn subtract(left: &[f64], right: &[f64]) -> Vec<f64> {
    left.iter()
        .zip(right)
        .map(|(&left, &right)| left - right)
        .collect()
}

fn identity_minus(value: &[f64], dimension: usize) -> Vec<f64> {
    let mut output = value.iter().map(|entry| -*entry).collect::<Vec<_>>();
    for index in 0..dimension {
        output[index * dimension + index] += 1.0;
    }
    output
}

fn submatrix(matrix: &[f64], dimension: usize, rows: &[usize], columns: &[usize]) -> Vec<f64> {
    let mut output = vec![0.0; rows.len() * columns.len()];
    for (local_row, &row) in rows.iter().enumerate() {
        for (local_column, &column) in columns.iter().enumerate() {
            output[local_row * columns.len() + local_column] = matrix[row * dimension + column];
        }
    }
    output
}

fn max_abs_difference(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(&left, &right)| (left - right).abs())
        .fold(0.0, f64::max)
}

fn assert_close(left: f64, right: f64, tolerance: f64) {
    let scale = left.abs().max(right.abs()).max(1.0);
    assert!(
        (left - right).abs() <= tolerance * scale,
        "left={left:.17e}, right={right:.17e}, tolerance={tolerance:.3e}"
    );
}

fn collapsed_matrix_free_action(target: &[f64], maker: &[f64], vector: &[f64]) -> Vec<f64> {
    let n = vector.len();
    let projection = identity_minus(maker, n);
    let fitted = matvec(&projection, n, n, vector);
    let residual = subtract(vector, &fitted);
    let ratio = (0..n)
        .map(|row| target[row * n + row] / maker[row * n + row])
        .collect::<Vec<_>>();
    let ratio_vector = ratio
        .iter()
        .zip(vector)
        .map(|(&ratio, &value)| ratio * value)
        .collect::<Vec<_>>();
    let ratio_projection = matvec(&projection, n, n, &ratio_vector);
    let target_action = matvec(target, n, n, vector);
    (0..n)
        .map(|row| {
            target_action[row]
                - 0.5 * (ratio[row] * residual[row] + ratio_vector[row] - ratio_projection[row])
        })
        .collect()
}

fn target_matrices(rows: &[StoredRow]) -> [Vec<f64>; 3] {
    let p = 7;
    let total = rows.iter().map(|row| row.target_mass).sum::<f64>();
    let mut worker_mass = [0.0; 4];
    let mut firm_mass = [0.0; 4];
    let mut joint = [[0.0; 4]; 4];
    for row in rows {
        worker_mass[row.worker] += row.target_mass / total;
        firm_mass[row.firm] += row.target_mass / total;
        joint[row.worker][row.firm] += row.target_mass / total;
    }
    let mut worker = vec![0.0; p * p];
    let mut firm = vec![0.0; p * p];
    let mut covariance = vec![0.0; p * p];
    for left in 0..4 {
        for right in 0..4 {
            worker[left * p + right] = if left == right {
                worker_mass[left]
            } else {
                0.0
            } - worker_mass[left] * worker_mass[right];
        }
    }
    for left in 0..3 {
        for right in 0..3 {
            firm[(4 + left) * p + 4 + right] = if left == right { firm_mass[left] } else { 0.0 }
                - firm_mass[left] * firm_mass[right];
        }
    }
    for worker_index in 0..4 {
        for firm_index in 0..3 {
            let value = 0.5
                * (joint[worker_index][firm_index]
                    - worker_mass[worker_index] * firm_mass[firm_index]);
            covariance[worker_index * p + 4 + firm_index] = value;
            covariance[(4 + firm_index) * p + worker_index] = value;
        }
    }
    [worker, firm, covariance]
}

#[derive(Clone, Debug)]
struct OracleState {
    x: Vec<f64>,
    y: Vec<f64>,
    groups: Vec<Vec<usize>>,
    beta: Vec<f64>,
    projection: Vec<f64>,
    maker: Vec<f64>,
    target_kernel: [Vec<f64>; 3],
    leaveout_kernel: [Vec<f64>; 3],
    correction: [f64; 3],
}

fn full_sample_offset(rows: &[StoredRow]) -> (Vec<f64>, [f64; 2]) {
    let parameters = 9;
    let mut gram = vec![0.0; parameters * parameters];
    let mut rhs = vec![0.0; parameters];
    for row in rows {
        let mut design = fe_row(row.worker, row.firm);
        design.extend_from_slice(&row.controls);
        for left in 0..parameters {
            rhs[left] += row.frequency as f64 * design[left] * row.outcome;
            for right in 0..parameters {
                gram[left * parameters + right] +=
                    row.frequency as f64 * design[left] * design[right];
            }
        }
    }
    let coefficients = matvec(&inverse(&gram, parameters), parameters, parameters, &rhs);
    let gamma = [coefficients[7], coefficients[8]];
    let offset_outcome = rows
        .iter()
        .map(|row| row.outcome - row.controls[0] * gamma[0] - row.controls[1] * gamma[1])
        .collect();
    (offset_outcome, gamma)
}

fn physical_block_oracle(rows: &[StoredRow], y_star: &[f64], q: &[Vec<f64>; 3]) -> OracleState {
    let p = 7;
    let groups_count = rows
        .iter()
        .map(|row| row.deletion)
        .max()
        .expect("nonempty rows")
        + 1;
    let physical = rows.iter().map(|row| row.frequency).sum::<usize>();
    let mut x = Vec::with_capacity(physical * p);
    let mut y = Vec::with_capacity(physical);
    let mut groups = vec![Vec::new(); groups_count];
    for (stored, row) in rows.iter().enumerate() {
        for _ in 0..row.frequency {
            let physical_row = y.len();
            x.extend_from_slice(&fe_row(row.worker, row.firm));
            y.push(y_star[stored]);
            groups[row.deletion].push(physical_row);
        }
    }
    let xt = transpose(&x, physical, p);
    let h = multiply(&xt, p, physical, &x, p);
    let inverse_h = inverse(&h, p);
    let beta = matvec(&inverse_h, p, p, &matvec(&xt, p, physical, &y));
    let projection = multiply(
        &multiply(&x, physical, p, &inverse_h, p),
        physical,
        p,
        &xt,
        physical,
    );
    let maker = identity_minus(&projection, physical);
    let residual = matvec(&maker, physical, physical, &y);
    let target_kernel: [Vec<f64>; 3] = core::array::from_fn(|target| {
        let middle = multiply(
            &multiply(&inverse_h, p, p, &q[target], p),
            p,
            p,
            &inverse_h,
            p,
        );
        multiply(
            &multiply(&x, physical, p, &middle, p),
            physical,
            p,
            &xt,
            physical,
        )
    });
    let mut correction = [0.0; 3];
    let mut leaveout_kernel: [Vec<f64>; 3] =
        core::array::from_fn(|_| vec![0.0; physical * physical]);
    for target in 0..3 {
        let mut r = vec![0.0; physical * physical];
        for group in &groups {
            let mgg = submatrix(&maker, physical, group, group);
            let bgg = submatrix(&target_kernel[target], physical, group, group);
            let product = multiply(
                &bgg,
                group.len(),
                group.len(),
                &inverse(&mgg, group.len()),
                group.len(),
            );
            let yg = group.iter().map(|&row| y[row]).collect::<Vec<_>>();
            let eg = group.iter().map(|&row| residual[row]).collect::<Vec<_>>();
            correction[target] += yg
                .iter()
                .zip(matvec(&product, group.len(), group.len(), &eg))
                .map(|(&left, right)| left * right)
                .sum::<f64>();
            for (local_row, &row) in group.iter().enumerate() {
                for (local_column, &column) in group.iter().enumerate() {
                    r[row * physical + column] = product[local_row * group.len() + local_column];
                }
            }
        }
        let rm = multiply(&r, physical, physical, &maker, physical);
        let mrt = multiply(
            &maker,
            physical,
            physical,
            &transpose(&r, physical, physical),
            physical,
        );
        for index in 0..physical * physical {
            leaveout_kernel[target][index] =
                target_kernel[target][index] - 0.5 * (rm[index] + mrt[index]);
        }
    }
    OracleState {
        x,
        y,
        groups,
        beta,
        projection,
        maker,
        target_kernel,
        leaveout_kernel,
        correction,
    }
}

fn collapsed_scalar_oracle(rows: &[StoredRow], y_star: &[f64], q: &[Vec<f64>; 3]) -> OracleState {
    let p = 7;
    let groups_count = rows
        .iter()
        .map(|row| row.deletion)
        .max()
        .expect("nonempty rows")
        + 1;
    let mut mass = vec![0.0; groups_count];
    let mut y_sum = vec![0.0; groups_count];
    for (index, row) in rows.iter().enumerate() {
        mass[row.deletion] += row.frequency as f64;
        y_sum[row.deletion] += row.frequency as f64 * y_star[index];
    }
    let mut x = Vec::with_capacity(groups_count * p);
    let mut y = Vec::with_capacity(groups_count);
    for group in 0..groups_count {
        let root = mass[group].sqrt();
        let stored = rows
            .iter()
            .find(|row| row.deletion == group)
            .expect("match row");
        x.extend(
            fe_row(stored.worker, stored.firm)
                .into_iter()
                .map(|value| root * value),
        );
        y.push(y_sum[group] / root);
    }
    let xt = transpose(&x, groups_count, p);
    let h = multiply(&xt, p, groups_count, &x, p);
    let inverse_h = inverse(&h, p);
    let beta = matvec(&inverse_h, p, p, &matvec(&xt, p, groups_count, &y));
    let projection = multiply(
        &multiply(&x, groups_count, p, &inverse_h, p),
        groups_count,
        p,
        &xt,
        groups_count,
    );
    let maker = identity_minus(&projection, groups_count);
    let residual = matvec(&maker, groups_count, groups_count, &y);
    let target_kernel: [Vec<f64>; 3] = core::array::from_fn(|target| {
        let middle = multiply(
            &multiply(&inverse_h, p, p, &q[target], p),
            p,
            p,
            &inverse_h,
            p,
        );
        multiply(
            &multiply(&x, groups_count, p, &middle, p),
            groups_count,
            p,
            &xt,
            groups_count,
        )
    });
    let mut correction = [0.0; 3];
    let leaveout_kernel: [Vec<f64>; 3] = core::array::from_fn(|target| {
        let mut ratio = vec![0.0; groups_count];
        for group in 0..groups_count {
            ratio[group] = target_kernel[target][group * groups_count + group]
                / maker[group * groups_count + group];
            correction[target] += ratio[group] * y[group] * residual[group];
        }
        let mut output = target_kernel[target].clone();
        for row in 0..groups_count {
            for column in 0..groups_count {
                output[row * groups_count + column] -= 0.5
                    * (ratio[row] * maker[row * groups_count + column]
                        + maker[row * groups_count + column] * ratio[column]);
            }
        }
        output
    });
    OracleState {
        x,
        y,
        groups: (0..groups_count).map(|group| vec![group]).collect(),
        beta,
        projection,
        maker,
        target_kernel,
        leaveout_kernel,
        correction,
    }
}

fn collapse_map(physical: &OracleState) -> Vec<f64> {
    let n = physical.y.len();
    let groups = physical.groups.len();
    let mut map = vec![0.0; n * groups];
    for (group, rows) in physical.groups.iter().enumerate() {
        let scale = (rows.len() as f64).sqrt().recip();
        for &row in rows {
            map[row * groups + group] = scale;
        }
    }
    map
}

fn covariance_blocks(pattern: usize, physical: &OracleState) -> Vec<Vec<f64>> {
    physical
        .groups
        .iter()
        .enumerate()
        .map(|(group, rows)| {
            let n = rows.len();
            let mut covariance = vec![0.0; n * n];
            match pattern {
                0 => {
                    for row in 0..n {
                        covariance[row * n + row] = 0.6 + 0.07 * (group + row) as f64;
                    }
                }
                1 => {
                    for row in 0..n {
                        for column in 0..n {
                            covariance[row * n + column] = 0.18;
                        }
                        covariance[row * n + row] += 0.45 + 0.01 * group as f64;
                    }
                }
                2 => {
                    for row in 0..n {
                        for column in 0..n {
                            covariance[row * n + column] = (0.52 + 0.015 * group as f64)
                                * 0.63_f64.powi(row.abs_diff(column) as i32);
                        }
                    }
                }
                3 => {
                    // Different block shapes are rescaled to exactly the same
                    // aggregate variance, demonstrating that only v'Gamma v
                    // enters the collapsed kernel.
                    for row in 0..n {
                        for column in 0..n {
                            covariance[row * n + column] =
                                0.7_f64.powi(row.abs_diff(column) as i32);
                        }
                    }
                    let sum = covariance.iter().sum::<f64>() / n as f64;
                    for value in &mut covariance {
                        *value *= 1.25 / sum;
                    }
                }
                _ => unreachable!(),
            }
            covariance
        })
        .collect()
}

fn block_diagonal(blocks: &[Vec<f64>], groups: &[Vec<usize>], dimension: usize) -> Vec<f64> {
    let mut output = vec![0.0; dimension * dimension];
    for (block, rows) in blocks.iter().zip(groups) {
        for (local_row, &row) in rows.iter().enumerate() {
            for (local_column, &column) in rows.iter().enumerate() {
                output[row * dimension + column] = block[local_row * rows.len() + local_column];
            }
        }
    }
    output
}

fn gaussian_covariance(
    kernel_left: &[f64],
    kernel_right: &[f64],
    sigma: &[f64],
    mu: &[f64],
) -> f64 {
    let n = mu.len();
    let left_mu = matvec(kernel_left, n, n, mu);
    let right_mu = matvec(kernel_right, n, n, mu);
    let sigma_right = matvec(sigma, n, n, &right_mu);
    let linear = 4.0
        * left_mu
            .iter()
            .zip(sigma_right)
            .map(|(&left, right)| left * right)
            .sum::<f64>();
    let product = multiply(
        &multiply(
            &multiply(kernel_left, n, n, sigma, n),
            n,
            n,
            kernel_right,
            n,
        ),
        n,
        n,
        sigma,
        n,
    );
    let trace = (0..n).map(|index| product[index * n + index]).sum::<f64>();
    linear + 2.0 * trace
}

fn reported_matrices(primitive: &[Vec<f64>; 3]) -> [Vec<f64>; 4] {
    let total = primitive[0]
        .iter()
        .zip(&primitive[1])
        .zip(&primitive[2])
        .map(|((&worker, &firm), &covariance)| worker + firm + 2.0 * covariance)
        .collect();
    [
        primitive[0].clone(),
        primitive[1].clone(),
        primitive[2].clone(),
        total,
    ]
}

fn symmetric_extreme_eigenpair(matrix: &[f64], dimension: usize) -> (f64, Vec<f64>) {
    assert_eq!(matrix.len(), dimension * dimension);
    let mut action = matrix.to_vec();
    let mut vectors = vec![0.0; dimension * dimension];
    for index in 0..dimension {
        vectors[index * dimension + index] = 1.0;
    }
    let scale = matrix.iter().map(|value| value.abs()).fold(0.0, f64::max);
    for _ in 0..100 {
        let mut maximum = 0.0_f64;
        for left in 0..dimension {
            for right in left + 1..dimension {
                let cross = action[left * dimension + right];
                maximum = maximum.max(cross.abs());
                if cross.abs() <= 1.0e-15 * scale.max(1.0) {
                    continue;
                }
                let left_diagonal = action[left * dimension + left];
                let right_diagonal = action[right * dimension + right];
                let theta = (right_diagonal - left_diagonal) / (2.0 * cross);
                let tangent = if theta >= 0.0 {
                    1.0 / (theta + (1.0 + theta * theta).sqrt())
                } else {
                    -1.0 / (-theta + (1.0 + theta * theta).sqrt())
                };
                let cosine = 1.0 / (1.0 + tangent * tangent).sqrt();
                let sine = tangent * cosine;
                for row in 0..dimension {
                    if row == left || row == right {
                        continue;
                    }
                    let first = action[row * dimension + left];
                    let second = action[row * dimension + right];
                    let rotated_first = cosine * first - sine * second;
                    let rotated_second = sine * first + cosine * second;
                    action[row * dimension + left] = rotated_first;
                    action[left * dimension + row] = rotated_first;
                    action[row * dimension + right] = rotated_second;
                    action[right * dimension + row] = rotated_second;
                }
                action[left * dimension + left] = left_diagonal - tangent * cross;
                action[right * dimension + right] = right_diagonal + tangent * cross;
                action[left * dimension + right] = 0.0;
                action[right * dimension + left] = 0.0;
                for row in 0..dimension {
                    let first = vectors[row * dimension + left];
                    let second = vectors[row * dimension + right];
                    vectors[row * dimension + left] = cosine * first - sine * second;
                    vectors[row * dimension + right] = sine * first + cosine * second;
                }
            }
        }
        if maximum <= 2.0e-13 * scale.max(1.0) {
            break;
        }
    }
    let selected = (0..dimension)
        .max_by(|&left, &right| {
            action[left * dimension + left]
                .abs()
                .total_cmp(&action[right * dimension + right].abs())
        })
        .expect("positive eigen dimension");
    let eigenvalue = action[selected * dimension + selected];
    let mode = (0..dimension)
        .map(|row| vectors[row * dimension + selected])
        .collect::<Vec<_>>();
    let residual = subtract(
        &matvec(matrix, dimension, dimension, &mode),
        &mode
            .iter()
            .map(|value| eigenvalue * value)
            .collect::<Vec<_>>(),
    );
    assert!(
        residual
            .iter()
            .map(|value| value * value)
            .sum::<f64>()
            .sqrt()
            <= 2.0e-10 * eigenvalue.abs().max(1.0),
        "independent dense eigensolver did not converge"
    );
    (eigenvalue, mode)
}

fn physical_block_leaveout_kernel(
    target: &[f64],
    maker: &[f64],
    groups: &[Vec<usize>],
    dimension: usize,
) -> Vec<f64> {
    let mut ratio = vec![0.0; dimension * dimension];
    for group in groups {
        let block_maker = submatrix(maker, dimension, group, group);
        let block_target = submatrix(target, dimension, group, group);
        let product = multiply(
            &block_target,
            group.len(),
            group.len(),
            &inverse(&block_maker, group.len()),
            group.len(),
        );
        for (local_row, &row) in group.iter().enumerate() {
            for (local_column, &column) in group.iter().enumerate() {
                ratio[row * dimension + column] = product[local_row * group.len() + local_column];
            }
        }
    }
    let ratio_maker = multiply(&ratio, dimension, dimension, maker, dimension);
    let maker_ratio_transpose = multiply(
        maker,
        dimension,
        dimension,
        &transpose(&ratio, dimension, dimension),
        dimension,
    );
    target
        .iter()
        .zip(ratio_maker)
        .zip(maker_ratio_transpose)
        .map(|((&target, left), right)| target - 0.5 * (left + right))
        .collect()
}

fn scalar_leaveout_kernel(target: &[f64], maker: &[f64], dimension: usize) -> Vec<f64> {
    let ratio = (0..dimension)
        .map(|row| target[row * dimension + row] / maker[row * dimension + row])
        .collect::<Vec<_>>();
    let mut output = target.to_vec();
    for row in 0..dimension {
        for column in 0..dimension {
            output[row * dimension + column] -= 0.5
                * (ratio[row] * maker[row * dimension + column]
                    + maker[row * dimension + column] * ratio[column]);
        }
    }
    output
}

fn subtract_rank_one(matrix: &[f64], mode: &[f64], eigenvalue: f64) -> Vec<f64> {
    let dimension = mode.len();
    (0..dimension)
        .flat_map(|row| {
            (0..dimension).map(move |column| {
                matrix[row * dimension + column] - eigenvalue * mode[row] * mode[column]
            })
        })
        .collect()
}

fn raw_physical_block_recenter(state: &OracleState, mode: &[f64]) -> f64 {
    let residual = matvec(&state.maker, state.y.len(), state.y.len(), &state.y);
    state
        .groups
        .iter()
        .map(|group| {
            let block_maker = submatrix(&state.maker, state.y.len(), group, group);
            let deleted_residual = group.iter().map(|&row| residual[row]).collect::<Vec<_>>();
            let adjusted = matvec(
                &inverse(&block_maker, group.len()),
                group.len(),
                group.len(),
                &deleted_residual,
            );
            let score = group
                .iter()
                .map(|&row| mode[row] * state.y[row])
                .sum::<f64>();
            let adjusted_score = group
                .iter()
                .zip(adjusted)
                .map(|(&row, value)| mode[row] * value)
                .sum::<f64>();
            score * adjusted_score
        })
        .sum()
}

fn raw_physical_observationwise_recenter(state: &OracleState, mode: &[f64]) -> f64 {
    let residual = matvec(&state.maker, state.y.len(), state.y.len(), &state.y);
    state
        .groups
        .iter()
        .map(|group| {
            let block_maker = submatrix(&state.maker, state.y.len(), group, group);
            let deleted_residual = group.iter().map(|&row| residual[row]).collect::<Vec<_>>();
            let adjusted = matvec(
                &inverse(&block_maker, group.len()),
                group.len(),
                group.len(),
                &deleted_residual,
            );
            group
                .iter()
                .zip(adjusted)
                .map(|(&row, value)| mode[row].powi(2) * state.y[row] * value)
                .sum::<f64>()
        })
        .sum()
}

fn raw_scalar_recenter(state: &OracleState, mode: &[f64]) -> f64 {
    let residual = matvec(&state.maker, state.y.len(), state.y.len(), &state.y);
    (0..state.y.len())
        .map(|group| {
            mode[group].powi(2) * state.y[group] * residual[group]
                / state.maker[group * state.y.len() + group]
        })
        .sum()
}

fn q1_population_covariance(
    mode: &[f64],
    remainder_kernel: &[f64],
    covariance: &[f64],
    mean: &[f64],
) -> [f64; 3] {
    let dimension = mode.len();
    let covariance_mode = matvec(covariance, dimension, dimension, mode);
    let leading_variance = mode
        .iter()
        .zip(&covariance_mode)
        .map(|(&left, &right)| left * right)
        .sum::<f64>();
    let influence = matvec(remainder_kernel, dimension, dimension, mean);
    let covariance_influence = matvec(covariance, dimension, dimension, &influence);
    let leading_remainder = 2.0
        * mode
            .iter()
            .zip(covariance_influence)
            .map(|(&left, right)| left * right)
            .sum::<f64>();
    let remainder_variance =
        gaussian_covariance(remainder_kernel, remainder_kernel, covariance, mean);
    [leading_variance, leading_remainder, remainder_variance]
}

#[test]
fn physical_block_maker_and_collapsed_scalar_oracles_agree() {
    let rows = fixture();
    let (y_star, gamma) = full_sample_offset(&rows);
    assert!(gamma.iter().all(|value| value.is_finite()));
    assert!(gamma.iter().any(|value| value.abs() > 1.0e-3));
    assert!(rows.iter().enumerate().any(|(left, row)| {
        rows.iter().skip(left + 1).any(|other| {
            row.deletion == other.deletion
                && (row.controls[0] != other.controls[0] || row.controls[1] != other.controls[1])
        })
    }));
    let q = target_matrices(&rows);
    let physical = physical_block_oracle(&rows, &y_star, &q);
    let collapsed = collapsed_scalar_oracle(&rows, &y_star, &q);
    assert!(max_abs_difference(&physical.beta, &collapsed.beta) < 2.0e-12);

    let map = collapse_map(&physical);
    let map_t = transpose(&map, physical.y.len(), collapsed.y.len());
    let collapsed_from_physical = matvec(&map_t, collapsed.y.len(), physical.y.len(), &physical.y);
    assert!(max_abs_difference(&collapsed_from_physical, &collapsed.y) < 2.0e-13);
    let collapsed_projection = multiply(
        &multiply(
            &map_t,
            collapsed.y.len(),
            physical.y.len(),
            &physical.projection,
            physical.y.len(),
        ),
        collapsed.y.len(),
        physical.y.len(),
        &map,
        collapsed.y.len(),
    );
    assert!(max_abs_difference(&collapsed_projection, &collapsed.projection) < 2.0e-12);

    for target in 0..3 {
        let plugin_physical = quadratic(&physical.beta, &q[target]);
        let plugin_collapsed = quadratic(&collapsed.beta, &q[target]);
        assert_close(plugin_physical, plugin_collapsed, 2.0e-12);
        assert_close(
            physical.correction[target],
            collapsed.correction[target],
            2.0e-11,
        );
        assert_close(
            quadratic(&physical.y, &physical.leaveout_kernel[target]),
            plugin_physical - physical.correction[target],
            2.0e-11,
        );
        let arbitrary = (0..collapsed.y.len())
            .map(|row| ((row * 29 + 7) % 43) as f64 / 17.0 - 1.0)
            .collect::<Vec<_>>();
        let dense_action = matvec(
            &collapsed.leaveout_kernel[target],
            collapsed.y.len(),
            collapsed.y.len(),
            &arbitrary,
        );
        let matrix_free_action = collapsed_matrix_free_action(
            &collapsed.target_kernel[target],
            &collapsed.maker,
            &arbitrary,
        );
        assert!(max_abs_difference(&dense_action, &matrix_free_action) < 3.0e-11);
        let compressed_target = multiply(
            &multiply(
                &map_t,
                collapsed.y.len(),
                physical.y.len(),
                &physical.target_kernel[target],
                physical.y.len(),
            ),
            collapsed.y.len(),
            physical.y.len(),
            &map,
            collapsed.y.len(),
        );
        assert!(max_abs_difference(&compressed_target, &collapsed.target_kernel[target]) < 3.0e-11);
        assert_close(
            quadratic(&collapsed.y, &collapsed.leaveout_kernel[target]),
            plugin_collapsed - collapsed.correction[target],
            2.0e-11,
        );

        let compressed_kernel = multiply(
            &multiply(
                &map_t,
                collapsed.y.len(),
                physical.y.len(),
                &physical.leaveout_kernel[target],
                physical.y.len(),
            ),
            collapsed.y.len(),
            physical.y.len(),
            &map,
            collapsed.y.len(),
        );
        assert!(
            max_abs_difference(&compressed_kernel, &collapsed.leaveout_kernel[target]) < 3.0e-11
        );
        for group in 0..collapsed.y.len() {
            assert!(
                collapsed.leaveout_kernel[target][group * collapsed.y.len() + group].abs()
                    < 2.0e-12
            );
        }
    }

    let physical_residual = matvec(
        &physical.maker,
        physical.y.len(),
        physical.y.len(),
        &physical.y,
    );
    let collapsed_residual = matvec(
        &collapsed.maker,
        collapsed.y.len(),
        collapsed.y.len(),
        &collapsed.y,
    );
    for (group, rows) in physical.groups.iter().enumerate() {
        let mgg = submatrix(&physical.maker, physical.y.len(), rows, rows);
        let eg = rows
            .iter()
            .map(|&row| physical_residual[row])
            .collect::<Vec<_>>();
        let adjusted = matvec(&inverse(&mgg, rows.len()), rows.len(), rows.len(), &eg);
        let contraction = adjusted.iter().sum::<f64>() / (rows.len() as f64).sqrt();
        let scalar = collapsed_residual[group] / collapsed.maker[group * collapsed.y.len() + group];
        assert_close(contraction, scalar, 2.0e-11);

        let xt = transpose(&physical.x, physical.y.len(), 7);
        let deleted_x = rows
            .iter()
            .flat_map(|&row| physical.x[row * 7..(row + 1) * 7].iter().copied())
            .collect::<Vec<_>>();
        let deleted_xt = transpose(&deleted_x, rows.len(), 7);
        let full_h = multiply(&xt, 7, physical.y.len(), &physical.x, 7);
        let deleted_h = multiply(&deleted_xt, 7, rows.len(), &deleted_x, 7);
        let remaining_h = subtract(&full_h, &deleted_h);
        let deleted_y = rows.iter().map(|&row| physical.y[row]).collect::<Vec<_>>();
        let full_rhs = matvec(&xt, 7, physical.y.len(), &physical.y);
        let deleted_rhs = matvec(&deleted_xt, 7, rows.len(), &deleted_y);
        let physical_beta = matvec(
            &inverse(&remaining_h, 7),
            7,
            7,
            &subtract(&full_rhs, &deleted_rhs),
        );

        let xc = &collapsed.x[group * 7..(group + 1) * 7];
        let collapsed_h = multiply(
            &transpose(&collapsed.x, collapsed.y.len(), 7),
            7,
            collapsed.y.len(),
            &collapsed.x,
            7,
        );
        let collapsed_rhs = matvec(
            &transpose(&collapsed.x, collapsed.y.len(), 7),
            7,
            collapsed.y.len(),
            &collapsed.y,
        );
        let removed_h = (0..7)
            .flat_map(|left| (0..7).map(move |right| xc[left] * xc[right]))
            .collect::<Vec<_>>();
        let removed_rhs = xc
            .iter()
            .map(|value| value * collapsed.y[group])
            .collect::<Vec<_>>();
        let collapsed_beta = matvec(
            &inverse(&subtract(&collapsed_h, &removed_h), 7),
            7,
            7,
            &subtract(&collapsed_rhs, &removed_rhs),
        );
        assert!(max_abs_difference(&physical_beta, &collapsed_beta) < 3.0e-11);
    }
}

#[test]
fn arbitrary_within_match_covariance_enters_only_through_aggregate_variance() {
    let rows = fixture();
    let (y_star, _) = full_sample_offset(&rows);
    let q = target_matrices(&rows);
    let physical = physical_block_oracle(&rows, &y_star, &q);
    let collapsed = collapsed_scalar_oracle(&rows, &y_star, &q);
    let map = collapse_map(&physical);
    let map_t = transpose(&map, physical.y.len(), collapsed.y.len());
    let signal_coefficients = vec![0.2, -0.1, 0.35, -0.25, 0.3, -0.15, 0.18];
    let physical_mean = matvec(&physical.x, physical.y.len(), 7, &signal_coefficients);
    let collapsed_mean = matvec(&map_t, collapsed.y.len(), physical.y.len(), &physical_mean);

    for pattern in 0..4 {
        let blocks = covariance_blocks(pattern, &physical);
        let sigma = block_diagonal(&blocks, &physical.groups, physical.y.len());
        let mut tau = vec![0.0; physical.groups.len()];
        for (group, block) in blocks.iter().enumerate() {
            tau[group] = block.iter().sum::<f64>() / physical.groups[group].len() as f64;
        }
        let mut collapsed_sigma = vec![0.0; tau.len() * tau.len()];
        for group in 0..tau.len() {
            collapsed_sigma[group * tau.len() + group] = tau[group];
        }
        for left in 0..3 {
            for right in 0..3 {
                let original = gaussian_covariance(
                    &physical.leaveout_kernel[left],
                    &physical.leaveout_kernel[right],
                    &sigma,
                    &physical_mean,
                );
                let scalar = gaussian_covariance(
                    &collapsed.leaveout_kernel[left],
                    &collapsed.leaveout_kernel[right],
                    &collapsed_sigma,
                    &collapsed_mean,
                );
                assert_close(original, scalar, 5.0e-10);
            }
        }
        if pattern == 3 {
            for value in tau {
                assert_close(value, 1.25, 2.0e-12);
            }
        }
    }
}

#[test]
fn physical_block_and_collapsed_scalar_q1_oracles_agree() {
    let rows = fixture();
    let (y_star, gamma) = full_sample_offset(&rows);
    assert!(gamma.iter().any(|value| value.abs() > 1.0e-3));
    let q = target_matrices(&rows);
    let physical = physical_block_oracle(&rows, &y_star, &q);
    let collapsed = collapsed_scalar_oracle(&rows, &y_star, &q);
    let physical_targets = reported_matrices(&physical.target_kernel);
    let collapsed_targets = reported_matrices(&collapsed.target_kernel);
    let physical_points = reported_matrices(&physical.leaveout_kernel);
    let collapsed_points = reported_matrices(&collapsed.leaveout_kernel);
    let map = collapse_map(&physical);
    let map_t = transpose(&map, physical.y.len(), collapsed.y.len());
    let signal_coefficients = vec![0.2, -0.1, 0.35, -0.25, 0.3, -0.15, 0.18];
    let physical_mean = matvec(&physical.x, physical.y.len(), 7, &signal_coefficients);
    let collapsed_mean = matvec(&map_t, collapsed.y.len(), physical.y.len(), &physical_mean);
    let mut differs_from_observationwise_recenter = false;

    for target in 0..4 {
        let (physical_eigenvalue, mut physical_mode) =
            symmetric_extreme_eigenpair(&physical_targets[target], physical.y.len());
        let (collapsed_eigenvalue, collapsed_mode) =
            symmetric_extreme_eigenpair(&collapsed_targets[target], collapsed.y.len());
        assert_close(physical_eigenvalue, collapsed_eigenvalue, 2.0e-10);
        let mut mapped_mode = matvec(&map_t, collapsed.y.len(), physical.y.len(), &physical_mode);
        let orientation = mapped_mode
            .iter()
            .zip(&collapsed_mode)
            .map(|(&left, &right)| left * right)
            .sum::<f64>();
        if orientation < 0.0 {
            for value in &mut physical_mode {
                *value = -*value;
            }
            for value in &mut mapped_mode {
                *value = -*value;
            }
        }
        assert!(max_abs_difference(&mapped_mode, &collapsed_mode) < 2.0e-9);

        let physical_score = physical_mode
            .iter()
            .zip(&physical.y)
            .map(|(&mode, &outcome)| mode * outcome)
            .sum::<f64>();
        let collapsed_score = collapsed_mode
            .iter()
            .zip(&collapsed.y)
            .map(|(&mode, &outcome)| mode * outcome)
            .sum::<f64>();
        assert_close(physical_score, collapsed_score, 2.0e-10);
        let physical_recenter = raw_physical_block_recenter(&physical, &physical_mode);
        let collapsed_recenter = raw_scalar_recenter(&collapsed, &collapsed_mode);
        assert_close(physical_recenter, collapsed_recenter, 4.0e-10);
        let observationwise_recenter =
            raw_physical_observationwise_recenter(&physical, &physical_mode);
        differs_from_observationwise_recenter |=
            (physical_recenter - observationwise_recenter).abs() > 1.0e-6;

        let physical_remainder_target = subtract_rank_one(
            &physical_targets[target],
            &physical_mode,
            physical_eigenvalue,
        );
        let collapsed_remainder_target = subtract_rank_one(
            &collapsed_targets[target],
            &collapsed_mode,
            collapsed_eigenvalue,
        );
        let physical_remainder = physical_block_leaveout_kernel(
            &physical_remainder_target,
            &physical.maker,
            &physical.groups,
            physical.y.len(),
        );
        let collapsed_remainder = scalar_leaveout_kernel(
            &collapsed_remainder_target,
            &collapsed.maker,
            collapsed.y.len(),
        );
        let compressed_remainder = multiply(
            &multiply(
                &map_t,
                collapsed.y.len(),
                physical.y.len(),
                &physical_remainder,
                physical.y.len(),
            ),
            collapsed.y.len(),
            physical.y.len(),
            &map,
            collapsed.y.len(),
        );
        assert!(max_abs_difference(&compressed_remainder, &collapsed_remainder) < 8.0e-10);

        let physical_point = quadratic(&physical.y, &physical_points[target]);
        let collapsed_point = quadratic(&collapsed.y, &collapsed_points[target]);
        assert_close(physical_point, collapsed_point, 3.0e-10);
        let physical_direct_remainder = quadratic(&physical.y, &physical_remainder);
        let collapsed_direct_remainder = quadratic(&collapsed.y, &collapsed_remainder);
        let physical_decomposition = physical_point
            - physical_eigenvalue * (physical_score * physical_score - physical_recenter);
        let collapsed_decomposition = collapsed_point
            - collapsed_eigenvalue * (collapsed_score * collapsed_score - collapsed_recenter);
        assert_close(physical_direct_remainder, physical_decomposition, 5.0e-10);
        assert_close(collapsed_direct_remainder, collapsed_decomposition, 5.0e-10);
        assert_close(
            physical_direct_remainder,
            collapsed_direct_remainder,
            5.0e-10,
        );

        for pattern in 0..4 {
            let blocks = covariance_blocks(pattern, &physical);
            let sigma = block_diagonal(&blocks, &physical.groups, physical.y.len());
            let mut collapsed_sigma = vec![0.0; collapsed.y.len() * collapsed.y.len()];
            for (group, block) in blocks.iter().enumerate() {
                collapsed_sigma[group * collapsed.y.len() + group] =
                    block.iter().sum::<f64>() / physical.groups[group].len() as f64;
            }
            let physical_covariance = q1_population_covariance(
                &physical_mode,
                &physical_remainder,
                &sigma,
                &physical_mean,
            );
            let collapsed_covariance = q1_population_covariance(
                &collapsed_mode,
                &collapsed_remainder,
                &collapsed_sigma,
                &collapsed_mean,
            );
            for (physical_value, collapsed_value) in
                physical_covariance.into_iter().zip(collapsed_covariance)
            {
                assert_close(physical_value, collapsed_value, 2.0e-8);
            }
        }
    }
    assert!(
        differs_from_observationwise_recenter,
        "whole-match recenter must retain within-block cross-products"
    );
}
