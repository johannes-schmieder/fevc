// SPDX-License-Identifier: MIT

use super::*;
use crate::interrupt::NeverInterrupt;
use crate::residual_noise::NoiseAccumulator;

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
    let (covariance, terms) = finish_repeat_covariance(
        &influence,
        &variance,
        &adjustment,
        &moments,
        None,
        &mut NeverInterrupt,
    )
    .unwrap();
    assert!(terms.is_none());
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

#[test]
fn remainder_pair_term_nets_the_direction_out_of_the_weights() {
    // The fixture of the mover-set test: cell 4 of a mover (rows 0, 2, 5,
    // sample variance 9) and a stayer's cell 9 (rows 1, 3, 6, variance 7).
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
    assert_eq!(fit.sets.end, vec![3, 6]);
    assert_eq!(fit.sets.members, vec![0, 2, 5, 1, 3, 6]);
    assert_eq!(fit.sets.mover, vec![true, false]);
    let mode = vec![0.3, 0.1, 0.3, 0.1, 0.7, 0.3, 0.1, -0.2];
    let eigenvalue = 0.5;
    // Without a direction the term is the diagonal entry of the pair term.
    for (target, stayer) in [(0, true), (1, false), (2, false)] {
        let plain = fit
            .remainder_pair_term(
                &units.diagonal[target],
                &units.maker,
                &vec![0.0; rows],
                eigenvalue,
                stayer,
                &mut NeverInterrupt,
            )
            .unwrap();
        let expected = fit.adjustment[target * 4];
        assert!((plain - expected).abs() <= 1.0e-13 * expected.abs().max(1.0));
    }
    // Mover set: (0.2 - 0.5 * 0.09) * 1.25; stayer: (0.2 - 0.5 * 0.01) * 3/2.
    let weight_mover: f64 = (0.2 - 0.045) * 1.25;
    let weight_stayer: f64 = (0.2 - 0.005) * 1.5;
    let expected =
        4.0 * 81.0 * 0.5 * weight_mover.powi(2) + 4.0 * 49.0 * 0.5 * weight_stayer.powi(2);
    let value = fit
        .remainder_pair_term(
            &units.diagonal[0],
            &units.maker,
            &mode,
            eigenvalue,
            true,
            &mut NeverInterrupt,
        )
        .unwrap();
    assert!((value - expected).abs() <= 1.0e-12 * expected);
    // A target without a stayer diagonal keeps the direction's own part.
    let weight_mover: f64 = (0.1 - 0.045) * 1.25;
    let weight_stayer: f64 = -0.005 * 1.5;
    let expected =
        4.0 * 81.0 * 0.5 * weight_mover.powi(2) + 4.0 * 49.0 * 0.5 * weight_stayer.powi(2);
    let value = fit
        .remainder_pair_term(
            &units.diagonal[1],
            &units.maker,
            &mode,
            eigenvalue,
            false,
            &mut NeverInterrupt,
        )
        .unwrap();
    assert!((value - expected).abs() <= 1.0e-12 * expected);
}

