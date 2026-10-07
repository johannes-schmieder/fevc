// SPDX-License-Identifier: GPL-3.0-only
//! Shared point correction. Auxiliary systems use ordinary weighted model solves.
use crate::error::{BackendError, ErrorCode, Result};
use crate::interrupt::{checkpoint_chunk, InterruptCheck};
use crate::jla::VarianceComponents;
use crate::model_operator::{
    reserve_exact, zeroed_f64_with_interrupt, CanonicalModelData, ModelRhs,
};
use crate::model_solver::{ModelCoefficients, ModelRoutingOptions, PreparedModelSolver};

pub(crate) struct Unit {
    pub rows: Vec<usize>,
    pub weight: Vec<f64>,
    pub mass: f64,
    delta: f64,
    q: f64,
    t: Vec<f64>,
    xi: Vec<f64>,
}
impl Unit {
    pub(crate) fn new(
        rows: Vec<usize>,
        weight: Vec<f64>,
        z: &[f64],
        r: &[f64],
        direction: &[f64],
        n: f64,
    ) -> Result<Self> {
        let mass = weight.iter().sum::<f64>();
        let sr = weight.iter().zip(r).map(|(w, v)| w * v).sum::<f64>();
        let sz = weight.iter().zip(z).map(|(w, v)| w * v).sum::<f64>();
        let t: Vec<_> = z.iter().map(|v| v * sr).collect();
        let xi: Vec<_> = t.iter().zip(r).map(|(t, r)| (t - r * sz) / mass).collect();
        let a = weight
            .iter()
            .zip(direction)
            .map(|(w, v)| w * v)
            .sum::<f64>()
            / mass;
        let q = weight.iter().zip(&t).map(|(w, v)| w * v).sum::<f64>()
            - n * weight
                .iter()
                .zip(direction)
                .zip(&xi)
                .map(|((w, d), x)| w * d * x)
                .sum::<f64>();
        if !mass.is_finite() || mass <= 0.0 || !q.is_finite() {
            return Err(failure("nonfinite unit geometry"));
        }
        Ok(Self {
            rows,
            weight,
            mass,
            delta: n * a - mass,
            q,
            t,
            xi,
        })
    }
}
fn failure(message: &str) -> BackendError {
    BackendError::new(ErrorCode::JlaConstraintFailed, "centering", message)
}
fn zeros(n: usize, interrupt: &mut dyn InterruptCheck) -> Result<Vec<f64>> {
    zeroed_f64_with_interrupt(
        n,
        "centering point workspace",
        interrupt,
        "centering_allocate",
    )
}
fn predict(data: CanonicalModelData<'_>, coef: &ModelCoefficients, row: usize) -> f64 {
    coef.worker[data.row_worker[row] as usize]
        + coef.firm[data.row_firm[row] as usize]
        + data
            .controls
            .iter()
            .zip(&coef.control)
            .map(|(x, b)| x[row] * b)
            .sum::<f64>()
}
fn score_solve(
    solver: &PreparedModelSolver<'_>,
    data: CanonicalModelData<'_>,
    score: &[f64],
    interrupt: &mut dyn InterruptCheck,
) -> Result<ModelCoefficients> {
    let mut w = vec![crate::numerical_mc::Sum::default(); data.workers];
    let mut f = vec![crate::numerical_mc::Sum::default(); data.firms];
    let mut c = vec![crate::numerical_mc::Sum::default(); data.controls.len()];
    let mut energy = 0.0;
    let mut control_energy = vec![0.0; data.controls.len()];
    for (row, &value) in score.iter().enumerate() {
        checkpoint_chunk(interrupt, row, "centering_rhs")?;
        w[data.row_worker[row] as usize].add(value);
        f[data.row_firm[row] as usize].add(value);
        energy += value.abs();
        for (j, x) in data.controls.iter().enumerate() {
            let v = x[row] * value;
            c[j].add(v);
            control_energy[j] += v.abs();
        }
    }
    let mut w: Vec<_> = w.into_iter().map(|v| v.get()).collect();
    let mut f: Vec<_> = f.into_iter().map(|v| v.get()).collect();
    let mut c: Vec<_> = c.into_iter().map(|v| v.get()).collect();
    // An annihilated coefficient score can retain cancellation roundoff. Only
    // zero a whole RHS within its own accumulation envelope; the complete K
    // equation is independently checked after recovery.
    let tiny = energy.is_finite()
        && w.iter()
            .chain(&f)
            .all(|v| v.abs() <= 128.0 * f64::EPSILON * energy)
        && c.iter()
            .zip(&control_energy)
            .all(|(v, e)| e.is_finite() && v.abs() <= 128.0 * f64::EPSILON * e);
    if tiny {
        w.fill(0.0);
        f.fill(0.0);
        c.fill(0.0);
    }
    Ok(solver
        .solve_with_interrupt(
            ModelRhs {
                worker: &w,
                firm: &f,
                control: &c,
            },
            interrupt,
        )?
        .coefficients)
}
fn unit_dot(data: CanonicalModelData<'_>, unit: &Unit, coef: &ModelCoefficients) -> f64 {
    unit.rows
        .iter()
        .zip(&unit.weight)
        .map(|(&row, &w)| w * predict(data, coef, row))
        .sum()
}
fn unit_score(score: &mut [f64], unit: &Unit, factor: f64) {
    for (&row, &w) in unit.rows.iter().zip(&unit.weight) {
        score[row] += w * factor;
    }
}

