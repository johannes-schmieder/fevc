// SPDX-License-Identifier: GPL-3.0-only
//! Full response storage is compiled only into core unit tests.
use super::*;

thread_local! {
    static ENABLED: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
}

pub(super) struct OracleGuard(bool);
impl Drop for OracleGuard {
    fn drop(&mut self) {
        ENABLED.with(|enabled| enabled.set(self.0));
    }
}

// Only the explicit Python oracle test captures full responses. Ordinary Rust
// regressions must remain independent of a repository venv, including on CI.
pub(super) fn enable_oracle() -> OracleGuard {
    OracleGuard(ENABLED.with(|enabled| enabled.replace(true)))
}

pub(in crate::generic_jla) struct Cache {
    enabled: bool,
    units: std::collections::BTreeMap<(u8, usize), Unit>,
}
impl Default for Cache {
    fn default() -> Self {
        Self {
            enabled: ENABLED.with(std::cell::Cell::get),
            units: std::collections::BTreeMap::new(),
        }
    }
}
#[derive(Default)]
struct Unit {
    u: Vec<[f64; 5]>,
    l: Vec<Vec<[f64; 3]>>,
    cp: Vec<f64>,
    v: Vec<f64>,
    e: Vec<f64>,
}
impl Cache {
    pub(in crate::generic_jla) fn leverage(
        &mut self,
        problem: &CompressedProblem,
        classes: &[ObservationClass],
        plan: Option<&MatchPlan>,
        solver: &PreparedModelSolver<'_>,
        solution: &ModelSolve,
        atoms: &[i64],
        rng: CounterRng,
        probe: usize,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        let c = &solution.coefficients;
        let predict = |row| {
            solver
                .operator()
                .prediction_at(&c.worker, &c.firm, &c.control, row)
        };
        let mut offsets = vec![0; problem.outcome.len() + 1];
        for row in 0..problem.outcome.len() {
            offsets[row + 1] = offsets[row] + problem.frequency[row] as usize;
        }
        for class in classes {
            let mut offset = 0;
            for &row in &class.rows {
                let p = predict(row)?;
                for copy in 0..problem.frequency[row] {
                    let q = f64::from(rademacher_copy(
                        rng,
                        ProbeDomain::Leverage,
                        probe as u64,
                        class.entity,
                        offset + copy,
                    ));
                    let m = q - p;
                    self.units
                        .entry((0, offsets[row] + copy as usize))
                        .or_default()
                        .u
                        .push([p * p, m * m, p.powi(4), m.powi(4), p * p * m * m]);
                }
                offset += problem.frequency[row];
            }
        }
        if let Some(plan) = plan {
            for group in 0..plan.rows.len() {
                let root = (plan.physical_count[group] as f64).sqrt();
                let p = root * predict(plan.rows[group][0])?;
                let m = atoms[group] as f64 / root - p;
                self.units.entry((1, group)).or_default().u.push([
                    p * p,
                    m * m,
                    p.powi(4),
                    m.powi(4),
                    p * p * m * m,
                ]);
            }
        }
        Ok(())
    }

    pub(in crate::generic_jla) fn observation_geometry(
        &mut self,
        correlations: &ObservationCorrelations,
        control: &[f64],
    ) {
        if !self.enabled {
            return;
        }
        for ((kind, physical), unit) in &mut self.units {
            if *kind != 0 {
                continue;
            }
            let row = correlations
                .row_offset
                .partition_point(|&offset| offset <= *physical)
                - 1;
            unit.cp = vec![control[row]];
        }
    }

    pub(in crate::generic_jla) fn block_geometry(
        &mut self,
        group: usize,
        low_rank: &[f64],
        width: usize,
        rank: usize,
        rhs: &[f64],
    ) {
        if !self.enabled {
            return;
        }
        let unit = self.units.get_mut(&(1, group)).unwrap();
        unit.cp = (0..width)
            .flat_map(|i| {
                (0..width).map(move |j| {
                    (1..rank)
                        .map(|k| low_rank[i * rank + k] * low_rank[j * rank + k])
                        .sum()
                })
            })
            .collect();
        unit.e = rhs[..width].to_vec();
        unit.v = rhs[width..].to_vec();
    }

