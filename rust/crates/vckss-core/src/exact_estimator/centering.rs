// SPDX-License-Identifier: GPL-3.0-only
//! Strong-system correction, shared by all exact targets. No observation square.
use super::*;
use std::sync::Mutex;

pub(super) struct Unit {
    f: Vec<f64>,
    t: Vec<f64>,
    q: f64,
    delta: f64,
    mass: f64,
    multiple: f64,
    diagonal: VarianceComponents,
    local: VarianceComponents,
    source: usize,
}
pub(super) struct State {
    units: Mutex<Vec<Option<Unit>>>,
    expected: usize,
}
impl State {
    pub(super) fn new(count: usize, expected: usize) -> Result<Self> {
        let mut units = Vec::new();
        reserve_exact(&mut units, count, "centering units")?;
        units.resize_with(count, || None);
        Ok(Self {
            units: Mutex::new(units),
            expected,
        })
    }
    fn insert(&self, index: usize, unit: Unit) -> Result<()> {
        let mut units = self
            .units
            .lock()
            .map_err(|_| BackendError::invariant("centering", "unit mutex poisoned"))?;
        if units[index].is_some() {
            return Err(BackendError::invariant("centering", "duplicate unit"));
        }
        units[index] = Some(unit);
        Ok(())
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn observation(
        &self,
        index: usize,
        row: usize,
        source: usize,
        design: &[f64],
        xa: &[f64],
        p: usize,
        frequency: u64,
        z: f64,
        e: f64,
        maker: f64,
        n: u64,
        target: &TargetMoments<'_>,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        let mass = frequency as f64;
        let root = mass.sqrt();
        let delta = n as f64 * maker - 1.0;
        if frequency > 1 && delta <= 1e-12 * (n as f64 * maker).abs().max(1.0) {
            return Err(nonestimable(
                "centering correction has a singular or nonpositive omitted copy-contrast mode",
            ));
        }
        let x = &design[row * p..(row + 1) * p];
        let a = &xa[row * p..(row + 1) * p];
        let f = x.iter().map(|v| root * v).collect();
        let t = x.iter().map(|v| mass * v * z * e / maker).collect();
        let mut diagonal = target.bilinear(
            a,
            a,
            interrupt,
            "centering_observation_target",
            "centering_observation_cells",
        )?;
        diagonal.worker *= mass;
        diagonal.firm *= mass;
        diagonal.covariance *= mass;
        diagonal.total *= mass;
        self.insert(
            index,
            Unit {
                f,
                t,
                q: root * z * e / maker,
                delta,
                mass: 1.0,
                multiple: root,
                diagonal,
                local: VarianceComponents::default(),
                source,
            },
        )
    }
    #[allow(clippy::too_many_arguments)]
    pub(super) fn block(
        &self,
        index: usize,
        indices: &[usize],
        design: &[f64],
        p: usize,
        frequency: &[u64],
        block: &MatchBlockResult,
        n: u64,
        target: &TargetMoments<'_>,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        let width = indices.len();
        let mut f = vec![0.0; p];
        let mut c = vec![0.0; width];
        let mut mass = 0.0;
        let mut sr = 0.0;
        let mut sz = 0.0;
        for (i, &row) in indices.iter().enumerate() {
            checkpoint_chunk(interrupt, i, "centering_block")?;
            c[i] = (frequency[row] as f64).sqrt();
            mass += frequency[row] as f64;
            sr += c[i] * block.deleted_residual[i];
            sz += c[i] * block.transformed_outcome[i];
            for j in 0..p {
                f[j] += frequency[row] as f64 * design[row * p + j];
            }
        }
        let w = transpose_matvec(
            &block.block_inverse,
            width,
            p,
            &c,
            interrupt,
            "centering_direction",
        )?;
        let mut t = vec![0.0; p];
        let mut xi = vec![0.0; width];
        let mut q = 0.0;
        let mut a = 0.0;
        for (i, &row) in indices.iter().enumerate() {
            let ti = block.transformed_outcome[i] * sr;
            xi[i] = (ti - block.deleted_residual[i] * sz) / mass;
            let direction =
                c[i] - c[i] * row_dot(design, row, p, &w, interrupt, "centering_maker_direction")?;
            a += c[i] * direction;
            q += c[i] * ti - n as f64 * direction * xi[i];
            for j in 0..p {
                t[j] += c[i] * design[row * p + j] * ti;
            }
        }
        let h = transpose_matvec(
            &block.block_inverse,
            width,
            p,
            &xi,
            interrupt,
            "centering_contrast",
        )?;
        let local = target.bilinear(
            &w,
            &h,
            interrupt,
            "centering_local_target",
            "centering_local_cells",
        )?;
        let diagonal = target.bilinear(
            &w,
            &w,
            interrupt,
            "centering_block_target",
            "centering_block_cells",
        )?;
        self.insert(
            index,
            Unit {
                f,
                t,
                q,
                delta: n as f64 * a / mass - mass,
                mass,
                multiple: 1.0,
                diagonal,
                local,
                source: 0,
            },
        )
    }
    pub(super) fn finish(
        self,
        information: &[f64],
        inverse: &DenseInverse,
        p: usize,
        firms: core::ops::Range<usize>,
        options: ExactEstimatorOptions,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<[VarianceComponents; 2]> {
        let units = self
            .units
            .into_inner()
            .map_err(|_| BackendError::invariant("centering", "unit mutex poisoned"))?;
        let units: Vec<_> = units.into_iter().flatten().collect();
        let g = units.len();
        if g != self.expected {
            return Err(BackendError::invariant(
                "centering",
                "incomplete unit inventory",
            ));
        }
        let mut score = vec![0.0; p];
        for u in &units {
            for (j, (s, t)) in score.iter_mut().zip(&u.t).enumerate() {
                checkpoint_chunk(interrupt, j, "centering_score")?;
                *s += t;
            }
        }
        let original = matvec(
            &inverse.inverse,
            p,
            &score,
            interrupt,
            "centering_original_score",
        )?;
        let b: Vec<_> = units
            .iter()
            .map(|u| u.q - u.f.iter().zip(&original).map(|(f, v)| f * v).sum::<f64>())
            .collect();
        let mut j = information.to_vec();
        let mut rhs = vec![0.0; p];
        let mut bad = Vec::new();
        for (i, u) in units.iter().enumerate() {
            checkpoint_chunk(interrupt, i, "centering_coefficient_system")?;
            if u.delta > 0.1 * (u.delta + u.mass) {
                for a in 0..p {
                    rhs[a] += u.f[a] * b[i] / u.delta;
                    for c in 0..p {
                        checkpoint_chunk(interrupt, a * p + c, "centering_coefficient_entries")?;
                        j[a * p + c] += u.f[a] * u.f[c] / u.delta;
                    }
                }
            } else {
                bad.push(i);
            }
        }
        if bad.len() > 64 {
            return Err(resource_error(
                "centering has more than 64 exceptional representatives",
            ));
        }
        let factor = crate::dense::invert_scaled_zero_sum_quotient(
            &j,
            p,
            firms,
            options.rank_tolerance,
            interrupt,
            "centering_coefficient_factor",
        )?;
        let mut gamma = matvec(
            &factor.inverse,
            p,
            &rhs,
            interrupt,
            "centering_coefficient_solve",
        )?;
        let mut kappa = vec![0.0; g];
        if !bad.is_empty() {
            let k = bad.len();
            let mut actions = Vec::new();
            for &i in &bad {
                actions.push(matvec(
                    &factor.inverse,
                    p,
                    &units[i].f,
                    interrupt,
                    "centering_exception_action",
                )?);
            }
            let mut e = vec![0.0; k * k];
            let mut r = vec![0.0; k];
            for (a, &i) in bad.iter().enumerate() {
                r[a] = b[i]
                    - units[i]
                        .f
                        .iter()
                        .zip(&gamma)
                        .map(|(f, v)| f * v)
                        .sum::<f64>();
                for (c, action) in actions.iter().enumerate() {
                    e[a * k + c] = units[i]
                        .f
                        .iter()
                        .zip(action)
                        .map(|(f, v)| f * v)
                        .sum::<f64>();
                }
                e[a * k + a] += units[i].delta;
            }
            let ef = crate::dense::invert_scaled_spd(
                &e,
                k,
                options.rank_tolerance,
                interrupt,
                "centering_exception_factor",
            )?;
            let kb = matvec(&ef.inverse, k, &r, interrupt, "centering_exception_solve")?;
            for (a, &i) in bad.iter().enumerate() {
                kappa[i] = kb[a];
                for j in 0..p {
                    gamma[j] += actions[a][j] * kb[a];
                }
            }
        }
        for (i, u) in units.iter().enumerate() {
            if u.delta > 0.1 * (u.delta + u.mass) {
                kappa[i] =
                    (b[i] - u.f.iter().zip(&gamma).map(|(f, v)| f * v).sum::<f64>()) / u.delta;
            }
        }
        let mut score = vec![0.0; p];
        for (u, &k) in units.iter().zip(&kappa) {
            for j in 0..p {
                score[j] += u.f[j] * k;
            }
        }
        let original = matvec(
            &inverse.inverse,
            p,
            &score,
            interrupt,
            "centering_complete_action",
        )?;
        let mut error = 0.0_f64;
        let mut norm = 0.0_f64;
        for (i, u) in units.iter().enumerate() {
            let r = u.delta * kappa[i] + u.f.iter().zip(&original).map(|(f, v)| f * v).sum::<f64>()
                - b[i];
            error = error.hypot(r);
            norm = norm.hypot(b[i]);
        }
        let relative = if norm == 0.0 { error } else { error / norm };
        if !norm.is_finite()
            || !error.is_finite()
            || !relative.is_finite()
            || relative > (10.0 * options.solver_tolerance).max(1e-11)
        {
            return Err(BackendError::new(
                ErrorCode::InverseResidualFailed,
                "centering",
                "complete correction-equation residual failed",
            ));
        }
        let mut out = [VarianceComponents::default(); 2];
        for (u, &k) in units.iter().zip(&kappa) {
            add_scaled(&mut out[u.source], u.local, 1.0);
            add_scaled(&mut out[u.source], u.diagonal, k / (u.mass * u.multiple));
        }
        for v in &mut out {
            v.total = v.worker + v.firm + 2.0 * v.covariance;
            v.verify_accounting(1e-9)?;
        }
        Ok(out)
    }
}
