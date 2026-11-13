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

/// Ordinary least squares by Gaussian elimination with partial pivoting; an
/// oracle that shares nothing with the production solver.
fn least_squares(columns: &[Vec<f64>], outcome: &[f64]) -> Vec<f64> {
    let k = columns.len();
    let mut a = vec![vec![0.0; k + 1]; k];
    for i in 0..k {
        for j in 0..k {
            a[i][j] = columns[i].iter().zip(&columns[j]).map(|(x, y)| x * y).sum();
        }
        a[i][k] = columns[i].iter().zip(outcome).map(|(x, y)| x * y).sum();
    }
    for pivot in 0..k {
        let best = (pivot..k)
            .max_by(|&l, &r| a[l][pivot].abs().total_cmp(&a[r][pivot].abs()))
            .unwrap();
        a.swap(pivot, best);
        for row in 0..k {
            if row != pivot {
                let factor = a[row][pivot] / a[pivot][pivot];
                for column in pivot..=k {
                    a[row][column] -= factor * a[pivot][column];
                }
            }
        }
    }
    (0..k).map(|i| a[i][k] / a[i][i]).collect()
}

/// Midranks mapped to (-1, 1), written out independently of the production
/// helper.
fn ranks(values: &[f64]) -> Vec<f64> {
    let n = values.len();
    (0..n)
        .map(|i| {
            let below = values.iter().filter(|&&v| v < values[i]).count() as f64;
            let equal = values.iter().filter(|&&v| v == values[i]).count() as f64;
            let midrank = below + 0.5 * (equal + 1.0);
            2.0 * ((midrank - 0.5) / n as f64) - 1.0
        })
        .collect()
}

struct Units {
    proxy: Vec<f64>,
    outcome: Vec<f64>,
    residual: Vec<f64>,
    maker: Vec<f64>,
    leverage: Vec<f64>,
    mass: Vec<f64>,
    worker: Vec<u32>,
    worker_effect: Vec<f64>,
    firm_effect: Vec<f64>,
    diagonal: Vec<f64>,
    movers: usize,
    /// Derived by `input` as match deletion forms them: a mover unit is alone
    /// in its set, the remaining units are grouped by worker.
    set: Vec<u64>,
    mover: Vec<bool>,
    zero: Vec<f64>,
}

impl Units {
    fn input(&mut self) -> RepeatInputs<'_> {
        let rows = self.proxy.len();
        self.set = (0..rows)
            .map(|row| {
                if row < self.movers {
                    row as u64
                } else {
                    rows as u64 + u64::from(self.worker[row])
                }
            })
            .collect();
        self.mover = (0..rows).map(|row| row < self.movers).collect();
        self.zero = vec![0.0; rows];
        RepeatInputs {
            proxy: &self.proxy,
            outcome: &self.outcome,
            residual: &self.residual,
            maker_inverse: &self.maker,
            leverage: &self.leverage,
            mass: &self.mass,
            set: &self.set,
            mover: &self.mover,
            worker_effect: &self.worker_effect,
            firm_effect: &self.firm_effect,
            target_diagonal: [&self.diagonal, &self.zero, &self.zero],
            positivity_multiplier: 1.0e-8,
        }
    }
}

/// `movers` mover units followed by `stayers` workers with two observations.
fn mixed_units(movers: usize, stayers: usize, seed: u64) -> Units {
    let rows = movers + 2 * stayers;
    let mut state = seed;
    let mut units = Units {
        proxy: vec![0.0; rows],
        outcome: (0..rows).map(|_| normal(&mut state)).collect(),
        residual: vec![1.0; rows],
        maker: vec![2.0; rows],
        leverage: (0..rows)
            .map(|_| 0.05 + 0.9 * splitmix(&mut state))
            .collect(),
        mass: (0..rows)
            .map(|row| {
                if row < movers {
                    1.0 + (row % 6) as f64
                } else {
                    1.0
                }
            })
            .collect(),
        worker: (0..rows)
            .map(|row| {
                if row < movers {
                    (row / 2) as u32
                } else {
                    (movers + (row - movers) / 2) as u32
                }
            })
            .collect(),
        worker_effect: vec![0.0; rows],
        firm_effect: (0..rows).map(|_| 0.3 * normal(&mut state)).collect(),
        diagonal: vec![0.0; rows],
        movers,
        set: Vec::new(),
        mover: Vec::new(),
        zero: Vec::new(),
    };
    // One worker effect per worker, as in a fitted model.
    let workers = units.worker.iter().copied().max().unwrap() as usize + 1;
    let effect = (0..workers)
        .map(|_| 0.5 * normal(&mut state))
        .collect::<Vec<_>>();
    for row in 0..rows {
        units.worker_effect[row] = effect[units.worker[row] as usize];
    }
    units
}