#[test]
fn q1_target_uses_the_larger_leading_variance_and_the_remainder_pair_term() {
    let mode = [0.6, 0.0, 0.8];
    let influence = [0.5, -1.0, 0.25];
    let variance = [2.0, 0.5, 1.0];
    let (eigenvalue, score, point) = (0.4_f64, 1.5_f64, 3.0_f64);
    let fitted: f64 = 0.36 * 2.0 + 0.64;
    let linear: f64 = 4.0 * (0.25 * 2.0 + 0.5 + 0.0625);
    let cross: f64 = 2.0 * (0.6 * 2.0 * 0.5 + 0.8 * 0.25);
    for leave_out in [0.5 * fitted, 3.0 * fitted] {
        let remainder = point - eigenvalue * (score * score - leave_out);
        let result = finish_repeat_q1_target(
            point, score, leave_out, remainder, 1.0e-12, eigenvalue, &mode, &influence, &variance,
            0.3, 0.01, 0.2, 1.0e-8, None,
        )
        .unwrap();
        let leading = fitted.max(leave_out);
        let remainder_variance = linear - 0.6 + 0.2;
        assert_eq!(result.status, ComponentQ1Status::Computed);
        assert!((result.leading_variance - leading).abs() <= 1.0e-14);
        assert_eq!(result.leading_variance_correction, leave_out);
        assert!((result.remainder_variance - remainder_variance).abs() <= 1.0e-14);
        assert!((result.remainder_estimate - remainder).abs() <= 1.0e-14);
        assert!((result.leading_remainder_covariance - cross).abs() <= 1.0e-14);
        assert_eq!(result.remainder_trace_mcse, 0.02);
        let correlation = cross / (leading * remainder_variance).sqrt();
        let curvature = 2.0 * eigenvalue * leading
            / (remainder_variance * (1.0 - correlation * correlation)).sqrt();
        assert!((result.curvature - curvature).abs() <= 1.0e-12);
    }
    // A remainder variance that is not positive is a status, not an error.
    let result = finish_repeat_q1_target(
        point,
        score,
        fitted,
        point - eigenvalue * (score * score - fitted),
        1.0e-12,
        eigenvalue,
        &mode,
        &influence,
        &variance,
        5.0,
        0.01,
        0.0,
        1.0e-8,
        None,
    )
    .unwrap();
    assert_eq!(result.status, ComponentQ1Status::NonpositiveVariance);
    assert!(result.curvature.is_nan());
    // A decomposition that does not reproduce the direct remainder is refused.
    let error = finish_repeat_q1_target(
        point, score, fitted, 0.0, 1.0e-12, eigenvalue, &mode, &influence, &variance, 0.3, 0.01,
        0.2, 1.0e-8, None,
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::TargetIdentityFailed);
}

/// Four units and two controls with nonzero within-unit products on three of
/// them, written as dense matrices: `Zc`, `Zt` and `W` are n x k row-major.
struct ControlFixture {
    adjustment: ControlAdjustment,
    zc: [[f64; 2]; 4],
    zt: [[f64; 2]; 4],
    w: [[f64; 2]; 4],
    lam: [[f64; 2]; 2],
    residual: [f64; 4],
    maker: [f64; 4],
}

fn control_fixture() -> ControlFixture {
    let zc = [[1.0, -0.5], [0.3, 2.0], [-1.2, 0.4], [0.7, 0.1]];
    let zt = [[0.4, -0.2], [-0.1, 0.9], [-0.6, 0.3], [0.3, -1.0]];
    let w = [[0.25, -0.1], [0.0, 0.0], [-0.3, 0.6], [0.15, 0.2]];
    let lam = [[0.8, -0.15], [-0.15, 0.5]];
    let control_major = |m: &[[f64; 2]; 4]| -> Vec<f64> {
        (0..2).flat_map(|j| (0..4).map(move |a| m[a][j])).collect()
    };
    ControlFixture {
        adjustment: ControlAdjustment {
            controls: 2,
            collapsed: control_major(&zc),
            residualized: control_major(&zt),
            within: control_major(&w),
            precision: vec![lam[0][0], lam[0][1], lam[1][0], lam[1][1]],
        },
        zc,
        zt,
        w,
        lam,
        residual: [0.5, -1.0, 0.2, 0.8],
        maker: [1.25, 1.5, 2.0, 1.1],
    }
}

impl ControlFixture {
    /// The dense projection `Pi = I - Zt Lam Zc'`.
    fn pi(&self) -> [[f64; 4]; 4] {
        let mut pi = [[0.0; 4]; 4];
        for a in 0..4 {
            for b in 0..4 {
                let mut value = f64::from(a == b);
                for j in 0..2 {
                    for l in 0..2 {
                        value -= self.zt[a][j] * self.lam[j][l] * self.zc[b][l];
                    }
                }
                pi[a][b] = value;
            }
        }
        pi
    }

    /// `Lam Zc' h`.
    fn coefficient(&self, h: &[f64]) -> [f64; 2] {
        let slope: [f64; 2] = core::array::from_fn(|j| (0..4).map(|a| self.zc[a][j] * h[a]).sum());
        core::array::from_fn(|j| (0..2).map(|l| self.lam[j][l] * slope[l]).sum())
    }

    fn apply(&self, matrix: &[[f64; 4]; 4], h: &[f64]) -> Vec<f64> {
        (0..4)
            .map(|a| (0..4).map(|b| matrix[a][b] * h[b]).sum())
            .collect()
    }

    fn within(&self, load: &[f64; 2]) -> Vec<f64> {
        (0..4)
            .map(|a| self.w[a][0] * load[0] + self.w[a][1] * load[1])
            .collect()
    }