    pub(in crate::generic_jla) fn target(
        &mut self,
        problem: &CompressedProblem,
        classes: &[ObservationClass],
        solver: &PreparedModelSolver<'_>,
        pair: &[ModelSolve],
        y: &[f64],
        residual: &[f64],
        plan: Option<&MatchPlan>,
    ) -> Result<()> {
        if !self.enabled {
            return Ok(());
        }
        let predict = |side: usize, row| {
            let c = &pair[side].coefficients;
            solver
                .operator()
                .prediction_at(&c.worker, &c.firm, &c.control, row)
        };
        let mut offsets = vec![0; problem.outcome.len() + 1];
        for row in 0..problem.outcome.len() {
            offsets[row + 1] = offsets[row] + problem.frequency[row] as usize;
        }
        for class in classes {
            for &row in &class.rows {
                let sw = predict(0, row)?;
                let sf = predict(1, row)?;
                let factor = y[row] * residual[row];
                let l = [factor * sw * sw, factor * sf * sf, factor * sw * sf];
                for copy in offsets[row]..offsets[row + 1] {
                    self.units.get_mut(&(0, copy)).unwrap().l.push(vec![l]);
                }
            }
        }
        if let Some(plan) = plan {
            for (group, rows) in plan.rows.iter().enumerate() {
                let aw = rows
                    .iter()
                    .map(|&row| Ok(problem.frequency[row] as f64 * y[row] * predict(0, row)?))
                    .collect::<Result<Vec<_>>>()?
                    .iter()
                    .sum::<f64>();
                let af = rows
                    .iter()
                    .map(|&row| Ok(problem.frequency[row] as f64 * y[row] * predict(1, row)?))
                    .collect::<Result<Vec<_>>>()?
                    .iter()
                    .sum::<f64>();
                let mut l = Vec::new();
                for &row in rows {
                    let root = (problem.frequency[row] as f64).sqrt();
                    let sw = predict(0, row)?;
                    let sf = predict(1, row)?;
                    l.push([
                        root * aw * sw,
                        root * af * sf,
                        root * (aw * sf + af * sw) * 0.5,
                    ]);
                }
                self.units.get_mut(&(1, group)).unwrap().l.push(l);
            }
        }
        Ok(())
    }

    pub(in crate::generic_jla) fn validate(
        &self,
        r: u32,
        t: u32,
        scores: &[Vec<Primitive>; 2],
        leverage: numerical_mc::Matrix3,
    ) {
        if !self.enabled {
            return;
        }
        let mut input = format!(
            "{{\"r\":{r},\"t\":{t},\"scores\":{:?},\"leverage\":{leverage:?},\"units\":[",
            scores
                .each_ref()
                .map(|s| s.iter().map(|v| v.values()).collect::<Vec<_>>())
        );
        for (i, ((kind, _), unit)) in self.units.iter().enumerate() {
            if i > 0 {
                input.push(',');
            }
            if *kind == 0 {
                input.push_str(&format!(
                    "{{\"kind\":\"observation\",\"u\":{:?},\"l\":{:?},\"control\":{}}}",
                    unit.u, unit.l, unit.cp[0]
                ));
            } else {
                let width = unit.v.len();
                let cp: Vec<_> = unit.cp.chunks(width).collect();
                input.push_str(&format!("{{\"kind\":\"match\",\"u\":{:?},\"l\":{:?},\"control\":{cp:?},\"v\":{:?},\"e\":{:?}}}",unit.u,unit.l,unit.v,unit.e));
            }
        }
        input.push_str("]}");
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../..");
        let mut child = std::process::Command::new(root.join(".venv/bin/python"))
            .arg(root.join("fevc/tools/all_probe_cache_oracle.py"))
            .stdin(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .unwrap();
        use std::io::Write;
        child
            .stdin
            .take()
            .unwrap()
            .write_all(input.as_bytes())
            .unwrap();
        let output = child.wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