/// The two steps written directly from their definition.
fn two_step_oracle(units: &Units) -> (Vec<f64>, Vec<f64>) {
    let m = units.movers;
    let rows = units.proxy.len();
    let lever = ranks(&units.leverage[..m]);
    let mass = ranks(&units.mass[..m]);
    let z = vec![
        vec![1.0; m],
        lever.clone(),
        mass.clone(),
        lever.iter().map(|v| v * v).collect(),
        mass.iter().map(|v| v * v).collect(),
    ];
    let c = least_squares(&z, &units.proxy[..m]);
    let mean = units.proxy[..m].iter().sum::<f64>() / m as f64;
    let scale = (0..m)
        .map(|i| {
            (0..5)
                .map(|k| z[k][i] * c[k])
                .sum::<f64>()
                .max(SCALE_FLOOR_SHARE * mean)
        })
        .collect::<Vec<_>>();
    let standard = |values: &[f64]| -> Vec<f64> {
        let total = units.mass.iter().sum::<f64>();
        let mean = (0..rows).map(|i| units.mass[i] * values[i]).sum::<f64>() / total;
        let deviation = ((0..rows)
            .map(|i| units.mass[i] * (values[i] - mean).powi(2))
            .sum::<f64>()
            / total)
            .sqrt();
        values
            .iter()
            .map(|v| ((v - mean) / deviation).clamp(-PROFILE_CAP, PROFILE_CAP))
            .collect()
    };
    let a = standard(&units.worker_effect);
    let p = standard(&units.firm_effect);
    let basis = |i: usize| [1.0, a[i], a[i] * a[i], p[i], p[i] * p[i], a[i] * p[i]];
    let x = (0..PROFILE_TERMS)
        .map(|k| (0..m).map(|i| scale[i] * basis(i)[k]).collect::<Vec<_>>())
        .collect::<Vec<_>>();
    let d = least_squares(&x, &units.proxy[..m]);
    let fitted = (0..m)
        .map(|i| (0..PROFILE_TERMS).map(|k| x[k][i] * d[k]).sum())
        .collect();
    (fitted, d)
}

#[test]
fn stayers_receive_their_own_sample_variance_and_the_pair_term() {
    // Workers 7, 3 and 9 with two, three and four observations, interleaved.
    let worker = vec![7, 3, 9, 3, 7, 9, 3, 9, 9];
    let outcome = vec![1.0, 0.0, 5.0, 3.0, 3.0, 5.0, 6.0, 5.0, 5.0];
    let diagonal = vec![0.1, 0.2, 0.3, 0.2, 0.1, 0.3, 0.2, 0.3, 0.3];
    let rows = worker.len();
    let mut units = Units {
        proxy: vec![0.0; rows],
        outcome,
        residual: vec![1.0; rows],
        maker: vec![2.0; rows],
        leverage: vec![0.5; rows],
        mass: vec![1.0; rows],
        worker,
        worker_effect: vec![0.0; rows],
        firm_effect: vec![0.0; rows],
        diagonal,
        movers: 0,
        set: Vec::new(),
        mover: Vec::new(),
        zero: Vec::new(),
    };
    let fit = fit_repeat_variance(units.input(), &mut NeverInterrupt).unwrap();
    let floor = 2.0e-8;
    assert_eq!(fit.floor, floor);
    // Worker 7: (1, 3), variance 2. Worker 3: (0, 3, 6), variance 9. Worker 9
    // is constant: its pseudo-noise variance is the floor, its own is zero.
    let expected = [2.0, 9.0, floor, 9.0, 2.0, floor, 9.0, floor, floor];
    for (value, expected) in fit.variance.iter().zip(expected) {
        assert!((value - expected).abs() <= 1.0e-14 * expected.max(1.0));
    }
    assert_eq!(
        (fit.stayers, fit.stayer_units, fit.stayer_floored),
        (3, 9, 4)
    );
    assert_eq!((fit.scale_terms, fit.profile_terms, fit.floored), (0, 0, 0));
    // Two observations contribute nothing. Three: pair weight 0.2 * 3 / 2 and
    // 4 * 0.3^2 * 9^2 * (1 * 2 / 4).
    let pair: f64 = 0.2 * 3.0 / 2.0;
    let term = 4.0 * pair.powi(2) * 81.0 * 0.5;
    assert!((fit.adjustment[0] - term).abs() <= 1.0e-12 * term);
    assert!(fit.adjustment[1..].iter().all(|&value| value == 0.0));
}