    fn omega(&self, left: &[f64; 2], right: &[f64; 2]) -> f64 {
        let mut value = 0.0;
        for j in 0..2 {
            for l in 0..2 {
                let entry: f64 = (0..4).map(|a| self.w[a][j] * self.w[a][l]).sum();
                value += left[j] * entry * right[l];
            }
        }
        value
    }
}

#[test]
fn transformed_draws_carry_the_covariance_of_the_adjusted_noise() {
    let fixture = control_fixture();
    let control = RepeatControl::new(
        &fixture.adjustment,
        &fixture.residual,
        &fixture.maker,
        &mut NeverInterrupt,
    )
    .unwrap();
    // Unit 1 has no within-unit product and needs no extra normal.
    assert_eq!(control.within_units(), &[0, 2, 3]);
    let pi = fixture.pi();
    // The map is linear: zeta -> Pi' zeta, xi -> -Zc Lam W' xi.
    for unit in 0..4 {
        let mut draw = vec![0.0; 4];
        draw[unit] = 1.0;
        control.transform_draw(&mut draw, &[0.0; 3]).unwrap();
        for row in 0..4 {
            assert!((draw[row] - pi[unit][row]).abs() <= 1.0e-15, "{unit} {row}");
        }
    }
    for (position, &unit) in [0_usize, 2, 3].iter().enumerate() {
        let mut draw = vec![0.0; 4];
        let mut xi = [0.0; 3];
        xi[position] = 1.0;
        control.transform_draw(&mut draw, &xi).unwrap();
        let load: [f64; 2] =
            core::array::from_fn(|j| (0..2).map(|l| fixture.lam[j][l] * fixture.w[unit][l]).sum());
        for row in 0..4 {
            let expected = -(0..2).map(|j| fixture.zc[row][j] * load[j]).sum::<f64>();
            assert!((draw[row] - expected).abs() <= 1.0e-15);
        }
    }
    assert!(control.transform_draw(&mut [0.0; 4], &[0.0; 2]).is_err());
}

