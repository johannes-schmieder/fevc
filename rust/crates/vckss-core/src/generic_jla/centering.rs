// SPDX-License-Identifier: GPL-3.0-only
use super::*;
use crate::centering::jla::{Map, Unit};
#[derive(Clone, Copy, Default)]
pub(super) struct Power {
    p: StableAccumulator,
    m: StableAccumulator,
}
impl Power {
    pub(super) fn add(&mut self, p: f64, m: f64) {
        self.p.add(p * p);
        self.m.add(m * m);
    }
    fn share(self) -> Result<f64> {
        let p = self.p.finish();
        let m = self.m.finish();
        let s = p + m;
        if !s.is_finite() || s <= 0.0 {
            return Err(invalid("centering probe mass is nonpositive"));
        }
        Ok(p / s)
    }
}
pub(super) struct Pools {
    matched: Vec<[Power; 3]>,
    pub(super) observation: Vec<[Power; 3]>,
    units: [Vec<Unit>; 3],
}
impl Pools {
    pub(super) fn new(
        groups: usize,
        physical: usize,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Self> {
        Ok(Self {
            matched: repeated(
                groups,
                [Power::default(); 3],
                "centering match halves",
                interrupt,
                "centering_allocate",
            )?,
            observation: repeated(
                physical,
                [Power::default(); 3],
                "centering copy halves",
                interrupt,
                "centering_allocate",
            )?,
            units: [Vec::new(), Vec::new(), Vec::new()],
        })
    }
    pub(super) fn record_match(
        &mut self,
        plan: &MatchPlan,
        solver: &PreparedModelSolver<'_>,
        solved: &[ModelSolve],
        atoms: &[i64],
        first: usize,
        probes: usize,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        for (column, solution) in solved.iter().enumerate() {
            let pool = 1 + usize::from(first + column >= probes / 2);
            for (g, rows) in plan.rows.iter().enumerate() {
                checkpoint_chunk(interrupt, g, "centering_match_moments")?;
                let c = &solution.coefficients;
                let p = (plan.physical_count[g] as f64).sqrt()
                    * solver
                        .operator()
                        .prediction_at(&c.worker, &c.firm, &c.control, rows[0])?;
                let m = atoms[column * plan.rows.len() + g] as f64
                    / (plan.physical_count[g] as f64).sqrt()
                    - p;
                self.matched[g][0].add(p, m);
                self.matched[g][pool].add(p, m);
            }
        }
        Ok(())
    }
    pub(super) fn build_match(
        &mut self,
        problem: &CompressedProblem,
        plan: &MatchPlan,
        z: &[f64],
        residual: &[f64],
        geometry: Option<&ControlGeometry>,
        options: GenericJlaOptions,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        let q = geometry.map_or(0, |g| g.controls);
        let rank = q + 1;
        for (g, rows) in plan.rows.iter().enumerate() {
            let width = rows.len();
            let weights: Vec<_> = rows.iter().map(|&r| problem.frequency[r] as f64).collect();
            let root: Vec<_> = weights.iter().map(|f| f.sqrt()).collect();
            let mass = weights.iter().sum::<f64>();
            let zz: Vec<_> = rows.iter().map(|&r| z[r]).collect();
            for pool in 0..3 {
                let p = self.matched[g][pool].share()?;
                let mut low = vec![0.0; width * rank];
                let mut rhs = vec![0.0; width];
                for (i, &row) in rows.iter().enumerate() {
                    low[i * rank] = p.sqrt() * root[i] / mass.sqrt();
                    rhs[i] = root[i] * residual[row];
                    if let Some(geometry) = geometry {
                        for output in 0..q {
                            for input in 0..q {
                                low[i * rank + 1 + output] += root[i]
                                    * geometry.residualized[input * problem.outcome.len() + row]
                                    * geometry.factor[input * q + output];
                            }
                        }
                    }
                }
                let action = maker_actions(&low, width, rank, &rhs, 1, options, interrupt)?;
                let r: Vec<_> = action
                    .values
                    .iter()
                    .zip(&root)
                    .map(|(v, s)| v / s)
                    .collect();
                let mut direction = vec![1.0; width];
                for k in 0..rank {
                    let d = (0..width).map(|i| low[i * rank + k] * root[i]).sum::<f64>();
                    for i in 0..width {
                        direction[i] -= low[i * rank + k] * d / root[i];
                    }
                }
                self.units[pool].push(Unit::new(
                    rows.clone(),
                    weights.clone(),
                    &zz,
                    &r,
                    &direction,
                    problem.physical_total as f64,
                )?);
            }
        }
        Ok(())
    }
    pub(super) fn build_obs(
        &mut self,
        problem: &CompressedProblem,
        classes: &[ObservationClass],
        z: &[f64],
        residual: &[f64],
        control: &[f64],
        options: GenericJlaOptions,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<()> {
        let mut offset = vec![0; problem.outcome.len() + 1];
        for row in 0..problem.outcome.len() {
            offset[row + 1] = offset[row] + problem.frequency[row] as usize;
        }
        for class in classes {
            for &row in &class.rows {
                for copy in 0..problem.frequency[row] as usize {
                    checkpoint_chunk(interrupt, copy, "centering_observation_units")?;
                    for pool in 0..3 {
                        let maker = 1.0
                            - self.observation[offset[row] + copy][pool].share()?
                            - control[row];
                        if maker <= options.block_tolerance {
                            return Err(invalid(
                                "centering half-pool observation maker is nonpositive",
                            ));
                        }
                        self.units[pool].push(Unit::new(
                            vec![row],
                            vec![1.0],
                            &[z[row]],
                            &[residual[row] / maker],
                            &[maker],
                            problem.physical_total as f64,
                        )?);
                    }
                }
            }
        }
        Ok(())
    }
    pub(super) fn finish(
        self,
        data: CanonicalModelData<'_>,
        solver: &PreparedModelSolver<'_>,
        routing: ModelRoutingOptions,
        interrupt: &mut dyn InterruptCheck,
    ) -> Result<Map> {
        let [full, first, second] = self.units;
        Map::jackknife(
            Map::solve(data, solver, full, routing, interrupt)?,
            Map::solve(data, solver, first, routing, interrupt)?,
            Map::solve(data, solver, second, routing, interrupt)?,
        )
    }
}