#[test]
fn a_stayer_needs_two_observations() {
    let mut units = mixed_units(0, 2, 5);
    units.worker[3] = 99;
    let error = fit_repeat_variance(units.input(), &mut NeverInterrupt).unwrap_err();
    assert_eq!(error.code, ErrorCode::InternalInvariantFailed);
}

#[test]
fn mover_model_matches_the_two_steps_written_out() {
    let mut units = mixed_units(1_200, 150, 11);
    let mut state = 23;
    // Arbitrary positive leave-out products with noise: nothing is in the span.
    for row in 0..units.movers {
        units.proxy[row] = (1.0
            + 0.4 * units.leverage[row]
            + 0.1 * units.mass[row]
            + 0.3 * units.worker_effect[row].powi(2))
            * (1.0 + 0.5 * normal(&mut state));
    }
    let fit = fit_repeat_variance(units.input(), &mut NeverInterrupt).unwrap();
    let (expected, coefficient) = two_step_oracle(&units);
    assert_eq!((fit.scale_terms, fit.profile_terms), (5, 6));
    for term in 0..PROFILE_TERMS {
        assert!((fit.profile[term] - coefficient[term]).abs() <= 1.0e-9);
    }
    for row in 0..units.movers {
        assert!(
            (fit.variance[row] - expected[row].max(fit.floor)).abs() <= 1.0e-9,
            "row {row}: {} against {}",
            fit.variance[row],
            expected[row]
        );
    }
    assert_eq!(fit.stayers, 150);

    // The effects are identified up to a common shift between workers and
    // firms; the variances must not depend on it.
    let mut shifted = mixed_units(1_200, 150, 11);
    shifted.proxy.clone_from(&units.proxy);
    for row in 0..shifted.proxy.len() {
        shifted.worker_effect[row] += 3.5;
        shifted.firm_effect[row] -= 3.5;
    }
    let again = fit_repeat_variance(shifted.input(), &mut NeverInterrupt).unwrap();
    for row in 0..units.proxy.len() {
        assert!((again.variance[row] - fit.variance[row]).abs() <= 1.0e-9);
    }
}

#[test]
fn mover_model_recovers_a_scale_in_its_span_with_a_flat_profile() {
    let mut units = mixed_units(900, 0, 3);
    let lever = ranks(&units.leverage);
    let mass = ranks(&units.mass);
    for row in 0..units.movers {
        units.proxy[row] = 1.0 + 0.3 * lever[row] + 0.2 * mass[row] * mass[row];
    }
    let fit = fit_repeat_variance(units.input(), &mut NeverInterrupt).unwrap();
    for row in 0..units.movers {
        assert!((fit.variance[row] - units.proxy[row]).abs() <= 1.0e-10);
    }
    assert!((fit.profile[0] - 1.0).abs() <= 1.0e-10);
    assert!(fit.profile[1..].iter().all(|value| value.abs() <= 1.0e-10));
    assert_eq!((fit.scale_floored, fit.floored), (0, 0));
}

#[test]
fn small_mover_samples_shorten_both_bases() {
    let mut units = mixed_units(150, 0, 9);
    let mut state = 41;
    for row in 0..units.movers {
        units.proxy[row] = 2.0 + normal(&mut state);
    }
    let fit = fit_repeat_variance(units.input(), &mut NeverInterrupt).unwrap();
    assert_eq!((fit.scale_terms, fit.profile_terms), (1, 1));
    let mean = units.proxy.iter().sum::<f64>() / 150.0;
    assert!(fit
        .variance
        .iter()
        .all(|value| (value - mean).abs() <= 1.0e-12));
}

#[test]
fn floors_are_applied_and_counted() {
    let mut units = mixed_units(600, 0, 13);
    let lever = ranks(&units.leverage);
    // A scale that crosses zero: the fitted scale is floored at five per
    // cent of the mean product, and the result stays positive.
    for row in 0..units.movers {
        units.proxy[row] = 0.4 + lever[row];
    }
    let fit = fit_repeat_variance(units.input(), &mut NeverInterrupt).unwrap();
    assert!(fit.scale_floored > 0);
    let mean = units.proxy.iter().sum::<f64>() / 600.0;
    assert!((fit.scale_floor - SCALE_FLOOR_SHARE * mean).abs() <= 1.0e-15);
    assert!(fit.variance.iter().all(|&value| value >= fit.floor));

    // A nonpositive mean product cannot be scaled.
    for row in 0..units.movers {
        units.proxy[row] = -1.0;
    }
    let error = fit_repeat_variance(units.input(), &mut NeverInterrupt).unwrap_err();
    assert_eq!(error.code, ErrorCode::JlaConstraintFailed);
}