#[test]
fn control_term_enters_the_high_rank_covariance() {
    let fixture = control_fixture();
    let control = RepeatControl::new(
        &fixture.adjustment,
        &fixture.residual,
        &fixture.maker,
        &mut NeverInterrupt,
    )
    .unwrap();
    assert_eq!(control.controls(), 2);
    let influence = [
        vec![1.0, -2.0, 0.5, 0.3],
        vec![0.5, 0.25, -1.0, 0.8],
        vec![-1.0, 0.0, 2.0, -0.4],
    ];
    let variance = [2.0, 0.5, 1.5, 0.9];
    // Three pseudo-noise images per draw, two draws.
    let images = [
        [
            vec![0.3, -0.2, 0.9, 0.1],
            vec![-0.5, 0.4, 0.2, 0.6],
            vec![0.7, 0.1, -0.3, -0.2],
        ],
        [
            vec![-0.1, 0.8, 0.4, -0.6],
            vec![0.2, -0.3, 0.5, 0.9],
            vec![0.6, 0.6, -0.1, 0.3],
        ],
    ];
    let mut accumulator = NoiseAccumulator::default();
    for draw in &images {
        let mut projected = core::array::from_fn(|_| Vec::new());
        let mut coefficient = core::array::from_fn(|_| Vec::new());
        for target in 0..3 {
            let image = control.image(&draw[target]).unwrap();
            projected[target] = image.projected;
            coefficient[target] = image.coefficient;
        }
        accumulator
            .push_control(
                &variance,
                &[1.0; 4],
                &[0.0; 4],
                projected,
                coefficient,
                control.omega(),
                &mut NeverInterrupt,
            )
            .unwrap();
    }
    let moments = accumulator.finish().unwrap();
    assert_eq!(moments.draws, 2);
    // The bilinear form sum_a v_a (Pi x)_a (Pi y)_a + (Lam Zc' x)' W'W (Lam Zc' y).
    let pi = fixture.pi();
    let form = |x: &[f64], y: &[f64]| {
        let (px, py) = (fixture.apply(&pi, x), fixture.apply(&pi, y));
        (0..4).map(|a| variance[a] * px[a] * py[a]).sum::<f64>()
            + fixture.omega(&fixture.coefficient(x), &fixture.coefficient(y))
    };
    for left in 0..3 {
        for right in 0..3 {
            let expected = images
                .iter()
                .map(|draw| form(&draw[left], &draw[right]))
                .sum::<f64>()
                / 2.0;
            assert!((moments.trace[left * 3 + right] - expected).abs() <= 1.0e-14);
        }
    }
    // The consecutive-draw product of the effective rank takes the same form.
    let pair = form(&images[0][0], &images[1][0]);
    let rank = moments.trace[0] * moments.trace[0] / (pair * pair);
    assert!((moments.effective_rank[0] - rank.max(1.0)).abs() <= 1.0e-12 * rank);
    for draw in &images {
        assert!(
            (control.image_square(&variance, &draw[1]).unwrap() - form(&draw[1], &draw[1])).abs()
                <= 1.0e-14
        );
    }

    let mut adjustment = [0.0; SQUARE];
    adjustment[0] = 0.25;
    let (covariance, terms) = finish_repeat_covariance(
        &influence,
        &variance,
        &adjustment,
        &moments,
        Some(&control),
        &mut NeverInterrupt,
    )
    .unwrap();
    let terms = terms.expect("control terms");
    // Cov[L_s, L_t] = sum 4 m*_s m*_t v + 2 m*_s e~ w'c_t + 2 m*_t e~ w'c_s
    // + c_s' w w' c_t, with m* = Pi g and c = -2 Lam Zc' g.
    let lo: Vec<f64> = (0..4)
        .map(|a| fixture.residual[a] * fixture.maker[a])
        .collect();
    let star: Vec<Vec<f64>> = influence.iter().map(|g| fixture.apply(&pi, g)).collect();
    let load: Vec<[f64; 2]> = influence
        .iter()
        .map(|g| fixture.coefficient(g).map(|value| -2.0 * value))
        .collect();
    let within: Vec<Vec<f64>> = load.iter().map(|c| fixture.within(c)).collect();
    for left in 0..3 {
        for right in 0..3 {
            let index = left * 3 + right;
            let unit: f64 = (0..4)
                .map(|a| 4.0 * star[left][a] * star[right][a] * variance[a])
                .sum();
            let cross: f64 = (0..4)
                .map(|a| {
                    2.0 * star[left][a] * lo[a] * within[right][a]
                        + 2.0 * star[right][a] * lo[a] * within[left][a]
                })
                .sum();
            let inside: f64 = (0..4).map(|a| within[left][a] * within[right][a]).sum();
            assert!((terms.linear_unit[index] - unit).abs() <= 1.0e-13);
            assert!((terms.linear_cross[index] - cross).abs() <= 1.0e-13);
            assert!((terms.linear_within[index] - inside).abs() <= 1.0e-13);
            let signal = unit + cross + inside - 4.0 * moments.trace[index] + adjustment[index];
            assert!((covariance.signal[index] - signal).abs() <= 1.0e-13);
            assert!(
                (covariance.primitive[index] - signal - 2.0 * moments.trace[index]).abs()
                    <= 1.0e-13
            );
            assert_eq!(covariance.trace_term[index], 2.0 * moments.trace[index]);
        }
    }
    // The within-unit part matters here.
    assert!(within[0].iter().any(|value| value.abs() > 0.1));
    assert_eq!(covariance.model_primitive, covariance.primitive);
    // A large quadratic form drives a diagonal negative: reported, not
    // repaired and not an error.
    let mut large = moments.clone();
    large.trace[4] = 50.0;
    let (negative, _) = finish_repeat_covariance(
        &influence,
        &variance,
        &adjustment,
        &large,
        Some(&control),
        &mut NeverInterrupt,
    )
    .unwrap();
    assert!(negative.primitive[4] < 0.0);
    assert_eq!(negative.signal_truncation, 0.0);
    assert!(
        (negative.primitive[4]
            - (terms.linear_unit[4] + terms.linear_cross[4] + terms.linear_within[4] - 100.0))
            .abs()
            <= 1.0e-12
    );
    // One accumulator does not mix the two forms.
    let mut mixed = NoiseAccumulator::default();
    mixed
        .push(
            &variance,
            &[1.0; 4],
            &[0.0; 4],
            images[0].clone(),
            &mut NeverInterrupt,
        )
        .unwrap();
    assert!(mixed
        .push_control(
            &variance,
            &[1.0; 4],
            &[0.0; 4],
            images[1].clone(),
            core::array::from_fn(|_| vec![0.0; 2]),
            control.omega(),
            &mut NeverInterrupt,
        )
        .is_err());
}

