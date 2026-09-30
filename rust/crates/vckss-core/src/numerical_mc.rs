// SPDX-License-Identifier: GPL-3.0-only

//! Local point-probe linearization. Primitive order is worker, firm, covariance.
//! This module changes neither point adjustments nor statistical random draws.

use crate::error::{BackendError, Result};

pub type Matrix3 = [[f64; 3]; 3];

#[derive(Clone, Copy, Debug, Default)]
pub struct Primitive {
    pub worker: f64,
    pub firm: f64,
    pub covariance: f64,
}

impl Primitive {
    pub fn values(self) -> [f64; 3] {
        [self.worker, self.firm, self.covariance]
    }
    pub fn from_values(v: [f64; 3]) -> Self {
        Self {
            worker: v[0],
            firm: v[1],
            covariance: v[2],
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Status {
    ExactZero,
    OkLocal,
    OkLocalPsdAdjusted,
    UnstableNonPsd,
    NonsmoothAdjustment,
    NonfiniteDerivative,
    ReplayFailed,
}

#[derive(Clone, Debug)]
pub struct Covariance {
    pub conditional: Matrix3,
    pub leverage: Matrix3,
    pub raw: Matrix3,
    pub usable: Option<Matrix3>,
    pub mcse: Option<[f64; 4]>,
    pub status: Status,
    pub psd_adjustment: f64,
}

/// Raw means, with R fixed. Gradients never differentiate a gate or clipping.
#[derive(Clone, Copy, Debug)]
pub struct FiniteDerivative {
    pub h: f64,
    pub m: f64,
    pub bias: f64,
    pub variance: f64,
    pub dh: [f64; 5],
    pub db: [f64; 5],
    pub dv: [f64; 5],
    pub nonsmooth: bool,
}

impl FiniteDerivative {
    pub fn new(u: [f64; 5], r: u32) -> Option<Self> {
        let [p, m, a, b, c] = u;
        let s = p + m;
        if r < 2 || s <= 0.0 || !u.iter().all(|x| x.is_finite()) {
            return None;
        }
        let h = p / s;
        let m = m / s;
        let r = f64::from(r);
        let bias = (m * a - h * b + (m - h) * c) / r;
        let variance = (m * m * a + h * h * b - 2.0 * h * m * c) / r;
        let dh = [m / s, -h / s, 0.0, 0.0, 0.0];
        let db = std::array::from_fn(|j| {
            let da = f64::from(j == 2);
            let db = f64::from(j == 3);
            let dc = f64::from(j == 4);
            (-(a + b + 2.0 * c) * dh[j] + m * da - h * db + (m - h) * dc) / r
        });
        let dv = std::array::from_fn(|j| {
            let da = f64::from(j == 2);
            let db = f64::from(j == 3);
            let dc = f64::from(j == 4);
            ((-2.0 * m * a + 2.0 * h * b - 2.0 * c * (m - h)) * dh[j] + m * m * da + h * h * db
                - 2.0 * h * m * dc)
                / r
        });
        let result = Self {
            h,
            m,
            bias,
            variance,
            dh,
            db,
            dv,
            nonsmooth: variance < 0.0,
        };
        [h, m, bias, variance]
            .into_iter()
            .chain(dh)
            .chain(db)
            .chain(dv)
            .all(f64::is_finite)
            .then_some(result)
    }

    pub fn observation(self, control: f64) -> Option<[f64; 5]> {
        let ell = self.m - control;
        if ell <= 0.0 || self.nonsmooth {
            return None;
        }
        let inv = ell.recip();
        let factor =
            -inv.powi(2) - 2.0 * self.bias * inv.powi(3) + 3.0 * self.variance * inv.powi(4);
        let gradient = std::array::from_fn(|j| {
            -factor * self.dh[j] + inv.powi(2) * self.db[j] - inv.powi(3) * self.dv[j]
        });
        gradient.iter().all(|x| x.is_finite()).then_some(gradient)
    }

    pub fn block(self, k: f64) -> Option<[f64; 5]> {
        if self.nonsmooth || !k.is_finite() || k <= 0.0 {
            return None;
        }
        let gradient = std::array::from_fn(|j| {
            (1.0 + 2.0 * self.bias * k - 3.0 * self.variance * k * k) * self.dh[j] + self.db[j]
                - k * self.dv[j]
        });
        gradient.iter().all(|x| x.is_finite()).then_some(gradient)
    }
}

pub fn copy_pullback(lambda: [f64; 5]) -> [f64; 4] {
    let [p, m, a, b, c] = lambda;
    [
        p + m + 6.0 * b + c,
        a + b + c,
        -2.0 * m - 4.0 * b,
        -4.0 * b - 2.0 * c,
    ]
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct Sum {
    value: f64,
    error: f64,
}
impl Sum {
    pub(crate) fn add(&mut self, v: f64) {
        let next = self.value + v;
        self.error += if self.value.abs() >= v.abs() {
            (self.value - next) + v
        } else {
            (v - next) + self.value
        };
        self.value = next;
    }
    pub(crate) fn get(self) -> f64 {
        self.value + self.error
    }
}

fn means(x: &[Primitive]) -> [f64; 3] {
    let mut sums = [Sum::default(); 3];
    for v in x {
        for (s, v) in sums.iter_mut().zip(v.values()) {
            s.add(v / x.len() as f64);
        }
    }
    sums.map(Sum::get)
}

/// Scaled centered cross product; inputs are whole-direction scores, not units.
pub fn cross_covariance(a: &[Primitive], b: &[Primitive]) -> Result<Matrix3> {
    if a.len() < 2 || a.len() != b.len() {
        return Err(BackendError::invalid(
            "numerical_mc",
            "invalid score counts",
        ));
    }
    let ma = means(a);
    let mb = means(b);
    let scale = a
        .iter()
        .chain(b)
        .flat_map(|v| v.values())
        .fold(0.0_f64, |s, v| s.max(v.abs()));
    if !a
        .iter()
        .chain(b)
        .flat_map(|v| v.values())
        .all(f64::is_finite)
    {
        return Err(BackendError::invalid("numerical_mc", "nonfinite scores"));
    }
    if scale == 0.0 {
        return Ok([[0.0; 3]; 3]);
    }
    let mut cov = [[Sum::default(); 3]; 3];
    let divisor = (a.len() as f64) * ((a.len() - 1) as f64);
    for (a, b) in a.iter().zip(b) {
        let a = a.values();
        let b = b.values();
        for i in 0..3 {
            for j in i..3 {
                let ai = a[i] / scale - ma[i] / scale;
                let aj = a[j] / scale - ma[j] / scale;
                let bi = b[i] / scale - mb[i] / scale;
                let bj = b[j] / scale - mb[j] / scale;
                cov[i][j].add((ai * bj + bi * aj) / (2.0 * divisor));
            }
        }
    }
    let mut out = [[0.0; 3]; 3];
    for i in 0..3 {
        for j in i..3 {
            out[i][j] = (cov[i][j].get() * scale) * scale;
            out[j][i] = out[i][j];
        }
    }
    Ok(out)
}

pub fn conditional_covariance(draws: &[Primitive]) -> Result<Matrix3> {
    cross_covariance(draws, draws)
}

pub fn finalize(conditional: Matrix3, leverage: Matrix3) -> Covariance {
    let raw = std::array::from_fn(|i| std::array::from_fn(|j| conditional[i][j] + leverage[i][j]));
    let mut result = Covariance {
        conditional,
        leverage,
        raw,
        usable: None,
        mcse: None,
        status: Status::NonfiniteDerivative,
        psd_adjustment: 0.0,
    };
    if !conditional
        .iter()
        .chain(leverage.iter())
        .flatten()
        .chain(raw.iter().flatten())
        .all(|v| v.is_finite())
    {
        return result;
    }
    let scale = conditional
        .iter()
        .chain(leverage.iter())
        .flatten()
        .fold(0.0_f64, |s, v| s.max(v.abs()));
    if scale == 0.0 {
        result.status = Status::OkLocal;
        result.usable = Some(raw);
        result.mcse = Some([0.0; 4]);
        return result;
    }
    let norm = |m: Matrix3| {
        m.iter()
            .flatten()
            .map(|v| (v / scale).powi(2))
            .sum::<f64>()
            .sqrt()
    };
    let tau = 1e-12 * (norm(conditional) + norm(leverage));
    let mut a = std::array::from_fn::<_, 3, _>(|i| {
        std::array::from_fn::<_, 3, _>(|j| (raw[i][j] / scale + raw[j][i] / scale) * 0.5)
    });
    let mut vectors = [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]];
    // Fixed dimension, scaled Jacobi rotations. A signed tie must rotate too.
    for _ in 0..32 {
        let (p, q) = [(0, 1), (0, 2), (1, 2)]
            .into_iter()
            .max_by(|&(i, j), &(k, l)| a[i][j].abs().total_cmp(&a[k][l].abs()))
            .unwrap();
        if a[p][q].abs() <= 1e-16 {
            break;
        }
        let delta = (a[q][q] - a[p][p]) * 0.5;
        let t = if delta == 0.0 {
            a[p][q].signum()
        } else {
            a[p][q] / (delta + delta.signum() * delta.hypot(a[p][q]))
        };
        let c = 1.0 / (1.0 + t * t).sqrt();
        let s = t * c;
        let off = a[p][q];
        a[p][p] -= t * off;
        a[q][q] += t * off;
        a[p][q] = 0.0;
        a[q][p] = 0.0;
        for k in 0..3 {
            if k != p && k != q {
                let ap = a[k][p];
                let aq = a[k][q];
                a[k][p] = c * ap - s * aq;
                a[p][k] = a[k][p];
                a[k][q] = s * ap + c * aq;
                a[q][k] = a[k][q];
            }
            let vp = vectors[k][p];
            let vq = vectors[k][q];
            vectors[k][p] = c * vp - s * vq;
            vectors[k][q] = s * vp + c * vq;
        }
    }
    if (0..3).any(|i| a[i][i] < -tau) {
        result.status = Status::UnstableNonPsd;
        return result;
    }
    let adjusted = (0..3).any(|i| a[i][i] < 0.0);
    let usable = if adjusted {
        std::array::from_fn(|i| {
            std::array::from_fn(|j| {
                (0..3)
                    .map(|k| vectors[i][k] * a[k][k].max(0.0) * vectors[j][k])
                    .sum::<f64>()
                    * scale
            })
        })
    } else {
        std::array::from_fn(|i| std::array::from_fn(|j| (raw[i][j] * 0.5) + (raw[j][i] * 0.5)))
    };
    result.psd_adjustment = usable
        .iter()
        .flatten()
        .zip(raw.iter().flatten())
        .map(|(u, r)| ((u - r) / scale).powi(2))
        .sum::<f64>()
        .sqrt()
        * scale;
    let contrast = [1.0, 1.0, 2.0];
    let total = (0..3)
        .flat_map(|i| (0..3).map(move |j| contrast[i] * (usable[i][j] / scale) * contrast[j]))
        .sum::<f64>();
    result.mcse = Some([
        usable[0][0].max(0.0).sqrt(),
        usable[1][1].max(0.0).sqrt(),
        usable[2][2].max(0.0).sqrt(),
        total.max(0.0).sqrt() * scale.sqrt(),
    ]);
    result.usable = Some(usable);
    result.status = if adjusted {
        Status::OkLocalPsdAdjusted
    } else {
        Status::OkLocal
    };
    result
}