#[test]
fn covariance_is_the_high_rank_form_plus_the_pair_term_without_truncation() {
    let influence = [
        vec![1.0, -2.0, 0.5],
        vec![0.5, 0.25, -1.0],
        vec![-1.0, 0.0, 2.0],
    ];
    let variance = [2.0, 0.5, 1.5];
    let mut trace = [0.0; SQUARE];
    let mut mcse = [0.0; SQUARE];
    // A large trace for the firm variance drives that diagonal negative.
    let values = [[0.7, 0.1, -0.2], [0.1, 9.0, 0.05], [-0.2, 0.05, 0.3]];
    for left in 0..3 {
        for right in 0..3 {
            trace[left * 3 + right] = values[left][right];
            mcse[left * 3 + right] = 0.01;
        }
    }
    let moments = NoiseMoments {
        trace,
        trace_mcse: mcse,
        residual_noise: [0.0; SQUARE],
        residual_noise_mcse: [0.0; SQUARE],
        draws: 16,
        effective_rank: [f64::NAN; REPORTED_TARGETS],
    };
    let mut adjustment = [0.0; SQUARE];
    adjustment[0] = 0.25;
    let covariance = finish_repeat_covariance(
        &influence,
        &variance,
        &adjustment,
        &moments,
        &mut NeverInterrupt,
    )
    .unwrap();
    for left in 0..3 {
        for right in 0..3 {
            let index = left * 3 + right;
            let leading = (0..3)
                .map(|row| 4.0 * variance[row] * influence[left][row] * influence[right][row])
                .sum::<f64>();
            let expected = leading - 2.0 * values[left][right] + adjustment[index];
            assert!((covariance.primitive[index] - expected).abs() <= 1.0e-13);
            assert!(
                (covariance.signal[index] + covariance.trace_term[index] - expected).abs()
                    <= 1.0e-13
            );
            assert_eq!(covariance.trace_term[index], 2.0 * values[left][right]);
            assert_eq!(covariance.trace_mcse[index], 0.02);
        }
    }
    assert_eq!(covariance.model_primitive, covariance.primitive);
    assert_eq!(covariance.signal_truncation, 0.0);
    // 4 * (2 * 0.25 + 0.5 * 0.0625 + 1.5) - 18 < 0 is reported, not repaired.
    assert!(covariance.primitive[4] < 0.0);
    // Largest unit share of 4 v (K z)^2 for the worker variance.
    let shares = [8.0, 8.0, 1.5];
    assert!(
        (covariance.influence_concentration[0] - 8.0 / shares.iter().sum::<f64>()).abs() <= 1.0e-14
    );
}

/// Physical observations as units, as observation deletion forms them.
struct Observations {
    proxy: Vec<f64>,
    outcome: Vec<f64>,
    residual: Vec<f64>,
    maker: Vec<f64>,
    leverage: Vec<f64>,
    mass: Vec<f64>,
    set: Vec<u64>,
    mover: Vec<bool>,
    effect: Vec<f64>,
    diagonal: [Vec<f64>; 3],
}

impl Observations {
    fn input(&self) -> RepeatInputs<'_> {
        RepeatInputs {
            proxy: &self.proxy,
            outcome: &self.outcome,
            residual: &self.residual,
            maker_inverse: &self.maker,
            leverage: &self.leverage,
            mass: &self.mass,
            set: &self.set,
            mover: &self.mover,
            worker_effect: &self.effect,
            firm_effect: &self.effect,
            target_diagonal: [&self.diagonal[0], &self.diagonal[1], &self.diagonal[2]],
            positivity_multiplier: 1.0e-8,
        }
    }
}