#[test]
fn control_term_enters_the_q1_pieces() {
    let fixture = control_fixture();
    let control = RepeatControl::new(
        &fixture.adjustment,
        &fixture.residual,
        &fixture.maker,
        &mut NeverInterrupt,
    )
    .unwrap();
    let mode = [0.6, 0.0, 0.8, 0.0];
    let influence = [0.5, -1.0, 0.25, 0.4];
    let variance = [2.0, 0.5, 1.0, 1.5];
    let proxy = [1.5, 0.2, 0.9, 1.1];
    let (eigenvalue, score, point) = (0.4_f64, 1.5_f64, 3.0_f64);
    let pi = fixture.pi();
    let lo: Vec<f64> = (0..4)
        .map(|a| fixture.residual[a] * fixture.maker[a])
        .collect();
    let ds = fixture.apply(&pi, &mode);
    let cb = fixture.coefficient(&mode).map(|value| -value);
    let wb = fixture.within(&cb);
    let rs = fixture.apply(&pi, &influence);
    let cr = fixture.coefficient(&influence).map(|value| -2.0 * value);
    let wr = fixture.within(&cr);
    let sum = |f: &dyn Fn(usize) -> f64| (0..4).map(f).sum::<f64>();
    let fitted =
        sum(&|a| variance[a] * ds[a] * ds[a] + 2.0 * lo[a] * ds[a] * wb[a] + wb[a] * wb[a]);
    let leave_out_score =
        sum(&|a| proxy[a] * ds[a] * ds[a] + 2.0 * lo[a] * ds[a] * wb[a] + wb[a] * wb[a]);
    let cross = sum(&|a| {
        2.0 * variance[a] * ds[a] * rs[a]
            + lo[a] * (ds[a] * wr[a] + 2.0 * rs[a] * wb[a])
            + wb[a] * wr[a]
    });
    let linear =
        sum(&|a| 4.0 * variance[a] * rs[a] * rs[a] + 4.0 * lo[a] * rs[a] * wr[a] + wr[a] * wr[a]);
    assert!(wb.iter().chain(&wr).any(|value| value.abs() > 0.05));
    // The identity's leave-out correction of the leading square stays d^2 s.
    let identity: f64 = sum(&|a| mode[a] * mode[a] * proxy[a]);
    // `rtrace` stands for the bilinear form of the remainder images.
    let (rtrace, pair) = (0.03, 0.02);
    let remainder = point - eigenvalue * (score * score - identity);
    let result = finish_repeat_q1_target(
        point,
        score,
        identity,
        remainder,
        1.0e-12,
        eigenvalue,
        &mode,
        &influence,
        &variance,
        rtrace,
        0.01,
        pair,
        1.0e-8,
        Some(RepeatQ1Control {
            control: &control,
            proxy: &proxy,
        }),
    )
    .unwrap();
    let remainder_variance = linear - 2.0 * rtrace + pair;
    assert!(
        (control
            .score_fitted_variance(&mode, &variance, &proxy)
            .unwrap()
            - fitted)
            .abs()
            <= 1.0e-14
    );
    assert!((result.leading_variance - fitted.max(leave_out_score)).abs() <= 1.0e-14);
    assert_eq!(result.leading_variance_correction, identity);
    assert!((result.leading_remainder_covariance - cross).abs() <= 1.0e-14);
    assert!((result.remainder_influence_variance - linear).abs() <= 1.0e-14);
    assert_eq!(result.remainder_trace_variance, 2.0 * rtrace);
    assert!((result.remainder_variance - remainder_variance).abs() <= 1.0e-14);
    assert!((result.remainder_estimate - remainder).abs() <= 1.0e-14);
    // Without the control the same inputs give a different covariance.
    let plain = finish_repeat_q1_target(
        point, score, identity, remainder, 1.0e-12, eigenvalue, &mode, &influence, &variance,
        rtrace, 0.01, pair, 1.0e-8, None,
    )
    .unwrap();
    assert!((plain.leading_remainder_covariance - cross).abs() > 1.0e-3);
}