pub(crate) struct Map {
    pub units: Vec<Unit>,
    pub chat: Vec<Vec<f64>>,
}
impl Map {
    pub(crate) fn solve(
        data: CanonicalModelData<'_>,
        solver: &PreparedModelSolver<'_>,
        units: Vec<Unit>,
        mut routing: ModelRoutingOptions,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        // Route is selected before probes by the caller. Never fall back after RNG.
        routing.allow_automatic_cmg_setup_fallback = false;
        let mut score = zeros(data.rows(), interrupt)?;
        for (g, u) in units.iter().enumerate() {
            checkpoint_chunk(interrupt, g, "centering_unit_score")?;
            for ((&row, &w), &t) in u.rows.iter().zip(&u.weight).zip(&u.t) {
                score[row] += w * t;
            }
        }
        let original = score_solve(solver, data, &score, interrupt).map_err(|mut e| {
            e.message = format!("original centering score: {}", e.message);
            e
        })?;
        let b: Vec<_> = units
            .iter()
            .map(|u| u.q - unit_dot(data, u, &original))
            .collect();
        let bad: Vec<_> = units
            .iter()
            .enumerate()
            .filter(|(_, u)| u.delta <= 0.1 * (u.delta + u.mass))
            .map(|(g, _)| g)
            .collect();
        if bad.len() > 64 {
            return Err(BackendError::new(
                ErrorCode::ResourceLimit,
                "centering",
                "more than 64 exceptional units",
            ));
        }
        let mut rw = data.row_worker.to_vec();
        let mut rf = data.row_firm.to_vec();
        let mut weight = data.weight.to_vec();
        let mut controls = data.controls.to_vec();
        for (g, u) in units.iter().enumerate() {
            checkpoint_chunk(interrupt, g, "centering_auxiliary_design")?;
            if u.delta > 0.1 * (u.delta + u.mass) {
                let row = u.rows[0];
                rw.push(data.row_worker[row]);
                rf.push(data.row_firm[row]);
                weight.push(u.mass * u.mass / u.delta);
                for (j, c) in controls.iter_mut().enumerate() {
                    c.push(
                        u.rows
                            .iter()
                            .zip(&u.weight)
                            .map(|(&row, &w)| w * data.controls[j][row])
                            .sum::<f64>()
                            / u.mass,
                    );
                }
            }
        }
        let auxiliary = PreparedModelSolver::prepare_routed_with_interrupt(
            CanonicalModelData {
                workers: data.workers,
                firms: data.firms,
                row_worker: &rw,
                row_firm: &rf,
                weight: &weight,
                controls: &controls,
            },
            routing,
            interrupt,
        )?;
        score.fill(0.0);
        for (g, u) in units.iter().enumerate() {
            if u.delta > 0.1 * (u.delta + u.mass) {
                unit_score(&mut score, u, b[g] / u.delta);
            }
        }
        let mut gamma = score_solve(&auxiliary, data, &score, interrupt).map_err(|mut e| {
            e.message = format!("auxiliary centering score: {}", e.message);
            e
        })?;
        let mut kappa = zeros(units.len(), interrupt)?;
        if !bad.is_empty() {
            let k = bad.len();
            let mut actions = Vec::new();
            reserve_exact(&mut actions, k, "centering exception actions")?;
            for &g in &bad {
                score.fill(0.0);
                unit_score(&mut score, &units[g], 1.0);
                actions.push(score_solve(&auxiliary, data, &score, interrupt)?);
            }
            let mut schur = zeros(k * k, interrupt)?;
            let mut rhs = zeros(k, interrupt)?;
            for (i, &g) in bad.iter().enumerate() {
                rhs[i] = b[g] - unit_dot(data, &units[g], &gamma);
                for (j, a) in actions.iter().enumerate() {
                    schur[i * k + j] = unit_dot(data, &units[g], a);
                }
                schur[i * k + i] += units[g].delta;
            }
            let inverse = crate::dense::invert_scaled_spd(
                &schur,
                k,
                routing.solver.rank_tolerance,
                interrupt,
                "centering_exception",
            )?;
            for (i, &g) in bad.iter().enumerate() {
                kappa[g] = (0..k).map(|j| inverse.inverse[i * k + j] * rhs[j]).sum();
                for (v, a) in gamma.worker.iter_mut().zip(&actions[i].worker) {
                    *v += a * kappa[g];
                }
                for (v, a) in gamma.firm.iter_mut().zip(&actions[i].firm) {
                    *v += a * kappa[g];
                }
                for (v, a) in gamma.control.iter_mut().zip(&actions[i].control) {
                    *v += a * kappa[g];
                }
            }
        }
        for (g, u) in units.iter().enumerate() {
            checkpoint_chunk(interrupt, g, "centering_recovery")?;
            if u.delta > 0.1 * (u.delta + u.mass) {
                kappa[g] = (b[g] - unit_dot(data, u, &gamma)) / u.delta;
            }
        }
        score.fill(0.0);
        for (u, &k) in units.iter().zip(&kappa) {
            unit_score(&mut score, u, k);
        }
        let complete = score_solve(solver, data, &score, interrupt).map_err(|mut e| {
            e.message = format!("complete centering score: {}", e.message);
            e
        })?;
        let scale = b.iter().fold(0.0_f64, |s, v| s.max(v.abs()));
        let mut error = 0.0_f64;
        let mut norm = 0.0_f64;
        for (g, u) in units.iter().enumerate() {
            checkpoint_chunk(interrupt, g, "centering_complete_residual")?;
            let r = u.delta * kappa[g] + unit_dot(data, u, &complete) - b[g];
            let s = scale.max(1e-300);
            error = error.hypot(r / s);
            norm = norm.hypot(b[g] / s);
        }
        let residual = if norm == 0.0 { error } else { error / norm };
        if !residual.is_finite() || residual > routing.solver.full_residual_tolerance() {
            return Err(BackendError::new(
                ErrorCode::InverseResidualFailed,
                "centering",
                "complete correction equation failed",
            ));
        }
        let chat = units
            .iter()
            .zip(&kappa)
            .map(|(u, k)| u.xi.iter().map(|x| x + k / u.mass).collect())
            .collect();
        Ok(Self { units, chat })
    }
    pub(crate) fn jackknife(mut full: Self, first: Self, second: Self) -> Result<Self> {
        if full.units.len() != first.units.len() || full.units.len() != second.units.len() {
            return Err(failure("pool unit inventory disagrees"));
        }
        for ((f, a), b) in full.chat.iter_mut().zip(first.chat).zip(second.chat) {
            for ((f, a), b) in f.iter_mut().zip(a).zip(b) {
                *f = 2.0 * *f - 0.5 * (a + b);
                if !f.is_finite() {
                    return Err(failure("nonfinite jackknife map"));
                }
            }
        }
        Ok(full)
    }
    pub(crate) fn contract(
        &self,
        worker: &[f64],
        firm: &[f64],
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<VarianceComponents> {
        let mut out = VarianceComponents::default();
        for (g, (u, chat)) in self.units.iter().zip(&self.chat).enumerate() {
            checkpoint_chunk(interrupt, g, "centering_target")?;
            let mut sw = 0.0;
            let mut sf = 0.0;
            let mut cw = 0.0;
            let mut cf = 0.0;
            for ((&row, &mass), &c) in u.rows.iter().zip(&u.weight).zip(chat) {
                sw += mass * worker[row];
                sf += mass * firm[row];
                cw += mass * c * worker[row];
                cf += mass * c * firm[row];
            }
            out.worker += sw * cw;
            out.firm += sf * cf;
            out.covariance += 0.5 * (sw * cf + sf * cw);
        }
        out.total = out.worker + out.firm + 2.0 * out.covariance;
        if !out.worker.is_finite() || !out.firm.is_finite() || !out.covariance.is_finite() {
            return Err(failure("nonfinite target increment"));
        }
        Ok(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::dense::invert_scaled_spd;
    use crate::interrupt::NeverInterrupt;
    use crate::model_solver::ModelSolverRoute;
    fn dot(a: &[f64], b: &[f64]) -> f64 {
        a.iter().zip(b).map(|(a, b)| a * b).sum()
    }
    #[test]
    fn three_pool_map_matches_literal_dense_system_with_controls_and_copies() {
        for deletion in 0..3 {
            for joint in [false, true] {
                let mut check = NeverInterrupt;
                let workers = 4;
                let firms = 3;
                let rows = 24;
                let rw: Vec<_> = (0..rows).map(|i| (i / 6) as u32).collect();
                let rf: Vec<_> = (0..rows).map(|i| ((i / 2) % 3) as u32).collect();
                let frequency: Vec<_> = (0..rows).map(|i| 1.0 + (i % 2) as f64).collect();
                let controls = if joint {
                    vec![(0..rows).map(|i| (i as f64 * 0.71).sin()).collect()]
                } else {
                    Vec::new()
                };
                let data = CanonicalModelData {
                    workers,
                    firms,
                    row_worker: &rw,
                    row_firm: &rf,
                    weight: &frequency,
                    controls: &controls,
                };
                let routing = ModelRoutingOptions {
                    route: ModelSolverRoute::Diagonal,
                    allow_automatic_cmg_setup_fallback: false,
                    ..ModelRoutingOptions::default()
                };
                let solver =
                    PreparedModelSolver::prepare_routed_with_interrupt(data, routing, &mut check)
                        .unwrap();
                let fe = workers + firms - 1;
                let p = fe + controls.len();
                let mut x = vec![vec![0.0; p]; rows];
                for row in 0..rows {
                    x[row][rw[row] as usize] = 1.0;
                    if (rf[row] as usize) < firms - 1 {
                        x[row][workers + rf[row] as usize] = 1.0;
                    }
                    if joint {
                        x[row][p - 1] = controls[0][row];
                    }
                }
                let mut s = vec![0.0; p * p];
                let mut sfe = vec![0.0; fe * fe];
                for row in 0..rows {
                    for i in 0..p {
                        for j in 0..p {
                            s[i * p + j] += frequency[row] * x[row][i] * x[row][j];
                            if i < fe && j < fe {
                                sfe[i * fe + j] += frequency[row] * x[row][i] * x[row][j];
                            }
                        }
                    }
                }
                let a = invert_scaled_spd(&s, p, 1e-12, &mut check, "test")
                    .unwrap()
                    .inverse;
                let af = invert_scaled_spd(&sfe, fe, 1e-12, &mut check, "test")
                    .unwrap()
                    .inverse;
                let mut expanded = Vec::new();
                for (row, &f) in frequency.iter().enumerate() {
                    for _ in 0..f as usize {
                        expanded.push(row);
                    }
                }
                let n = expanded.len();
                let mut h = vec![0.0; n * n];
                let mut hf = vec![0.0; n * n];
                for i in 0..n {
                    for j in 0..n {
                        for u in 0..p {
                            for v in 0..p {
                                h[i * n + j] +=
                                    x[expanded[i]][u] * a[u * p + v] * x[expanded[j]][v];
                                if u < fe && v < fe {
                                    hf[i * n + j] +=
                                        x[expanded[i]][u] * af[u * fe + v] * x[expanded[j]][v];
                                }
                            }
                        }
                    }
                }
                let y: Vec<_> = (0..rows)
                    .map(|i| 3.0 + (i as f64 * 0.43).sin() + rw[i] as f64 * 0.2)
                    .collect();
                let mean = dot(&frequency, &y) / n as f64;
                let z: Vec<_> = expanded.iter().map(|&row| y[row] - mean).collect();
                let e: Vec<_> = (0..n)
                    .map(|i| {
                        y[expanded[i]] - (0..n).map(|j| h[i * n + j] * y[expanded[j]]).sum::<f64>()
                    })
                    .collect();
                let mut groups: Vec<Vec<usize>> = Vec::new();
                for i in 0..n {
                    let row = expanded[i];
                    if deletion == 1 || (deletion == 2 && rw[row] == 0) {
                        groups.push(vec![i]);
                    } else if let Some(g) = groups
                        .iter_mut()
                        .find(|g| rw[expanded[g[0]]] == rw[row] && rf[expanded[g[0]]] == rf[row])
                    {
                        g.push(i);
                    } else {
                        groups.push(vec![i]);
                    }
                }
                let g = groups.len();
                let wp: Vec<_> = (0..rows)
                    .map(|i| {
                        0.3 * rw[i] as f64
                            + 0.1 * rf[i] as f64
                            + if joint { 0.2 * controls[0][i] } else { 0.0 }
                    })
                    .collect();
                let fp: Vec<_> = (0..rows)
                    .map(|i| {
                        0.2 * rw[i] as f64 - 0.4 * rf[i] as f64
                            + if joint { 0.1 * controls[0][i] } else { 0.0 }
                    })
                    .collect();
                let mut maps = Vec::new();
                let mut expected_pools = Vec::new();
                for factor in [1.0, 1.025, 0.98] {
                    let mut units = Vec::new();
                    let mut xi = vec![0.0; n];
                    let mut t = vec![0.0; n];
                    let mut aa = vec![0.0; g];
                    let mut q = vec![0.0; g];
                    for (unit, indices) in groups.iter().enumerate() {
                        let m = indices.len();
                        let pf = indices
                            .iter()
                            .flat_map(|&i| indices.iter().map(move |&j| (i, j)))
                            .map(|(i, j)| hf[i * n + j])
                            .sum::<f64>()
                            / m as f64
                            * factor;
                        let mut d = vec![0.0; m * m];
                        for (i, &ii) in indices.iter().enumerate() {
                            for (j, &jj) in indices.iter().enumerate() {
                                d[i * m + j] = f64::from(i == j)
                                    - pf / m as f64
                                    - (h[ii * n + jj] - hf[ii * n + jj]);
                            }
                        }
                        let di = invert_scaled_spd(&d, m, 1e-12, &mut check, "test_maker")
                            .unwrap()
                            .inverse;
                        let zz: Vec<_> = indices.iter().map(|&i| z[i]).collect();
                        let rr: Vec<_> = (0..m)
                            .map(|i| {
                                indices
                                    .iter()
                                    .enumerate()
                                    .map(|(j, &ii)| di[i * m + j] * e[ii])
                                    .sum()
                            })
                            .collect();
                        let dir: Vec<_> =
                            (0..m).map(|i| d[i * m..(i + 1) * m].iter().sum()).collect();
                        let sr = rr.iter().sum::<f64>();
                        let sz = zz.iter().sum::<f64>();
                        for (i, &ii) in indices.iter().enumerate() {
                            t[ii] = zz[i] * sr;
                            xi[ii] = (t[ii] - rr[i] * sz) / m as f64;
                        }
                        aa[unit] = dir.iter().sum::<f64>() / m as f64;
                        q[unit] = indices.iter().map(|&i| t[i]).sum::<f64>()
                            - n as f64
                                * indices
                                    .iter()
                                    .enumerate()
                                    .map(|(i, &ii)| dir[i] * xi[ii])
                                    .sum::<f64>();
                        units.push(
                            Unit::new(
                                indices.iter().map(|&i| expanded[i]).collect(),
                                vec![1.0; m],
                                &zz,
                                &rr,
                                &dir,
                                n as f64,
                            )
                            .unwrap(),
                        );
                    }
                    let b: Vec<_> = groups
                        .iter()
                        .enumerate()
                        .map(|(u, indices)| {
                            q[u] - indices
                                .iter()
                                .map(|&i| (0..n).map(|j| h[i * n + j] * t[j]).sum::<f64>())
                                .sum::<f64>()
                        })
                        .collect();
                    let mut k = vec![0.0; g * g];
                    for i in 0..g {
                        for j in 0..g {
                            k[i * g + j] = if i == j {
                                n as f64 * aa[i] - groups[i].len() as f64
                            } else {
                                0.0
                            };
                            for &ii in &groups[i] {
                                for &jj in &groups[j] {
                                    k[i * g + j] += h[ii * n + jj];
                                }
                            }
                        }
                    }
                    let ki = invert_scaled_spd(&k, g, 1e-12, &mut check, "test_literal_system")
                        .unwrap()
                        .inverse;
                    let kk: Vec<_> = (0..g).map(|i| dot(&ki[i * g..(i + 1) * g], &b)).collect();
                    let mut chat = xi;
                    for (i, indices) in groups.iter().enumerate() {
                        for &ii in indices {
                            chat[ii] += kk[i] / indices.len() as f64;
                        }
                    }
                    let mut expected = VarianceComponents::default();
                    for indices in &groups {
                        let sw = indices.iter().map(|&i| wp[expanded[i]]).sum::<f64>();
                        let sf = indices.iter().map(|&i| fp[expanded[i]]).sum::<f64>();
                        let cw = indices
                            .iter()
                            .map(|&i| chat[i] * wp[expanded[i]])
                            .sum::<f64>();
                        let cf = indices
                            .iter()
                            .map(|&i| chat[i] * fp[expanded[i]])
                            .sum::<f64>();
                        expected.worker += sw * cw;
                        expected.firm += sf * cf;
                        expected.covariance += (sw * cf + sf * cw) / 2.0;
                    }
                    expected_pools.push(expected);
                    maps.push(
                        Map::solve(data, &solver, units, routing, &mut check).unwrap_or_else(|e| {
                            panic!("deletion={deletion} joint={joint} factor={factor}: {e:?}")
                        }),
                    );
                }
                let second = maps.pop().unwrap();
                let first = maps.pop().unwrap();
                let result = Map::jackknife(maps.pop().unwrap(), first, second)
                    .unwrap()
                    .contract(&wp, &fp, &mut check)
                    .unwrap();
                for (actual, expected) in [
                    (
                        result.worker,
                        2.0 * expected_pools[0].worker
                            - (expected_pools[1].worker + expected_pools[2].worker) / 2.0,
                    ),
                    (
                        result.firm,
                        2.0 * expected_pools[0].firm
                            - (expected_pools[1].firm + expected_pools[2].firm) / 2.0,
                    ),
                    (
                        result.covariance,
                        2.0 * expected_pools[0].covariance
                            - (expected_pools[1].covariance + expected_pools[2].covariance) / 2.0,
                    ),
                ] {
                    assert!(
                        (actual - expected).abs() < 1e-8 * (1.0 + expected.abs()),
                        "{deletion} {joint}: {actual} != {expected}"
                    );
                }
            }
        }
    }
}