#[test]
fn a_set_of_a_mover_reaches_all_three_kernels() {
    // Cell 4 of a mover has the three observations (0, 3, 6), interleaved
    // with a stayer's cell 9 of three observations (1, 2, 6) and two units of
    // the mover that are alone in their cells.
    let set = vec![4, 9, 4, 9, 2, 4, 9, 7];
    let mover = vec![true, false, true, false, true, true, false, true];
    let outcome = vec![0.0, 1.0, 3.0, 2.0, 5.0, 6.0, 6.0, -1.0];
    let rows = set.len();
    let units = Observations {
        proxy: vec![0.5; rows],
        outcome,
        residual: vec![1.0; rows],
        maker: vec![1.25; rows],
        leverage: vec![0.2; rows],
        mass: vec![1.0; rows],
        set,
        mover,
        effect: vec![0.0; rows],
        diagonal: [vec![0.2; rows], vec![0.1; rows], vec![-0.05; rows]],
    };
    let fit = fit_repeat_variance(units.input(), &mut NeverInterrupt).unwrap();
    // Sample variances 9 and 7; the two single units share the mean product.
    let expected = [9.0, 7.0, 9.0, 7.0, 0.5, 9.0, 7.0, 0.5];
    for (value, expected) in fit.variance.iter().zip(expected) {
        assert!((value - expected).abs() <= 1.0e-14 * expected);
    }
    assert_eq!((fit.stayers, fit.stayer_units), (2, 6));
    assert_eq!((fit.model_units, fit.model_fit_units), (2, 5));
    // Mover set: weights B / (1 - P) = 1.25 * (0.2, 0.1, -0.05) and the
    // factor 4 * 9^2 * (1 * 2 / 4). Stayer set: weight 0.2 * 3 / 2 for the
    // worker variance only, factor 4 * 7^2 * (1 * 2 / 4).
    let weight = [0.25, 0.125, -0.0625];
    let stayer: f64 = 0.2 * 3.0 / 2.0;
    for left in 0..3 {
        for right in 0..3 {
            let mut term = 4.0 * 81.0 * 0.5 * weight[left] * weight[right];
            if left == 0 && right == 0 {
                term += 4.0 * 49.0 * 0.5 * stayer * stayer;
            }
            assert!(
                (fit.adjustment[left * 3 + right] - term).abs() <= 1.0e-12 * term.abs(),
                "entry {left},{right}: {} against {term}",
                fit.adjustment[left * 3 + right]
            );
        }
    }
}

#[test]
fn few_single_units_borrow_the_units_of_all_movers() {
    // Twenty single units and thirty pairs of movers: fewer than a hundred
    // single units cannot carry the model, so it is fitted on all eighty
    // units of movers and, with that many, reduces to their mean product.
    let rows = 80;
    let mut state = 77;
    let mut units = Observations {
        proxy: (0..rows).map(|_| 1.0 + 0.2 * normal(&mut state)).collect(),
        outcome: (0..rows).map(|_| normal(&mut state)).collect(),
        residual: vec![1.0; rows],
        maker: vec![1.5; rows],
        leverage: (0..rows)
            .map(|_| 0.05 + 0.4 * splitmix(&mut state))
            .collect(),
        mass: vec![1.0; rows],
        set: (0..rows as u64)
            .map(|row| if row < 20 { row } else { 100 + (row - 20) / 2 })
            .collect(),
        mover: vec![true; rows],
        effect: (0..rows).map(|_| normal(&mut state)).collect(),
        diagonal: [vec![0.0; rows], vec![0.0; rows], vec![0.0; rows]],
    };
    let fit = fit_repeat_variance(units.input(), &mut NeverInterrupt).unwrap();
    let mean = units.proxy.iter().sum::<f64>() / rows as f64;
    assert_eq!((fit.scale_terms, fit.profile_terms), (1, 1));
    assert_eq!((fit.model_units, fit.model_fit_units), (20, 80));
    assert_eq!((fit.stayers, fit.stayer_units), (30, 60));
    for row in 0..20 {
        assert!((fit.variance[row] - mean).abs() <= 1.0e-12);
    }
    // With a hundred single units the model is fitted on them alone.
    let rows = 160;
    units.proxy = (0..rows)
        .map(|row| if row < 100 { 2.0 } else { 5.0 })
        .collect();
    units.outcome = (0..rows).map(|_| normal(&mut state)).collect();
    units.residual = vec![1.0; rows];
    units.maker = vec![1.5; rows];
    units.leverage = (0..rows)
        .map(|_| 0.05 + 0.4 * splitmix(&mut state))
        .collect();
    units.mass = vec![1.0; rows];
    units.set = (0..rows as u64)
        .map(|row| {
            if row < 100 {
                row
            } else {
                1_000 + (row - 100) / 2
            }
        })
        .collect();
    units.mover = vec![true; rows];
    units.effect = (0..rows).map(|_| normal(&mut state)).collect();
    units.diagonal = [vec![0.0; rows], vec![0.0; rows], vec![0.0; rows]];
    let fit = fit_repeat_variance(units.input(), &mut NeverInterrupt).unwrap();
    assert_eq!((fit.model_units, fit.model_fit_units), (100, 100));
    for row in 0..100 {
        assert!((fit.variance[row] - 2.0).abs() <= 1.0e-12);
    }
}
